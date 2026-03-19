use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use secrecy::SecretString;

use crate::crypto::{self, PassphraseVerifier};
use crate::errors::{VaultError, VaultResult};
use crate::migrations;
use crate::model::{
    Secret, SecretBytes, SecretGroup, SecretGroupId, SecretId, SecretMetadata,
    SecretSearchItem, SecretValueUpdate, names_conflict, sort_secret_groups,
    validate_name,
};
use crate::settings::Settings;

const BOOTSTRAP_HEADER: &str = "OTTY_VAULT_BOOTSTRAP_V1";

/// Internal store interface hidden behind the public `Vault` contract.
pub(crate) trait VaultStore {
    fn open_or_bootstrap(
        &self,
        path: &Path,
        passphrase: &SecretString,
    ) -> VaultResult<()>;
    fn verify_passphrase(
        &self,
        path: &Path,
        passphrase: &SecretString,
    ) -> VaultResult<()>;
    fn change_passphrase(
        &self,
        path: &Path,
        passphrase: &SecretString,
        new_passphrase: &SecretString,
    ) -> VaultResult<()>;
    fn settings(&self, path: &Path) -> VaultResult<Settings>;
    fn update_settings(
        &self,
        path: &Path,
        settings: Settings,
    ) -> VaultResult<()>;
    fn secret_groups(&self, path: &Path) -> VaultResult<Vec<SecretGroup>>;
    fn create_secret_group(
        &self,
        path: &Path,
        name: &str,
    ) -> VaultResult<SecretGroupId>;
    fn delete_secret_group(
        &self,
        path: &Path,
        id: &SecretGroupId,
    ) -> VaultResult<()>;
    fn rename_secret_group(
        &self,
        path: &Path,
        id: &SecretGroupId,
        name: &str,
    ) -> VaultResult<()>;
    fn create_secret(
        &self,
        path: &Path,
        metadata: SecretMetadata,
        value: SecretBytes,
    ) -> VaultResult<SecretId>;
    fn update_secret(
        &self,
        path: &Path,
        id: &SecretId,
        metadata: SecretMetadata,
        value: SecretValueUpdate,
    ) -> VaultResult<()>;
    fn get_secret_metadata(
        &self,
        path: &Path,
        id: &SecretId,
    ) -> VaultResult<SecretMetadata>;
    fn get_secret_value(
        &self,
        path: &Path,
        id: &SecretId,
    ) -> VaultResult<SecretBytes>;
    fn find_secrets_by_name(
        &self,
        path: &Path,
        query: &str,
    ) -> VaultResult<Vec<SecretSearchItem>>;
    fn delete_secret(&self, path: &Path, id: &SecretId) -> VaultResult<()>;
}

/// Foundation-stage store that persists only bootstrap metadata and keeps the
/// mutable logical snapshot process-local until SQLCipher-backed storage lands.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ScaffoldStore;

#[derive(Debug, Clone)]
struct BootstrapRecord {
    passphrase_verifier: PassphraseVerifier,
    format_version: u32,
    settings: Settings,
}

impl BootstrapRecord {
    fn new(
        passphrase_verifier: PassphraseVerifier,
        format_version: u32,
    ) -> Self {
        Self {
            passphrase_verifier,
            format_version,
            settings: Settings::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct SecretGroupRecord {
    id: SecretGroupId,
    name: String,
}

#[derive(Debug, Clone)]
struct SecretRecord {
    id: SecretId,
    metadata: SecretMetadata,
    encrypted_value: SecretBytes,
}

#[derive(Debug, Clone)]
struct VaultRecord {
    bootstrap: BootstrapRecord,
    secret_groups: BTreeMap<SecretGroupId, SecretGroupRecord>,
    secrets: BTreeMap<SecretId, SecretRecord>,
}

impl VaultRecord {
    fn from_bootstrap(bootstrap: BootstrapRecord) -> Self {
        Self {
            bootstrap,
            secret_groups: BTreeMap::new(),
            secrets: BTreeMap::new(),
        }
    }
}

static REGISTRY: OnceLock<Mutex<BTreeMap<PathBuf, VaultRecord>>> =
    OnceLock::new();

fn registry() -> &'static Mutex<BTreeMap<PathBuf, VaultRecord>> {
    REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn lock_registry()
-> VaultResult<MutexGuard<'static, BTreeMap<PathBuf, VaultRecord>>> {
    registry().lock().map_err(|_| VaultError::StorageError)
}

fn get_vault<'a>(
    registry: &'a BTreeMap<PathBuf, VaultRecord>,
    path: &Path,
) -> VaultResult<&'a VaultRecord> {
    registry.get(path).ok_or(VaultError::VaultCorrupted)
}

