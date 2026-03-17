use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use secrecy::SecretString;
use uuid::Uuid;

use crate::errors::{VaultError, VaultResult};
use crate::model::{
    FolderId, FolderTreeNode, SecretBytes, SecretId, SecretMetadata,
    SecretSearchItem, SecretTreeNode, SecretValueUpdate, VaultMetadataNode,
    VaultMetadataTree, names_conflict, sort_metadata_nodes, validate_name,
};
use crate::settings::VaultSettings;
use crate::{crypto, migrations};

const ROOT_FOLDER_NAME: &str = "root";
const BOOTSTRAP_HEADER: &str = "OTTY_VAULT_BOOTSTRAP_V1";

#[derive(Clone)]
struct FolderRecord {
    id: FolderId,
    parent_id: Option<FolderId>,
    name: String,
}

#[derive(Clone)]
struct SecretRecord {
    id: SecretId,
    metadata: SecretMetadata,
    encrypted_value: SecretBytes,
}

#[derive(Clone)]
struct PersistedVault {
    passphrase_fingerprint: u64,
    format_version: u32,
    settings: VaultSettings,
    root_id: FolderId,
    folders: HashMap<FolderId, FolderRecord>,
    secrets: HashMap<SecretId, SecretRecord>,
}

impl PersistedVault {
    fn bootstrap(passphrase_fingerprint: u64, format_version: u32) -> Self {
        let root_id = FolderId::new();

        Self {
            passphrase_fingerprint,
            format_version,
            settings: VaultSettings::default(),
            root_id,
            folders: HashMap::from([
                (
                    root_id,
                    FolderRecord {
                        id: root_id,
                        parent_id: None,
                        name: ROOT_FOLDER_NAME.to_string(),
                    }
                )
            ]),
            secrets: HashMap::new(),
        }
    }
}

static VAULT_REGISTRY: OnceLock<Mutex<HashMap<PathBuf, PersistedVault>>> =
    OnceLock::new();

fn registry() -> &'static Mutex<HashMap<PathBuf, PersistedVault>> {
    VAULT_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_registry()
-> VaultResult<MutexGuard<'static, HashMap<PathBuf, PersistedVault>>> {
    registry().lock().map_err(|_| VaultError::StorageError)
}

fn get_vault<'a>(
    registry: &'a HashMap<PathBuf, PersistedVault>,
    path: &Path,
) -> VaultResult<&'a PersistedVault> {
    registry.get(path).ok_or(VaultError::VaultCorrupted)
}

fn get_vault_mut<'a>(
    registry: &'a mut HashMap<PathBuf, PersistedVault>,
    path: &Path,
) -> VaultResult<&'a mut PersistedVault> {
    registry.get_mut(path).ok_or(VaultError::VaultCorrupted)
}

pub(crate) fn open_or_bootstrap(
    path: &Path,
    passphrase: &SecretString,
) -> VaultResult<()> {
    ensure_storage_file(path)?;
    let fingerprint = crypto::fingerprint_passphrase(passphrase);
    let mut registry = lock_registry()?;
    match registry.entry(path.to_path_buf()) {
        Entry::Occupied(mut occupied) => {
            if occupied.get().passphrase_fingerprint != fingerprint {
                return Err(VaultError::WrongPassphrase);
            }
            let next_version = migrations::run_migrations(Some(
                occupied.get().format_version,
            ))?;
            occupied.get_mut().format_version = next_version;
            persist_bootstrap(path, occupied.get())?;
            Ok(())
        },
        Entry::Vacant(vacant) => {
            let persisted = match load_bootstrap(path)? {
                Some(mut vault) => {
                    if vault.passphrase_fingerprint != fingerprint {
                        return Err(VaultError::WrongPassphrase);
                    }
                    vault.format_version =
                        migrations::run_migrations(Some(vault.format_version))?;
                    vault
                },
                None => {
                    let format_version = migrations::run_migrations(None)?;
                    PersistedVault::bootstrap(fingerprint, format_version)
                },
            };
            persist_bootstrap(path, &persisted)?;
            vacant.insert(persisted);
            Ok(())
        },
    }
}

pub(crate) fn verify_passphrase(
    path: &Path,
    passphrase: &SecretString,
) -> VaultResult<()> {
    let registry = lock_registry()?;
    let vault = get_vault(&registry, path)?;
    if vault.passphrase_fingerprint
        == crypto::fingerprint_passphrase(passphrase)
    {
        return Ok(());
    }
    Err(VaultError::WrongPassphrase)
}

pub(crate) fn change_passphrase(
    path: &Path,
    passphrase: &SecretString,
    new_passphrase: &SecretString,
) -> VaultResult<()> {
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    if vault.passphrase_fingerprint
        != crypto::fingerprint_passphrase(passphrase)
    {
        return Err(VaultError::WrongPassphrase);
    }
    vault.passphrase_fingerprint =
        crypto::fingerprint_passphrase(new_passphrase);
    persist_bootstrap(path, vault)?;
    Ok(())
}

