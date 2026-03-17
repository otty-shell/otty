//! Public facade for the `otty-vault` domain contract.

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
    FolderId, FolderTreeNode, SecretBytes, SecretId, SecretKind,
    SecretMetadata, SecretSearchItem, SecretTreeNode, SecretValueUpdate,
    VaultMetadataNode, VaultMetadataTree, VaultStatus,
};
use secrecy::SecretString;
pub use settings::VaultSettings;
pub use sql_cipher_vault::SqlCipherVault;

/// Domain-level vault API independent of storage and crypto backends.
pub trait Vault {
    /// Unlock the current handle.
    fn unlock(&mut self, passphrase: SecretString) -> VaultResult<()>;
    /// Lock the current handle.
    fn lock(&mut self) -> VaultResult<()>;
    /// Change the vault passphrase.
    fn change_passphrase(
        &mut self,
        passphrase: SecretString,
        new_passphrase: SecretString,
    ) -> VaultResult<()>;
    /// Return the runtime lock status.
    fn status(&self) -> VaultResult<VaultStatus>;
    /// Return current vault settings.
    fn settings(&self) -> VaultResult<VaultSettings>;
    /// Replace current vault settings.
    fn update_settings(&mut self, settings: VaultSettings) -> VaultResult<()>;
    /// Return a full metadata snapshot for the sidebar tree.
    fn metadata_tree(&self) -> VaultResult<VaultMetadataTree>;
    /// Create a folder under `parent_id`, or under root when `None`.
    fn create_folder(
        &mut self,
        parent_id: Option<&FolderId>,
        name: &str,
    ) -> VaultResult<FolderId>;
    /// Delete a folder by id.
    fn delete_folder(&mut self, id: &FolderId) -> VaultResult<()>;
    /// Rename a folder.
    fn rename_folder(&mut self, id: &FolderId, name: &str) -> VaultResult<()>;
    /// Move a folder to a different parent, or to root when `None`.
    fn move_folder(
        &mut self,
        id: &FolderId,
        parent_id: Option<&FolderId>,
    ) -> VaultResult<()>;
    /// Create a secret with metadata and value.
    fn create_secret(
        &mut self,
        metadata: SecretMetadata,
        value: SecretBytes,
    ) -> VaultResult<SecretId>;
    /// Update secret metadata and optionally its value.
    fn update_secret(
        &mut self,
        id: &SecretId,
        metadata: SecretMetadata,
        value: SecretValueUpdate,
    ) -> VaultResult<()>;
    /// Return metadata for a secret without revealing payload.
    fn get_secret_metadata(&self, id: &SecretId)
    -> VaultResult<SecretMetadata>;
    /// Return decrypted secret payload bytes.
    fn get_secret_value(&self, id: &SecretId) -> VaultResult<SecretBytes>;
    /// Search secrets by name and return ids, metadata and computed paths.
    fn find_secrets_by_name(
        &self,
        query: &str,
    ) -> VaultResult<Vec<SecretSearchItem>>;
    /// Delete a secret by id.
    fn delete_secret(&mut self, id: &SecretId) -> VaultResult<()>;
}
