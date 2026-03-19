use thiserror::Error;

/// Stable public errors returned by the vault domain API.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum VaultError {
    /// The current handle is locked and cannot serve read/write operations.
    #[error("vault is locked")]
    VaultLocked,
    /// The provided passphrase does not unlock the current vault.
    #[error("wrong passphrase")]
    WrongPassphrase,
    /// The persisted bootstrap metadata is inconsistent or unreadable.
    #[error("vault is corrupted")]
    VaultCorrupted,
    /// The requested secret does not exist.
    #[error("secret not found")]
    SecretNotFound,
    /// The requested secret group does not exist.
    #[error("secret group not found")]
    SecretGroupNotFound,
    /// A top-level secret group with the same trimmed name already exists.
    #[error("duplicate secret group name")]
    DuplicateSecretGroupName,
    /// A secret with the same trimmed name already exists in the target group.
    #[error("duplicate secret name within secret group")]
    DuplicateSecretNameWithinSecretGroup,
    /// The provided settings payload is invalid.
    #[error("invalid settings")]
    InvalidSettings,
    /// The provided name is empty or contains only whitespace.
    #[error("invalid name")]
    InvalidName,
    /// A storage migration failed or the persisted format version is unsupported.
    #[error("migration failed")]
    MigrationFailed,
    /// An internal storage operation failed.
    #[error("storage error")]
    StorageError,
    /// An internal cryptographic operation failed.
    #[error("crypto error")]
    CryptoError,
}

/// Convenience result alias for vault operations.
pub type VaultResult<T> = Result<T, VaultError>;

#[cfg(test)]
mod tests {
    use super::VaultError;

    #[test]
    fn wrong_passphrase_and_vault_locked_remain_distinct() {
        assert_ne!(VaultError::WrongPassphrase, VaultError::VaultLocked);
    }

    #[test]
    fn public_errors_do_not_embed_sensitive_material() {
        let display = format!("{}", VaultError::WrongPassphrase);
        let debug = format!("{:?}", VaultError::CryptoError);

        assert!(!display.contains("super-secret-passphrase"));
        assert!(!display.contains("ciphertext"));
        assert!(!debug.contains("super-secret-passphrase"));
        assert!(!debug.contains("raw-secret-value"));
    }
}
