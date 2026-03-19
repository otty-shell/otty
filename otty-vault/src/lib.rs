//! Stable public facade for the `otty-vault` domain contract.
//!
//! The crate accepts a vault storage path from its caller and does not impose
//! an application-specific filesystem location. The current storage backend is
//! a foundation scaffold that fixes the domain API, lock-state semantics, and
//! migration/crypto boundaries before the SQLCipher-backed implementation lands.

mod crypto;
mod errors;
mod migrations;
mod model;
mod session;
mod settings;
mod sql_cipher_vault;
mod store;

pub use errors::{VaultError, VaultResult};
pub use model::{
    Secret, SecretBytes, SecretGroup, SecretGroupId, SecretId, SecretKind,
    SecretMetadata, SecretSearchItem, SecretValueUpdate, VaultState,
};
use secrecy::SecretString;
pub use settings::Settings;
pub use sql_cipher_vault::SqlCipherVault;

/// Domain-level vault API independent of SQLCipher and storage-driver details.
pub trait Vault {
    /// Unlock the current handle with the provided master passphrase.
    fn unlock(&mut self, passphrase: SecretString) -> VaultResult<()>;

    /// Lock the current handle and stop read/write access.
    fn lock(&mut self) -> VaultResult<()>;

    /// Change the current master passphrase while the handle is unlocked.
    fn change_passphrase(
        &mut self,
        passphrase: SecretString,
        new_passphrase: SecretString,
    ) -> VaultResult<()>;

    /// Return the runtime lock state of the opened handle.
    fn state(&self) -> VaultResult<VaultState>;

    /// Return the persisted vault settings.
    fn settings(&self) -> VaultResult<Settings>;

    /// Replace the persisted vault settings.
    fn update_settings(&mut self, settings: Settings) -> VaultResult<()>;

    /// Return the full `secret_groups -> secrets` snapshot for the UI tree.
    fn secret_groups(&self) -> VaultResult<Vec<SecretGroup>>;

    /// Create a top-level secret group and return its generated identifier.
    fn create_secret_group(&mut self, name: &str)
    -> VaultResult<SecretGroupId>;

    /// Delete a secret group and all secrets stored inside it.
    fn delete_secret_group(&mut self, id: &SecretGroupId) -> VaultResult<()>;

    /// Rename a top-level secret group.
    fn rename_secret_group(
        &mut self,
        id: &SecretGroupId,
        name: &str,
    ) -> VaultResult<()>;

    /// Create a new secret and return its generated identifier.
    fn create_secret(
        &mut self,
        metadata: SecretMetadata,
        value: SecretBytes,
    ) -> VaultResult<SecretId>;

    /// Update an existing secret and optionally replace its value.
    fn update_secret(
        &mut self,
        id: &SecretId,
        metadata: SecretMetadata,
        value: SecretValueUpdate,
    ) -> VaultResult<()>;

    /// Return non-sensitive metadata for a secret.
    fn get_secret_metadata(&self, id: &SecretId)
    -> VaultResult<SecretMetadata>;

    /// Return the secret payload for explicit read flows.
    fn get_secret_value(&self, id: &SecretId) -> VaultResult<SecretBytes>;

    /// Search secrets by name and return UI-oriented search items.
    fn find_secrets_by_name(
        &self,
        query: &str,
    ) -> VaultResult<Vec<SecretSearchItem>>;

    /// Delete a secret by identifier.
    fn delete_secret(&mut self, id: &SecretId) -> VaultResult<()>;
}