fn get_vault_mut<'a>(
    registry: &'a mut BTreeMap<PathBuf, VaultRecord>,
    path: &Path,
) -> VaultResult<&'a mut VaultRecord> {
    registry.get_mut(path).ok_or(VaultError::VaultCorrupted)
}

impl VaultStore for ScaffoldStore {
    fn open_or_bootstrap(
        &self,
        path: &Path,
        passphrase: &SecretString,
    ) -> VaultResult<()> {
        ensure_storage_file(path)?;
        let expected_verifier = crypto::derive_passphrase_verifier(passphrase);
        let loaded_bootstrap = load_bootstrap(path)?;
        let format_version = migrations::run_migrations(
            loaded_bootstrap
                .as_ref()
                .map(|record| record.format_version),
        )?;

        let bootstrap = match loaded_bootstrap {
            Some(mut existing) => {
                if existing.passphrase_verifier != expected_verifier {
                    return Err(VaultError::WrongPassphrase);
                }
                existing.format_version = format_version;
                existing
            },
            None => BootstrapRecord::new(expected_verifier, format_version),
        };
        bootstrap.settings.validate()?;
        persist_bootstrap(path, &bootstrap)?;

        let mut registry = lock_registry()?;
        match registry.entry(path.to_path_buf()) {
            Entry::Occupied(mut occupied) => {
                if occupied.get().bootstrap.passphrase_verifier
                    != expected_verifier
                {
                    return Err(VaultError::WrongPassphrase);
                }
                occupied.get_mut().bootstrap = bootstrap;
            },
            Entry::Vacant(vacant) => {
                vacant.insert(VaultRecord::from_bootstrap(bootstrap));
            },
        }

        Ok(())
    }

    fn verify_passphrase(
        &self,
        path: &Path,
        passphrase: &SecretString,
    ) -> VaultResult<()> {
        let verifier = crypto::derive_passphrase_verifier(passphrase);

        let registry = lock_registry()?;
        let vault = get_vault(&registry, path)?;
        if vault.bootstrap.passphrase_verifier == verifier {
            return Ok(());
        }

        Err(VaultError::WrongPassphrase)
    }

    fn change_passphrase(
        &self,
        path: &Path,
        passphrase: &SecretString,
        new_passphrase: &SecretString,
    ) -> VaultResult<()> {
        let current = crypto::derive_passphrase_verifier(passphrase);
        let next = crypto::derive_passphrase_verifier(new_passphrase);

        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        if vault.bootstrap.passphrase_verifier != current {
            return Err(VaultError::WrongPassphrase);
        }

        vault.bootstrap.passphrase_verifier = next;
        persist_bootstrap(path, &vault.bootstrap)
    }

    fn settings(&self, path: &Path) -> VaultResult<Settings> {
        let registry = lock_registry()?;
        Ok(get_vault(&registry, path)?.bootstrap.settings)
    }

    fn update_settings(
        &self,
        path: &Path,
        settings: Settings,
    ) -> VaultResult<()> {
        settings.validate()?;

        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        vault.bootstrap.settings = settings;
        persist_bootstrap(path, &vault.bootstrap)
    }

    fn secret_groups(&self, path: &Path) -> VaultResult<Vec<SecretGroup>> {
        let registry = lock_registry()?;
        let vault = get_vault(&registry, path)?;
        build_secret_groups_snapshot(vault)
    }

    fn create_secret_group(
        &self,
        path: &Path,
        name: &str,
    ) -> VaultResult<SecretGroupId> {
        validate_name(name)?;

        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        ensure_secret_group_name_available(vault, name, None)?;

        let id = SecretGroupId::new();
        vault.secret_groups.insert(
            id,
            SecretGroupRecord {
                id,
                name: String::from(name),
            },
        );

        Ok(id)
    }