pub(crate) fn settings(path: &Path) -> VaultResult<VaultSettings> {
    let registry = lock_registry()?;
    let vault = get_vault(&registry, path)?;
    Ok(vault.settings)
}

pub(crate) fn update_settings(
    path: &Path,
    settings: VaultSettings,
) -> VaultResult<()> {
    settings.validate()?;
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    vault.settings = settings;
    persist_bootstrap(path, vault)?;
    Ok(())
}

pub(crate) fn metadata_tree(path: &Path) -> VaultResult<VaultMetadataTree> {
    let registry = lock_registry()?;
    let vault = get_vault(&registry, path)?;
    let root = build_folder_tree(vault, vault.root_id)?;
    Ok(VaultMetadataTree::new(root))
}

pub(crate) fn create_folder(
    path: &Path,
    parent_id: Option<&FolderId>,
    name: &str,
) -> VaultResult<FolderId> {
    validate_name(name)?;
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    let target_parent = parent_id.copied().unwrap_or(vault.root_id);
    if !vault.folders.contains_key(&target_parent) {
        return Err(VaultError::FolderNotFound);
    }
    ensure_name_available(vault, target_parent, name, None, None)?;

    let folder_id = FolderId::new();
    vault.folders.insert(
        folder_id,
        FolderRecord {
            id: folder_id,
            parent_id: Some(target_parent),
            name: name.to_string(),
        },
    );
    Ok(folder_id)
}

pub(crate) fn delete_folder(path: &Path, id: &FolderId) -> VaultResult<()> {
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    if *id == vault.root_id {
        return Err(VaultError::StorageError);
    }
    if !vault.folders.contains_key(id) {
        return Err(VaultError::FolderNotFound);
    }

    let mut pending = vec![*id];
    let mut to_remove = Vec::new();
    while let Some(folder_id) = pending.pop() {
        to_remove.push(folder_id);
        let children: Vec<FolderId> = vault
            .folders
            .values()
            .filter(|folder| folder.parent_id == Some(folder_id))
            .map(|folder| folder.id)
            .collect();
        pending.extend(children);
    }

    let removed_set: HashSet<FolderId> = to_remove.iter().copied().collect();
    vault.secrets.retain(|_, secret| {
        !removed_set.contains(&secret.metadata.folder_id())
    });
    for folder_id in to_remove {
        vault.folders.remove(&folder_id);
    }
    Ok(())
}

pub(crate) fn rename_folder(
    path: &Path,
    id: &FolderId,
    name: &str,
) -> VaultResult<()> {
    validate_name(name)?;
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    if *id == vault.root_id {
        return Err(VaultError::StorageError);
    }
    let parent_id = vault
        .folders
        .get(id)
        .ok_or(VaultError::FolderNotFound)?
        .parent_id
        .ok_or(VaultError::VaultCorrupted)?;
    ensure_name_available(vault, parent_id, name, Some(*id), None)?;
    let folder = vault
        .folders
        .get_mut(id)
        .ok_or(VaultError::FolderNotFound)?;
    folder.name = name.to_string();
    Ok(())
}

pub(crate) fn move_folder(
    path: &Path,
    id: &FolderId,
    parent_id: Option<&FolderId>,
) -> VaultResult<()> {
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    if *id == vault.root_id {
        return Err(VaultError::StorageError);
    }

    let folder_name = vault
        .folders
        .get(id)
        .ok_or(VaultError::FolderNotFound)?
        .name
        .clone();
    let target_parent = parent_id.copied().unwrap_or(vault.root_id);
    if !vault.folders.contains_key(&target_parent) {
        return Err(VaultError::FolderNotFound);
    }
    if target_parent == *id || is_descendant(vault, target_parent, *id) {
        return Err(VaultError::StorageError);
    }
    ensure_name_available(vault, target_parent, &folder_name, Some(*id), None)?;
    let folder = vault
        .folders
        .get_mut(id)
        .ok_or(VaultError::FolderNotFound)?;
    folder.parent_id = Some(target_parent);
    Ok(())
}

pub(crate) fn create_secret(
    path: &Path,
    metadata: SecretMetadata,
    value: SecretBytes,
) -> VaultResult<SecretId> {
    metadata.validate()?;
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    let folder_id = metadata.folder_id();
    if !vault.folders.contains_key(&folder_id) {
        return Err(VaultError::FolderNotFound);
    }
    ensure_name_available(vault, folder_id, metadata.name(), None, None)?;

    let secret_id = SecretId::new();
    let encrypted_value = crypto::encrypt_secret(&value)?;
    vault.secrets.insert(
        secret_id,
        SecretRecord {
            id: secret_id,
            metadata,
            encrypted_value,
        },
    );
    Ok(secret_id)
}

