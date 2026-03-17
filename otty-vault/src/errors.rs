use thiserror::Error;

/// Stable public errors returned by the vault domain API.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum VaultError {
    /// The vault exists but the current handle is locked.
    #[error("vault is locked")]
    VaultLocked,
    /// The provided passphrase is invalid for this vault.
    #[error("wrong passphrase")]
    WrongPassphrase,
    /// The vault data is inconsistent or unreadable.
    #[error("vault is corrupted")]
    VaultCorrupted,
    /// The requested secret does not exist.
    #[error("secret not found")]
    SecretNotFound,
    /// The requested folder does not exist.
    #[error("folder not found")]
    FolderNotFound,
    /// The operation requires an existing secret identifier.
    #[error("missing secret identifier")]
    MissingSecretId,
    /// A sibling folder or secret already uses the same trimmed name.
    #[error("duplicate name within folder")]
    DuplicateNameWithinFolder,
    /// The provided settings payload is invalid.
    #[error("invalid vault settings")]
    InvalidSettings,
    /// Schema migration could not be completed.
    #[error("vault migration failed")]
    MigrationFailed,
    /// Storage operations failed.
    #[error("vault storage error")]
    StorageError,
    /// Cryptographic operations failed.
    #[error("vault crypto error")]
    CryptoError,
}

/// Convenience result alias for vault operations.
pub type VaultResult<T> = Result<T, VaultError>;
