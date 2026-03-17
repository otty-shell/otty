use std::path::{Path, PathBuf};

use secrecy::SecretString;

use crate::session::VaultSession;
use crate::{
    FolderId, SecretBytes, SecretId, SecretMetadata, SecretSearchItem,
    SecretValueUpdate, Vault, VaultError, VaultMetadataTree, VaultResult,
    VaultSettings, VaultStatus, store,
};

/// Concrete vault handle bound to a single storage path.
#[derive(Debug)]
pub struct SqlCipherVault {
    path: PathBuf,
    session: VaultSession,
}

impl SqlCipherVault {
    /// Open or bootstrap a vault file at `path` using `passphrase`.
    ///
    /// If no vault is initialized yet, parent directories and the storage file
    /// are created, bootstrap metadata is initialized, and the returned handle
    /// starts as [`VaultStatus::Unlocked`].
    pub fn open(path: &Path, passphrase: SecretString) -> VaultResult<Self> {
        store::open_or_bootstrap(path, &passphrase)?;

        Ok(Self {
            path: path.to_path_buf(),
            session: VaultSession::new_unlocked(),
        })
    }

    fn ensure_unlocked(&self) -> VaultResult<()> {
        if self.session.is_unlocked() {
            return Ok(());
        }

        Err(VaultError::VaultLocked)
    }
}

impl Vault for SqlCipherVault {
    fn unlock(&mut self, passphrase: SecretString) -> VaultResult<()> {
        if self.session.is_unlocked() {
            return Ok(());
        }

        store::verify_passphrase(&self.path, &passphrase)?;
        self.session.unlock();

        Ok(())
    }

    fn lock(&mut self) -> VaultResult<()> {
        if !self.session.is_unlocked() {
            return Ok(());
        }

        self.session.lock();
        Ok(())
    }

    fn change_passphrase(
        &mut self,
        passphrase: SecretString,
        new_passphrase: SecretString,
    ) -> VaultResult<()> {
        self.ensure_unlocked()?;
        store::change_passphrase(&self.path, &passphrase, &new_passphrase)
    }

    fn status(&self) -> VaultResult<VaultStatus> {
        Ok(self.session.status())
    }

    fn settings(&self) -> VaultResult<VaultSettings> {
        self.ensure_unlocked()?;
        store::settings(&self.path)
    }

    fn update_settings(&mut self, settings: VaultSettings) -> VaultResult<()> {
        self.ensure_unlocked()?;
        store::update_settings(&self.path, settings)
    }

    fn metadata_tree(&self) -> VaultResult<VaultMetadataTree> {
        self.ensure_unlocked()?;
        store::metadata_tree(&self.path)
    }

    fn create_folder(
        &mut self,
        parent_id: Option<&FolderId>,
        name: &str,
    ) -> VaultResult<FolderId> {
        self.ensure_unlocked()?;
        store::create_folder(&self.path, parent_id, name)
    }

    fn delete_folder(&mut self, id: &FolderId) -> VaultResult<()> {
        self.ensure_unlocked()?;
        store::delete_folder(&self.path, id)
    }

    fn rename_folder(&mut self, id: &FolderId, name: &str) -> VaultResult<()> {
        self.ensure_unlocked()?;
        store::rename_folder(&self.path, id, name)
    }

    fn move_folder(
        &mut self,
        id: &FolderId,
        parent_id: Option<&FolderId>,
    ) -> VaultResult<()> {
        self.ensure_unlocked()?;
        store::move_folder(&self.path, id, parent_id)
    }

    fn create_secret(
        &mut self,
        metadata: SecretMetadata,
        value: SecretBytes,
    ) -> VaultResult<SecretId> {
        self.ensure_unlocked()?;
        store::create_secret(&self.path, metadata, value)
    }

    fn update_secret(
        &mut self,
        id: &SecretId,
        metadata: SecretMetadata,
        value: SecretValueUpdate,
    ) -> VaultResult<()> {
        self.ensure_unlocked()?;
        store::update_secret(&self.path, id, metadata, value)
    }