pub(crate) fn update_secret(
    path: &Path,
    id: &SecretId,
    metadata: SecretMetadata,
    value: SecretValueUpdate,
) -> VaultResult<()> {
    metadata.validate()?;
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    if !vault.secrets.contains_key(id) {
        return Err(VaultError::SecretNotFound);
    }

    let folder_id = metadata.folder_id();
    if !vault.folders.contains_key(&folder_id) {
        return Err(VaultError::FolderNotFound);
    }
    ensure_name_available(vault, folder_id, metadata.name(), None, Some(*id))?;

    let secret = vault
        .secrets
        .get_mut(id)
        .ok_or(VaultError::SecretNotFound)?;
    secret.metadata = metadata;
    if let SecretValueUpdate::Set(next_value) = value {
        secret.encrypted_value = crypto::encrypt_secret(&next_value)?;
    }
    Ok(())
}

pub(crate) fn get_secret_metadata(
    path: &Path,
    id: &SecretId,
) -> VaultResult<SecretMetadata> {
    let registry = lock_registry()?;
    let vault = get_vault(&registry, path)?;
    let secret = vault.secrets.get(id).ok_or(VaultError::SecretNotFound)?;
    Ok(secret.metadata.clone())
}

pub(crate) fn get_secret_value(
    path: &Path,
    id: &SecretId,
) -> VaultResult<SecretBytes> {
    let registry = lock_registry()?;
    let vault = get_vault(&registry, path)?;
    let secret = vault.secrets.get(id).ok_or(VaultError::SecretNotFound)?;
    crypto::decrypt_secret(&secret.encrypted_value)
}

pub(crate) fn find_secrets_by_name(
    path: &Path,
    query: &str,
) -> VaultResult<Vec<SecretSearchItem>> {
    let registry = lock_registry()?;
    let vault = get_vault(&registry, path)?;
    let mut results: Vec<SecretSearchItem> = vault
        .secrets
        .values()
        .filter(|secret| {
            query.is_empty() || secret.metadata.name().contains(query)
        })
        .map(|secret| {
            let computed_path = build_secret_path(vault, secret)?;
            Ok(SecretSearchItem::new(
                secret.id,
                secret.metadata.clone(),
                computed_path,
            ))
        })
        .collect::<VaultResult<Vec<_>>>()?;
    results.sort_by(|left, right| left.path().cmp(right.path()));
    Ok(results)
}

pub(crate) fn delete_secret(path: &Path, id: &SecretId) -> VaultResult<()> {
    let mut registry = lock_registry()?;
    let vault = get_vault_mut(&mut registry, path)?;
    if vault.secrets.remove(id).is_none() {
        return Err(VaultError::SecretNotFound);
    }
    Ok(())
}

fn ensure_storage_file(path: &Path) -> VaultResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|_| VaultError::StorageError)?;
        }
    }

    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map(|_| ())
        .map_err(|_| VaultError::StorageError)
}

fn load_bootstrap(path: &Path) -> VaultResult<Option<PersistedVault>> {
    let raw = fs::read_to_string(path).map_err(|_| VaultError::StorageError)?;
    if raw.trim().is_empty() {
        return Ok(None);
    }

    let mut lines = raw.lines();
    if lines.next() != Some(BOOTSTRAP_HEADER) {
        return Err(VaultError::VaultCorrupted);
    }

    let fields = lines.try_fold(HashMap::new(), |mut fields, line| {
        let (key, value) =
            line.split_once('=').ok_or(VaultError::VaultCorrupted)?;
        fields.insert(key, value);
        Ok::<_, VaultError>(fields)
    })?;

    let passphrase_fingerprint =
        parse_u64_field(&fields, "passphrase_fingerprint")?;
    let format_version = parse_u32_field(&fields, "format_version")?;
    let auto_lock_timeout_secs =
        parse_u64_field(&fields, "auto_lock_timeout_secs")?;
    let clipboard_clear_timeout_secs =
        parse_u64_field(&fields, "clipboard_clear_timeout_secs")?;
    let root_id = fields
        .get("root_id")
        .ok_or(VaultError::VaultCorrupted)
        .and_then(|value| {
            Uuid::parse_str(value)
                .map(FolderId::from_uuid)
                .map_err(|_| VaultError::VaultCorrupted)
        })?;

    let mut vault =
        PersistedVault::bootstrap(passphrase_fingerprint, format_version);
    vault.settings = VaultSettings::new(
        Duration::from_secs(auto_lock_timeout_secs),
        Duration::from_secs(clipboard_clear_timeout_secs),
    )?;
    vault.root_id = root_id;
    vault.folders.clear();
    vault.folders.insert(
        root_id,
        FolderRecord {
            id: root_id,
            parent_id: None,
            name: ROOT_FOLDER_NAME.to_string(),
        },
    );

    Ok(Some(vault))
}