    fn delete_secret_group(
        &self,
        path: &Path,
        id: &SecretGroupId,
    ) -> VaultResult<()> {
        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        if vault.secret_groups.remove(id).is_none() {
            return Err(VaultError::SecretGroupNotFound);
        }

        vault
            .secrets
            .retain(|_, secret| secret.metadata.secret_group_id() != *id);
        Ok(())
    }

    fn rename_secret_group(
        &self,
        path: &Path,
        id: &SecretGroupId,
        name: &str,
    ) -> VaultResult<()> {
        validate_name(name)?;

        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        ensure_secret_group_exists(vault, id)?;
        ensure_secret_group_name_available(vault, name, Some(*id))?;

        let secret_group = vault
            .secret_groups
            .get_mut(id)
            .ok_or(VaultError::SecretGroupNotFound)?;
        secret_group.name = String::from(name);
        Ok(())
    }

    fn create_secret(
        &self,
        path: &Path,
        metadata: SecretMetadata,
        value: SecretBytes,
    ) -> VaultResult<SecretId> {
        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        ensure_secret_group_exists(vault, &metadata.secret_group_id())?;
        ensure_secret_name_available(vault, &metadata, None)?;

        let id = SecretId::new();
        let encrypted_value = crypto::encrypt_secret(&value)?;
        vault.secrets.insert(
            id,
            SecretRecord {
                id,
                metadata,
                encrypted_value,
            },
        );

        Ok(id)
    }

    fn update_secret(
        &self,
        path: &Path,
        id: &SecretId,
        metadata: SecretMetadata,
        value: SecretValueUpdate,
    ) -> VaultResult<()> {
        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        ensure_secret_group_exists(vault, &metadata.secret_group_id())?;
        ensure_secret_name_available(vault, &metadata, Some(*id))?;

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

    fn get_secret_metadata(
        &self,
        path: &Path,
        id: &SecretId,
    ) -> VaultResult<SecretMetadata> {
        let registry = lock_registry()?;
        let vault = get_vault(&registry, path)?;
        let secret = vault.secrets.get(id).ok_or(VaultError::SecretNotFound)?;
        Ok(secret.metadata.clone())
    }

    fn get_secret_value(
        &self,
        path: &Path,
        id: &SecretId,
    ) -> VaultResult<SecretBytes> {
        let registry = lock_registry()?;
        let vault = get_vault(&registry, path)?;
        let secret = vault.secrets.get(id).ok_or(VaultError::SecretNotFound)?;
        crypto::decrypt_secret(&secret.encrypted_value)
    }

    fn find_secrets_by_name(
        &self,
        path: &Path,
        query: &str,
    ) -> VaultResult<Vec<SecretSearchItem>> {
        let registry = lock_registry()?;
        let vault = get_vault(&registry, path)?;

        let mut items = Vec::new();
        for secret in vault.secrets.values() {
            if !query.is_empty() && !secret.metadata.name().contains(query) {
                continue;
            }

            let secret_group_name = vault
                .secret_groups
                .get(&secret.metadata.secret_group_id())
                .ok_or(VaultError::VaultCorrupted)?
                .name
                .clone();
            let item = SecretSearchItem::new(
                Secret::new(secret.id, secret.metadata.clone()),
                secret_group_name,
            )?;
            items.push(item);
        }

        items.sort_by(|left, right| {
            left.secret_group_name()
                .cmp(right.secret_group_name())
                .then_with(|| {
                    left.secret()
                        .metadata()
                        .name()
                        .cmp(right.secret().metadata().name())
                })
                .then_with(|| left.secret().id().cmp(&right.secret().id()))
        });

        Ok(items)
    }

    fn delete_secret(&self, path: &Path, id: &SecretId) -> VaultResult<()> {
        let mut registry = lock_registry()?;
        let vault = get_vault_mut(&mut registry, path)?;
        if vault.secrets.remove(id).is_none() {
            return Err(VaultError::SecretNotFound);
        }

        Ok(())
    }
}

fn ensure_storage_file(path: &Path) -> VaultResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| VaultError::StorageError)?;
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

