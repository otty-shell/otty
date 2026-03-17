use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use secrecy::{ExposeSecret, SecretString};

use crate::errors::VaultResult;
use crate::model::SecretBytes;

/// Minimal cryptographic interface used by the foundation storage scaffold.
pub(crate) trait VaultCrypto {
    fn fingerprint_passphrase(&self, passphrase: &SecretString) -> u64;
    fn encrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes>;
    fn decrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes>;
}

/// Temporary crypto implementation used until SQLCipher-backed crypto is wired.
pub(crate) struct NoopCrypto;

impl VaultCrypto for NoopCrypto {
    fn fingerprint_passphrase(&self, passphrase: &SecretString) -> u64 {
        let mut hasher = DefaultHasher::new();
        passphrase.expose_secret().hash(&mut hasher);
        hasher.finish()
    }

    fn encrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes> {
        Ok(value.clone())
    }

    fn decrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes> {
        Ok(value.clone())
    }
}

pub(crate) fn fingerprint_passphrase(passphrase: &SecretString) -> u64 {
    NoopCrypto.fingerprint_passphrase(passphrase)
}

pub(crate) fn encrypt_secret(value: &SecretBytes) -> VaultResult<SecretBytes> {
    NoopCrypto.encrypt_secret(value)
}

pub(crate) fn decrypt_secret(value: &SecretBytes) -> VaultResult<SecretBytes> {
    NoopCrypto.decrypt_secret(value)
}