    fn get_secret_metadata(
        &self,
        id: &SecretId,
    ) -> VaultResult<SecretMetadata> {
        self.ensure_unlocked()?;
        store::get_secret_metadata(&self.path, id)
    }

    fn get_secret_value(&self, id: &SecretId) -> VaultResult<SecretBytes> {
        self.ensure_unlocked()?;
        store::get_secret_value(&self.path, id)
    }

    fn find_secrets_by_name(
        &self,
        query: &str,
    ) -> VaultResult<Vec<SecretSearchItem>> {
        self.ensure_unlocked()?;
        store::find_secrets_by_name(&self.path, query)
    }

    fn delete_secret(&mut self, id: &SecretId) -> VaultResult<()> {
        self.ensure_unlocked()?;
        store::delete_secret(&self.path, id)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::Duration;

    use secrecy::SecretString;
    use uuid::Uuid;

    use crate::{
        FolderId, SecretBytes, SecretKind, SecretMetadata, SecretValueUpdate,
        SqlCipherVault, Vault, VaultError, VaultMetadataNode, VaultSettings,
        VaultStatus,
    };

    fn test_path(test_name: &str) -> PathBuf {
        let mut base = std::env::temp_dir();
        base.push("otty-vault-tests");
        base.push(format!("{test_name}-{}", Uuid::new_v4()));
        base.push("vault.db");
        base
    }

    fn passphrase(value: &str) -> SecretString {
        SecretString::new(value.to_string().into_boxed_str())
    }

    fn root_id(vault: &SqlCipherVault) -> FolderId {
        vault
            .metadata_tree()
            .expect("metadata tree must be available")
            .root()
            .folder_id()
    }

    fn cleanup(path: &Path) {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("test mutex should not be poisoned")
    }

    #[test]
    fn open_bootstraps_missing_path_and_returns_unlocked() {
        let _guard = test_guard();
        let path = test_path("open-bootstrap");
        let vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open must bootstrap vault");
        assert!(path.exists());
        assert_eq!(
            vault.status().expect("status must be available"),
            VaultStatus::Unlocked
        );
        cleanup(&path);
    }

    #[test]
    fn metadata_tree_always_contains_root_with_stable_id() {
        let _guard = test_guard();
        let path = test_path("root-stable");
        let first = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let first_root =
            first.metadata_tree().expect("tree").root().folder_id();
        drop(first);
        let second = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let second_root =
            second.metadata_tree().expect("tree").root().folder_id();
        assert_eq!(first_root, second_root);
        cleanup(&path);
    }

    #[test]
    fn root_folder_cannot_be_deleted() {
        let _guard = test_guard();
        let path = test_path("root-delete-guard");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let root_id = root_id(&vault);
        let error = vault
            .delete_folder(&root_id)
            .expect_err("root delete must fail");
        assert_eq!(error, VaultError::StorageError);
        cleanup(&path);
    }

    #[test]
    fn create_folder_none_creates_under_root() {
        let _guard = test_guard();
        let path = test_path("create-folder-root");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let folder_id = vault
            .create_folder(None, "alpha")
            .expect("folder must be created");
        let tree = vault.metadata_tree().expect("tree");
        let root = tree.root();
        let created = root.children().iter().any(|node| {
            matches!(
                node,
                VaultMetadataNode::Folder(folder)
                    if folder.folder_id() == folder_id
            )
        });
        assert!(created);
        cleanup(&path);
    }

    #[test]
    fn move_folder_none_moves_folder_under_root() {
        let _guard = test_guard();
        let path = test_path("move-folder-root");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let parent =
            vault.create_folder(None, "parent").expect("parent folder");
        let child = vault
            .create_folder(Some(&parent), "child")
            .expect("child folder");
        vault
            .move_folder(&child, None)
            .expect("move to root should succeed");

        let tree = vault.metadata_tree().expect("tree");
        let root_has_child = tree.root().children().iter().any(|node| {
            matches!(
                node,
                VaultMetadataNode::Folder(folder)
                    if folder.folder_id() == child
            )
        });
        assert!(root_has_child);
        cleanup(&path);
    }

    #[test]
    fn secret_metadata_folder_must_exist() {
        let _guard = test_guard();
        let path = test_path("secret-folder-exists");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let metadata = SecretMetadata::new(
            FolderId::new(),
            "ssh-key",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata shape is valid");
        let result = vault.create_secret(metadata, SecretBytes::default());
        assert_eq!(result, Err(VaultError::FolderNotFound));
        cleanup(&path);
    }

    #[test]
    fn create_and_update_secret_respect_keep_and_set_semantics() {
        let _guard = test_guard();
        let path = test_path("secret-update");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let root = root_id(&vault);
        let metadata = SecretMetadata::new(
            root,
            "db-password",
            Some("user".to_string()),
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata");
        let secret_id = vault
            .create_secret(metadata.clone(), SecretBytes::new(b"abc".to_vec()))
            .expect("secret create");

        let mut updated_metadata = metadata.clone();
        updated_metadata.set_username(Some("another-user".to_string()));
        vault
            .update_secret(
                &secret_id,
                updated_metadata,
                SecretValueUpdate::Keep,
            )
            .expect("metadata update");
        assert_eq!(
            vault
                .get_secret_value(&secret_id)
                .expect("secret value")
                .as_bytes(),
            b"abc"
        );

        vault
            .update_secret(
                &secret_id,
                metadata,
                SecretValueUpdate::Set(SecretBytes::default()),
            )
            .expect("empty value update");
        assert!(
            vault
                .get_secret_value(&secret_id)
                .expect("secret value")
                .is_empty()
        );
        cleanup(&path);
    }

    #[test]
    fn metadata_tree_children_order_is_deterministic() {
        let _guard = test_guard();
        let path = test_path("tree-order");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let root = root_id(&vault);
        let _ = vault.create_folder(None, "b").expect("folder b");
        let _ = vault.create_folder(None, "a").expect("folder a");
        let secret_c = SecretMetadata::new(
            root,
            "c",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata c");
        let secret_d = SecretMetadata::new(
            root,
            "d",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata d");
        let _ = vault
            .create_secret(secret_d, SecretBytes::default())
            .expect("secret d");
        let _ = vault
            .create_secret(secret_c, SecretBytes::default())
            .expect("secret c");

        let tree = vault.metadata_tree().expect("tree");
        let labels: Vec<String> = tree
            .root()
            .children()
            .iter()
            .map(|node| match node {
                VaultMetadataNode::Folder(folder) => {
                    format!("folder:{}", folder.name())
                },
                VaultMetadataNode::Secret(secret) => {
                    format!("secret:{}", secret.metadata().name())
                },
            })
            .collect();
        assert_eq!(
            labels,
            vec![
                "folder:a".to_string(),
                "folder:b".to_string(),
                "secret:c".to_string(),
                "secret:d".to_string(),
            ]
        );
        cleanup(&path);
    }

    #[test]
    fn duplicate_name_check_uses_trimmed_names_within_parent() {
        let _guard = test_guard();
        let path = test_path("duplicate-names");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let _ = vault.create_folder(None, "foo").expect("first folder");
        let duplicate = vault.create_folder(None, " foo ");
        assert_eq!(duplicate, Err(VaultError::DuplicateNameWithinFolder));
        cleanup(&path);
    }

    #[test]
    fn wrong_passphrase_and_vault_locked_are_distinct_errors() {
        let _guard = test_guard();
        let path = test_path("wrong-passphrase");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        vault.lock().expect("lock");
        let wrong_passphrase_error = vault
            .unlock(passphrase("incorrect-pass"))
            .expect_err("unlock with wrong passphrase must fail");
        let locked_error = vault
            .settings()
            .expect_err("settings when locked must fail");
        assert_eq!(wrong_passphrase_error, VaultError::WrongPassphrase);
        assert_eq!(locked_error, VaultError::VaultLocked);
        assert_ne!(wrong_passphrase_error, locked_error);

        let open_with_wrong_pass =
            SqlCipherVault::open(&path, passphrase("incorrect-pass"));
        assert!(matches!(
            open_with_wrong_pass,
            Err(VaultError::WrongPassphrase)
        ));
        cleanup(&path);
    }

    #[test]
    fn lock_and_unlock_are_safe_noops_when_repeated() {
        let _guard = test_guard();
        let path = test_path("lock-unlock-noop");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        vault.lock().expect("first lock");
        vault.lock().expect("second lock no-op");
        assert_eq!(vault.status().expect("status"), VaultStatus::Locked);

        vault.unlock(passphrase("master-pass")).expect("unlock");
        vault
            .unlock(passphrase("wrong-but-noop-when-unlocked"))
            .expect("second unlock no-op");
        assert_eq!(vault.status().expect("status"), VaultStatus::Unlocked);
        cleanup(&path);
    }

    #[test]
    fn secret_metadata_and_value_are_separate_read_flows() {
        let _guard = test_guard();
        let path = test_path("metadata-value-separation");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let metadata = SecretMetadata::new(
            root_id(&vault),
            "ssh",
            None,
            SecretKind::Password,
            vec!["prod".to_string()],
        )
        .expect("metadata");
        let secret_id = vault
            .create_secret(
                metadata.clone(),
                SecretBytes::new(b"payload".to_vec()),
            )
            .expect("create secret");

        let read_metadata = vault
            .get_secret_metadata(&secret_id)
            .expect("read metadata");
        assert_eq!(read_metadata, metadata);
        let read_value =
            vault.get_secret_value(&secret_id).expect("read value");
        assert_eq!(read_value.as_bytes(), b"payload");
        cleanup(&path);
    }

    #[test]
    fn find_secrets_returns_id_metadata_and_path() {
        let _guard = test_guard();
        let path = test_path("secret-search");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let metadata = SecretMetadata::new(
            root_id(&vault),
            "api-token",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata");
        let secret_id = vault
            .create_secret(metadata.clone(), SecretBytes::default())
            .expect("secret");

        let results = vault
            .find_secrets_by_name("token")
            .expect("search must succeed");
        assert_eq!(results.len(), 1);
        let item = &results[0];
        assert_eq!(item.secret_id(), secret_id);
        assert_eq!(item.metadata(), &metadata);
        assert_eq!(item.path(), "root/api-token");
        cleanup(&path);
    }

    #[test]
    fn error_display_and_debug_do_not_leak_secret_material() {
        let _guard = test_guard();
        let candidate_secret = "never-print-me";
        let error = VaultError::WrongPassphrase;
        assert!(!format!("{error}").contains(candidate_secret));
        assert!(!format!("{error:?}").contains(candidate_secret));
    }

    #[test]
    fn settings_can_be_updated_with_valid_timeouts() {
        let _guard = test_guard();
        let path = test_path("settings-update");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open");
        let settings = VaultSettings::new(
            Duration::from_secs(90),
            Duration::from_secs(15),
        )
        .expect("valid settings");
        vault
            .update_settings(settings)
            .expect("settings update must succeed");
        assert_eq!(vault.settings().expect("settings read"), settings);
        cleanup(&path);
    }

    #[test]
    fn bootstrap_metadata_survives_registry_reset() {
        let _guard = test_guard();
        let path = test_path("bootstrap-persistence");
        let settings = VaultSettings::new(
            Duration::from_secs(45),
            Duration::from_secs(12),
        )
        .expect("valid settings");

        {
            let mut vault =
                SqlCipherVault::open(&path, passphrase("master-pass"))
                    .expect("open");
            vault
                .update_settings(settings)
                .expect("settings update must succeed");
        }

        crate::store::clear_registry_for_tests()
            .expect("registry reset must succeed");

        let reopened = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("reopen with persisted bootstrap");
        assert_eq!(reopened.settings().expect("settings"), settings);

        let wrong_passphrase =
            SqlCipherVault::open(&path, passphrase("incorrect-pass"));
        assert!(matches!(wrong_passphrase, Err(VaultError::WrongPassphrase)));

        cleanup(&path);
    }
}
