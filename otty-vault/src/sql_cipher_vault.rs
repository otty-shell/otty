use std::path::{Path, PathBuf};

use secrecy::SecretString;

use crate::session::VaultSession;
use crate::store::{ScaffoldStore, VaultStore};
use crate::{
    SecretBytes, SecretGroup, SecretGroupId, SecretId, SecretMetadata,
    SecretSearchItem, SecretValueUpdate, Settings, Vault, VaultError,
    VaultResult, VaultState,
};

/// Concrete vault handle bound to a single storage path.
#[derive(Debug)]
pub struct SqlCipherVault {
    path: PathBuf,
    session: VaultSession,
    store: ScaffoldStore,
}

impl SqlCipherVault {
    /// Open or bootstrap a vault handle for the provided storage path.
    ///
    /// The path is chosen by the caller. Missing parent directories are
    /// created as needed, missing bootstrap metadata is initialized, and the
    /// returned handle starts in [`VaultState::Unlocked`] state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    ///
    /// use otty_vault::{SqlCipherVault, Vault};
    /// use secrecy::SecretString;
    ///
    /// let passphrase =
    ///     SecretString::new(String::from("master-pass").into_boxed_str());
    /// let vault =
    ///     SqlCipherVault::open(Path::new("/tmp/otty/vault.db"), passphrase)
    ///         .expect("vault should open");
    ///
    /// assert!(matches!(
    ///     vault.state(),
    ///     Ok(otty_vault::VaultState::Unlocked)
    /// ));
    /// ```
    pub fn open(path: &Path, passphrase: SecretString) -> VaultResult<Self> {
        let store = ScaffoldStore;
        store.open_or_bootstrap(path, &passphrase)?;

        Ok(Self {
            path: path.to_path_buf(),
            session: VaultSession::new_unlocked(),
            store,
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

        self.store.verify_passphrase(&self.path, &passphrase)?;
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
        self.store
            .change_passphrase(&self.path, &passphrase, &new_passphrase)
    }

    fn state(&self) -> VaultResult<VaultState> {
        Ok(self.session.state())
    }

    fn settings(&self) -> VaultResult<Settings> {
        self.ensure_unlocked()?;
        self.store.settings(&self.path)
    }

    fn update_settings(&mut self, settings: Settings) -> VaultResult<()> {
        self.ensure_unlocked()?;
        self.store.update_settings(&self.path, settings)
    }

    fn secret_groups(&self) -> VaultResult<Vec<SecretGroup>> {
        self.ensure_unlocked()?;
        self.store.secret_groups(&self.path)
    }

    fn create_secret_group(
        &mut self,
        name: &str,
    ) -> VaultResult<SecretGroupId> {
        self.ensure_unlocked()?;
        self.store.create_secret_group(&self.path, name)
    }

    fn delete_secret_group(&mut self, id: &SecretGroupId) -> VaultResult<()> {
        self.ensure_unlocked()?;
        self.store.delete_secret_group(&self.path, id)
    }

    fn rename_secret_group(
        &mut self,
        id: &SecretGroupId,
        name: &str,
    ) -> VaultResult<()> {
        self.ensure_unlocked()?;
        self.store.rename_secret_group(&self.path, id, name)
    }

    fn create_secret(
        &mut self,
        metadata: SecretMetadata,
        value: SecretBytes,
    ) -> VaultResult<SecretId> {
        self.ensure_unlocked()?;
        self.store.create_secret(&self.path, metadata, value)
    }

    fn update_secret(
        &mut self,
        id: &SecretId,
        metadata: SecretMetadata,
        value: SecretValueUpdate,
    ) -> VaultResult<()> {
        self.ensure_unlocked()?;
        self.store.update_secret(&self.path, id, metadata, value)
    }

    fn get_secret_metadata(
        &self,
        id: &SecretId,
    ) -> VaultResult<SecretMetadata> {
        self.ensure_unlocked()?;
        self.store.get_secret_metadata(&self.path, id)
    }

    fn get_secret_value(&self, id: &SecretId) -> VaultResult<SecretBytes> {
        self.ensure_unlocked()?;
        self.store.get_secret_value(&self.path, id)
    }

    fn find_secrets_by_name(
        &self,
        query: &str,
    ) -> VaultResult<Vec<SecretSearchItem>> {
        self.ensure_unlocked()?;
        self.store.find_secrets_by_name(&self.path, query)
    }

    fn delete_secret(&mut self, id: &SecretId) -> VaultResult<()> {
        self.ensure_unlocked()?;
        self.store.delete_secret(&self.path, id)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use secrecy::SecretString;
    use uuid::Uuid;

    use crate::{
        SecretBytes, SecretKind, SecretMetadata, SecretValueUpdate,
        SqlCipherVault, Vault, VaultError, VaultState,
    };

    fn test_path(test_name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push("otty-vault-tests");
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
    fn bootstrap_creates_storage_path_and_starts_unlocked_with_empty_secret_groups()
     {
        let path = test_path("bootstrap-empty");

        let vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        assert!(path.exists());
        assert_eq!(vault.state(), Ok(VaultState::Unlocked));
        assert_eq!(vault.secret_groups(), Ok(Vec::new()));

        cleanup(&path);
    }

    #[test]
    fn repeated_lock_in_locked_state_is_a_safe_no_op() {
        let path = test_path("lock-noop");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        vault.lock().expect("first lock should succeed");
        vault.lock().expect("second lock should stay a no-op");

        assert_eq!(vault.state(), Ok(VaultState::Locked));

        cleanup(&path);
    }

    #[test]
    fn repeated_unlock_in_unlocked_state_is_a_safe_no_op() {
        let path = test_path("unlock-noop");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        vault
            .unlock(passphrase("master-pass"))
            .expect("unlock should stay a no-op");

        assert_eq!(vault.state(), Ok(VaultState::Unlocked));

        cleanup(&path);
    }

    #[test]
    fn existing_vault_rejects_wrong_passphrase() {
        let path = test_path("wrong-passphrase");

        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");
        vault.lock().expect("lock should succeed");

        let error = vault
            .unlock(passphrase("wrong-pass"))
            .expect_err("unlock should reject wrong passphrase");

        assert_eq!(error, VaultError::WrongPassphrase);
        assert_ne!(error, VaultError::VaultLocked);

        cleanup(&path);
    }

    #[test]
    fn secret_groups_and_secrets_are_returned_in_deterministic_name_order() {
        let path = test_path("sorted-snapshot");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        let zeta = vault
            .create_secret_group("zeta")
            .expect("zeta group should be created");
        let alpha = vault
            .create_secret_group("alpha")
            .expect("alpha group should be created");

        let alpha_secret_b = SecretMetadata::new(
            alpha,
            "db",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        let alpha_secret_a = SecretMetadata::new(
            alpha,
            "api",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        let zeta_secret = SecretMetadata::new(
            zeta,
            "zzz",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");

        vault
            .create_secret(alpha_secret_b, SecretBytes::default())
            .expect("db secret should be created");
        vault
            .create_secret(zeta_secret, SecretBytes::default())
            .expect("zzz secret should be created");
        vault
            .create_secret(alpha_secret_a, SecretBytes::default())
            .expect("api secret should be created");

        let secret_groups =
            vault.secret_groups().expect("snapshot should be available");

        assert_eq!(secret_groups[0].name(), "alpha");
        assert_eq!(secret_groups[1].name(), "zeta");
        assert_eq!(secret_groups[0].secrets()[0].metadata().name(), "api");
        assert_eq!(secret_groups[0].secrets()[1].metadata().name(), "db");
        assert_eq!(secret_groups[1].secrets()[0].metadata().name(), "zzz");

        cleanup(&path);
    }

    #[test]
    fn invalid_secret_group_reference_is_rejected() {
        let path = test_path("missing-group");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        let metadata = SecretMetadata::new(
            crate::SecretGroupId::new(),
            "ssh-prod",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");

        let error = vault
            .create_secret(metadata, SecretBytes::default())
            .expect_err("creating a secret in a missing group should fail");

        assert_eq!(error, VaultError::SecretGroupNotFound);

        cleanup(&path);
    }

    #[test]
    fn update_secret_can_move_between_groups_without_changing_secret_id() {
        let path = test_path("move-secret");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        let group_a = vault
            .create_secret_group("alpha")
            .expect("alpha group should be created");
        let group_b = vault
            .create_secret_group("beta")
            .expect("beta group should be created");
        let create_metadata = SecretMetadata::new(
            group_a,
            "ssh-prod",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        let secret_id = vault
            .create_secret(
                create_metadata,
                SecretBytes::from(b"first".to_vec()),
            )
            .expect("secret should be created");

        let update_metadata = SecretMetadata::new(
            group_b,
            "ssh-prod",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        vault
            .update_secret(&secret_id, update_metadata, SecretValueUpdate::Keep)
            .expect("secret move should succeed");

        let secret_groups =
            vault.secret_groups().expect("snapshot should be available");
        let alpha_secrets = secret_groups
            .iter()
            .find(|group| group.id() == group_a)
            .expect("alpha group should exist")
            .secrets();
        let beta_secrets = secret_groups
            .iter()
            .find(|group| group.id() == group_b)
            .expect("beta group should exist")
            .secrets();

        assert!(alpha_secrets.is_empty());
        assert_eq!(beta_secrets.len(), 1);
        assert_eq!(beta_secrets[0].id(), secret_id);

        cleanup(&path);
    }

    #[test]
    fn duplicate_names_use_trimmed_comparison_rules() {
        let path = test_path("duplicate-names");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        let group_id = vault
            .create_secret_group("infra")
            .expect("group should be created");
        let duplicate_group_error = vault
            .create_secret_group(" infra ")
            .expect_err("duplicate trimmed group name should fail");

        let metadata = SecretMetadata::new(
            group_id,
            "ssh-prod",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        vault
            .create_secret(metadata, SecretBytes::default())
            .expect("secret should be created");
        let duplicate_secret_metadata = SecretMetadata::new(
            group_id,
            " ssh-prod ",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        let duplicate_secret_error = vault
            .create_secret(duplicate_secret_metadata, SecretBytes::default())
            .expect_err("duplicate trimmed secret name should fail");

        assert_eq!(duplicate_group_error, VaultError::DuplicateSecretGroupName);
        assert_eq!(
            duplicate_secret_error,
            VaultError::DuplicateSecretNameWithinSecretGroup
        );

        cleanup(&path);
    }

    #[test]
    fn find_secrets_by_name_returns_secret_and_group_name() {
        let path = test_path("find-secrets");
        let mut vault = SqlCipherVault::open(&path, passphrase("master-pass"))
            .expect("open should work");

        let group_id = vault
            .create_secret_group("infra")
            .expect("group should be created");
        let metadata = SecretMetadata::new(
            group_id,
            "ssh-prod",
            Some(String::from("root")),
            SecretKind::Password,
            vec![String::from("prod")],
        )
        .expect("metadata should be valid");
        let secret_id = vault
            .create_secret(metadata, SecretBytes::from(b"payload".to_vec()))
            .expect("secret should be created");

        let items = vault
            .find_secrets_by_name("ssh")
            .expect("search should succeed");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].secret_group_name(), "infra");
        assert_eq!(items[0].secret().id(), secret_id);

        cleanup(&path);
    }
}