fn persist_bootstrap(path: &Path, vault: &PersistedVault) -> VaultResult<()> {
    let payload = format!(
        "{BOOTSTRAP_HEADER}\npassphrase_fingerprint={}\nformat_version={}\nauto_lock_timeout_secs={}\nclipboard_clear_timeout_secs={}\nroot_id={}\n",
        vault.passphrase_fingerprint,
        vault.format_version,
        vault.settings.auto_lock_timeout().as_secs(),
        vault.settings.clipboard_clear_timeout().as_secs(),
        vault.root_id.as_uuid(),
    );
    fs::write(path, payload).map_err(|_| VaultError::StorageError)
}

fn parse_u64_field(
    fields: &HashMap<&str, &str>,
    key: &str,
) -> VaultResult<u64> {
    fields
        .get(key)
        .ok_or(VaultError::VaultCorrupted)?
        .parse::<u64>()
        .map_err(|_| VaultError::VaultCorrupted)
}

fn parse_u32_field(
    fields: &HashMap<&str, &str>,
    key: &str,
) -> VaultResult<u32> {
    fields
        .get(key)
        .ok_or(VaultError::VaultCorrupted)?
        .parse::<u32>()
        .map_err(|_| VaultError::VaultCorrupted)
}

fn ensure_name_available(
    vault: &PersistedVault,
    parent_id: FolderId,
    candidate_name: &str,
    skip_folder_id: Option<FolderId>,
    skip_secret_id: Option<SecretId>,
) -> VaultResult<()> {
    for folder in vault.folders.values() {
        if folder.parent_id == Some(parent_id)
            && Some(folder.id) != skip_folder_id
            && names_conflict(&folder.name, candidate_name)
        {
            return Err(VaultError::DuplicateNameWithinFolder);
        }
    }
    for secret in vault.secrets.values() {
        if secret.metadata.folder_id() == parent_id
            && Some(secret.id) != skip_secret_id
            && names_conflict(secret.metadata.name(), candidate_name)
        {
            return Err(VaultError::DuplicateNameWithinFolder);
        }
    }
    Ok(())
}

fn is_descendant(
    vault: &PersistedVault,
    candidate_parent: FolderId,
    folder_id: FolderId,
) -> bool {
    let mut current = Some(candidate_parent);
    while let Some(parent) = current {
        if parent == folder_id {
            return true;
        }
        current = vault
            .folders
            .get(&parent)
            .and_then(|folder| folder.parent_id);
    }
    false
}

fn build_folder_tree(
    vault: &PersistedVault,
    folder_id: FolderId,
) -> VaultResult<FolderTreeNode> {
    let folder = vault
        .folders
        .get(&folder_id)
        .ok_or(VaultError::VaultCorrupted)?;

    let mut children = Vec::new();
    let mut child_folders: Vec<&FolderRecord> = vault
        .folders
        .values()
        .filter(|item| item.parent_id == Some(folder_id))
        .collect();
    child_folders.sort_by(|left, right| left.name.cmp(&right.name));
    for child in child_folders {
        children.push(VaultMetadataNode::Folder(build_folder_tree(
            vault, child.id,
        )?));
    }

    let mut child_secrets: Vec<&SecretRecord> = vault
        .secrets
        .values()
        .filter(|item| item.metadata.folder_id() == folder_id)
        .collect();
    child_secrets
        .sort_by(|left, right| left.metadata.name().cmp(right.metadata.name()));
    for secret in child_secrets {
        children.push(VaultMetadataNode::Secret(SecretTreeNode::new(
            secret.id,
            secret.metadata.clone(),
        )));
    }

    sort_metadata_nodes(&mut children);
    FolderTreeNode::new(folder.id, folder.name.clone(), children)
}

fn build_secret_path(
    vault: &PersistedVault,
    secret: &SecretRecord,
) -> VaultResult<String> {
    let mut path_segments =
        folder_path_segments(vault, secret.metadata.folder_id())?;
    path_segments.push(secret.metadata.name().to_string());
    Ok(path_segments.join("/"))
}

fn folder_path_segments(
    vault: &PersistedVault,
    folder_id: FolderId,
) -> VaultResult<Vec<String>> {
    let mut reversed = Vec::new();
    let mut current = Some(folder_id);
    while let Some(next_id) = current {
        let folder = vault
            .folders
            .get(&next_id)
            .ok_or(VaultError::VaultCorrupted)?;
        reversed.push(folder.name.clone());
        current = folder.parent_id;
    }
    reversed.reverse();
    Ok(reversed)
}

#[cfg(test)]
pub(crate) fn clear_registry_for_tests() -> VaultResult<()> {
    lock_registry()?.clear();
    Ok(())
}