fn load_bootstrap(path: &Path) -> VaultResult<Option<BootstrapRecord>> {
    let raw = fs::read_to_string(path).map_err(|_| VaultError::StorageError)?;
    if raw.trim().is_empty() {
        return Ok(None);
    }

    let mut lines = raw.lines();
    if lines.next() != Some(BOOTSTRAP_HEADER) {
        return Err(VaultError::VaultCorrupted);
    }

    let mut fields = BTreeMap::new();
    for line in lines {
        let (key, value) =
            line.split_once('=').ok_or(VaultError::VaultCorrupted)?;
        fields.insert(key, value);
    }

    let passphrase_verifier = PassphraseVerifier::from_u64(parse_u64_field(
        &fields,
        "passphrase_verifier",
    )?);
    let format_version = parse_u32_field(&fields, "format_version")?;
    let auto_lock_timeout_secs =
        parse_u64_field(&fields, "auto_lock_timeout_secs")?;
    let clipboard_clear_timeout_secs =
        parse_u64_field(&fields, "clipboard_clear_timeout_secs")?;
    let settings = Settings::new(
        std::time::Duration::from_secs(auto_lock_timeout_secs),
        std::time::Duration::from_secs(clipboard_clear_timeout_secs),
    )?;

    Ok(Some(BootstrapRecord {
        passphrase_verifier,
        format_version,
        settings,
    }))
}

fn persist_bootstrap(
    path: &Path,
    bootstrap: &BootstrapRecord,
) -> VaultResult<()> {
    let payload = format!(
        "{BOOTSTRAP_HEADER}\npassphrase_verifier={}\nformat_version={}\nauto_lock_timeout_secs={}\nclipboard_clear_timeout_secs={}\n",
        bootstrap.passphrase_verifier.into_u64(),
        bootstrap.format_version,
        bootstrap.settings.auto_lock_timeout().as_secs(),
        bootstrap.settings.clipboard_clear_timeout().as_secs(),
    );

    fs::write(path, payload).map_err(|_| VaultError::StorageError)
}

fn parse_u64_field(
    fields: &BTreeMap<&str, &str>,
    key: &str,
) -> VaultResult<u64> {
    fields
        .get(key)
        .ok_or(VaultError::VaultCorrupted)?
        .parse::<u64>()
        .map_err(|_| VaultError::VaultCorrupted)
}

fn parse_u32_field(
    fields: &BTreeMap<&str, &str>,
    key: &str,
) -> VaultResult<u32> {
    fields
        .get(key)
        .ok_or(VaultError::VaultCorrupted)?
        .parse::<u32>()
        .map_err(|_| VaultError::VaultCorrupted)
}

fn build_secret_groups_snapshot(
    vault: &VaultRecord,
) -> VaultResult<Vec<SecretGroup>> {
    let mut secret_groups = Vec::with_capacity(vault.secret_groups.len());
    for secret_group in vault.secret_groups.values() {
        let mut secrets = Vec::new();
        for secret in vault.secrets.values() {
            if secret.metadata.secret_group_id() == secret_group.id {
                secrets.push(Secret::new(secret.id, secret.metadata.clone()));
            }
        }

        secret_groups.push(SecretGroup::new(
            secret_group.id,
            secret_group.name.clone(),
            secrets,
        )?);
    }

    sort_secret_groups(&mut secret_groups);
    Ok(secret_groups)
}

fn ensure_secret_group_exists(
    vault: &VaultRecord,
    id: &SecretGroupId,
) -> VaultResult<()> {
    if vault.secret_groups.contains_key(id) {
        return Ok(());
    }

    Err(VaultError::SecretGroupNotFound)
}

fn ensure_secret_group_name_available(
    vault: &VaultRecord,
    candidate_name: &str,
    skip_group_id: Option<SecretGroupId>,
) -> VaultResult<()> {
    if vault.secret_groups.values().any(|secret_group| {
        Some(secret_group.id) != skip_group_id
            && names_conflict(&secret_group.name, candidate_name)
    }) {
        return Err(VaultError::DuplicateSecretGroupName);
    }

    Ok(())
}

fn ensure_secret_name_available(
    vault: &VaultRecord,
    metadata: &SecretMetadata,
    skip_secret_id: Option<SecretId>,
) -> VaultResult<()> {
    if vault.secrets.values().any(|secret| {
        secret.metadata.secret_group_id() == metadata.secret_group_id()
            && Some(secret.id) != skip_secret_id
            && names_conflict(secret.metadata.name(), metadata.name())
    }) {
        return Err(VaultError::DuplicateSecretNameWithinSecretGroup);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use secrecy::SecretString;
    use uuid::Uuid;

    use super::{
        BOOTSTRAP_HEADER, BootstrapRecord, ScaffoldStore, VaultStore,
        load_bootstrap, persist_bootstrap,
    };
    use crate::{
        SecretBytes, SecretKind, SecretMetadata, SecretValueUpdate, Settings,
        VaultError, crypto, migrations,
    };

    fn test_path(test_name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push("otty-vault-store-tests");
        path.push(format!("{test_name}-{}", Uuid::new_v4()));
        path.push("vault.db");
        path
    }

    fn passphrase(value: &str) -> SecretString {
        SecretString::new(String::from(value).into_boxed_str())
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn bootstrap_file_round_trips_through_persistence_helpers() {
        let path = test_path("bootstrap-roundtrip");
        let bootstrap = BootstrapRecord {
            passphrase_verifier: crypto::derive_passphrase_verifier(
                &passphrase("master-pass"),
            ),
            format_version: migrations::CURRENT_FORMAT_VERSION,
            settings: Settings::new(
                Duration::from_secs(90),
                Duration::from_secs(15),
            )
            .expect("settings should be valid"),
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .expect("parent directory should be created");
        }
        persist_bootstrap(&path, &bootstrap)
            .expect("bootstrap persistence should succeed");
        let loaded = load_bootstrap(&path)
            .expect("bootstrap load should succeed")
            .expect("bootstrap should exist");

        assert_eq!(loaded.passphrase_verifier, bootstrap.passphrase_verifier);
        assert_eq!(loaded.format_version, bootstrap.format_version);
        assert_eq!(loaded.settings, bootstrap.settings);

        cleanup(&path);
    }

    #[test]
    fn empty_and_corrupted_bootstrap_files_are_reported_cleanly() {
        let empty_path = test_path("bootstrap-empty-file");
        if let Some(parent) = empty_path.parent() {
            fs::create_dir_all(parent)
                .expect("parent directory should be created");
        }
        fs::write(&empty_path, "").expect("empty file should be written");
        assert!(matches!(load_bootstrap(&empty_path), Ok(None)));
        cleanup(&empty_path);

        let corrupted_path = test_path("bootstrap-corrupted");
        if let Some(parent) = corrupted_path.parent() {
            fs::create_dir_all(parent)
                .expect("parent directory should be created");
        }
        fs::write(&corrupted_path, "NOT_A_VAULT")
            .expect("corrupted file should be written");
        assert!(matches!(
            load_bootstrap(&corrupted_path),
            Err(VaultError::VaultCorrupted)
        ));
        cleanup(&corrupted_path);
    }

    #[test]
    fn future_bootstrap_format_version_is_rejected() {
        let path = test_path("future-version");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .expect("parent directory should be created");
        }
        let payload = format!(
            "{BOOTSTRAP_HEADER}\npassphrase_verifier=1\nformat_version={}\nauto_lock_timeout_secs=1800\nclipboard_clear_timeout_secs=30\n",
            migrations::CURRENT_FORMAT_VERSION + 1,
        );
        fs::write(&path, payload).expect("future bootstrap should be written");

        let store = ScaffoldStore;
        let result = store.open_or_bootstrap(&path, &passphrase("master-pass"));

        assert_eq!(result, Err(VaultError::MigrationFailed));
        cleanup(&path);
    }

    #[test]
    fn scaffold_store_supports_bootstrap_crud_and_passphrase_rotation_flow() {
        let path = test_path("store-flow");
        let store = ScaffoldStore;
        store
            .open_or_bootstrap(&path, &passphrase("master-pass"))
            .expect("bootstrap should succeed");

        assert_eq!(
            store.verify_passphrase(&path, &passphrase("master-pass")),
            Ok(())
        );
        assert_eq!(
            store.verify_passphrase(&path, &passphrase("wrong-pass")),
            Err(VaultError::WrongPassphrase)
        );

        let updated_settings =
            Settings::new(Duration::from_secs(120), Duration::from_secs(20))
                .expect("settings should be valid");
        store
            .update_settings(&path, updated_settings)
            .expect("settings update should succeed");
        assert_eq!(store.settings(&path), Ok(updated_settings));

        let alpha = store
            .create_secret_group(&path, "alpha")
            .expect("alpha group should be created");
        let beta = store
            .create_secret_group(&path, "beta")
            .expect("beta group should be created");
        store
            .rename_secret_group(&path, &beta, "gamma")
            .expect("group rename should succeed");

        let secret_to_move = SecretMetadata::new(
            alpha,
            "ssh-prod",
            Some(String::from("root")),
            SecretKind::Password,
            vec![String::from("infra")],
        )
        .expect("metadata should be valid");
        let secret_id = store
            .create_secret(
                &path,
                secret_to_move,
                SecretBytes::from(b"payload-1".to_vec()),
            )
            .expect("secret should be created");

        let moved_metadata = SecretMetadata::new(
            beta,
            "ssh-renamed",
            Some(String::from("deploy")),
            SecretKind::Password,
            vec![String::from("prod")],
        )
        .expect("metadata should be valid");
        store
            .update_secret(
                &path,
                &secret_id,
                moved_metadata.clone(),
                SecretValueUpdate::Set(SecretBytes::from(
                    b"payload-2".to_vec(),
                )),
            )
            .expect("secret update should succeed");

        let search_hits = store
            .find_secrets_by_name(&path, "ssh")
            .expect("search should succeed");
        assert_eq!(search_hits.len(), 1);
        assert_eq!(search_hits[0].secret().id(), secret_id);
        assert_eq!(search_hits[0].secret_group_name(), "gamma");

        let loaded_metadata = store
            .get_secret_metadata(&path, &secret_id)
            .expect("secret metadata should load");
        let loaded_value = store
            .get_secret_value(&path, &secret_id)
            .expect("secret value should load");
        assert_eq!(loaded_metadata, moved_metadata);
        assert_eq!(loaded_value.as_bytes(), b"payload-2");

        let secret_to_cascade = SecretMetadata::new(
            alpha,
            "cascade-me",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        let cascaded_secret_id = store
            .create_secret(
                &path,
                secret_to_cascade,
                SecretBytes::from(Vec::new()),
            )
            .expect("cascade secret should be created");

        store
            .delete_secret_group(&path, &alpha)
            .expect("group delete should succeed");
        assert_eq!(
            store.get_secret_metadata(&path, &cascaded_secret_id),
            Err(VaultError::SecretNotFound)
        );
        assert_eq!(
            store.delete_secret_group(&path, &alpha),
            Err(VaultError::SecretGroupNotFound)
        );

        let snapshot = store
            .secret_groups(&path)
            .expect("snapshot should be available");
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].name(), "gamma");
        assert_eq!(snapshot[0].secrets()[0].id(), secret_id);

        store
            .delete_secret(&path, &secret_id)
            .expect("secret delete should succeed");
        assert_eq!(
            store.delete_secret(&path, &secret_id),
            Err(VaultError::SecretNotFound)
        );

        assert_eq!(
            store.change_passphrase(
                &path,
                &passphrase("wrong-pass"),
                &passphrase("rotated-pass"),
            ),
            Err(VaultError::WrongPassphrase)
        );
        store
            .change_passphrase(
                &path,
                &passphrase("master-pass"),
                &passphrase("rotated-pass"),
            )
            .expect("passphrase rotation should succeed");
        assert_eq!(
            store.verify_passphrase(&path, &passphrase("master-pass")),
            Err(VaultError::WrongPassphrase)
        );
        assert_eq!(
            store.verify_passphrase(&path, &passphrase("rotated-pass")),
            Ok(())
        );

        cleanup(&path);
    }
}
