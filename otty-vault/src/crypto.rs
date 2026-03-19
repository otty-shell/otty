use secrecy::{ExposeSecret, SecretString};

use crate::errors::VaultResult;
use crate::model::SecretBytes;

/// Private passphrase verifier used by the foundation storage scaffold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PassphraseVerifier(u64);

impl PassphraseVerifier {
    pub(crate) fn into_u64(self) -> u64 {
        self.0
    }

    pub(crate) fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

/// Internal crypto interface kept stable while real SQLCipher crypto lands.
pub(crate) trait VaultCrypto {
    fn derive_passphrase_verifier(
        &self,
        passphrase: &SecretString,
    ) -> PassphraseVerifier;
    fn encrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes>;
    fn decrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes>;
}

/// Foundation-stage placeholder crypto.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ScaffoldCrypto;

impl VaultCrypto for ScaffoldCrypto {
    fn derive_passphrase_verifier(
        &self,
        passphrase: &SecretString,
    ) -> PassphraseVerifier {
        // This is a deterministic scaffold-only verifier, not production crypto.
        let mut value = 0xcbf29ce484222325_u64;
        for byte in passphrase.expose_secret().as_bytes() {
            value ^= u64::from(*byte);
            value = value.wrapping_mul(0x100000001b3);
        }
        PassphraseVerifier(value)
    }

    fn encrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes> {
        Ok(value.clone())
    }

    fn decrypt_secret(&self, value: &SecretBytes) -> VaultResult<SecretBytes> {
        Ok(value.clone())
    }
}

pub(crate) fn derive_passphrase_verifier(
    passphrase: &SecretString,
) -> PassphraseVerifier {
    ScaffoldCrypto.derive_passphrase_verifier(passphrase)
}

pub(crate) fn encrypt_secret(value: &SecretBytes) -> VaultResult<SecretBytes> {
    ScaffoldCrypto.encrypt_secret(value)
}

pub(crate) fn decrypt_secret(value: &SecretBytes) -> VaultResult<SecretBytes> {
    ScaffoldCrypto.decrypt_secret(value)
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::{decrypt_secret, derive_passphrase_verifier, encrypt_secret};
    use crate::SecretBytes;

    fn passphrase(value: &str) -> SecretString {
        SecretString::new(String::from(value).into_boxed_str())
    }

    #[test]
    fn passphrase_verifier_is_deterministic_and_sensitive_to_input() {
        let first = derive_passphrase_verifier(&passphrase("alpha"));
        let same = derive_passphrase_verifier(&passphrase("alpha"));
        let different = derive_passphrase_verifier(&passphrase("beta"));

        assert_eq!(first, same);
        assert_ne!(first, different);
    }

    #[test]
    fn encrypt_and_decrypt_round_trip_secret_bytes() {
        let payload = SecretBytes::from(b"payload".to_vec());
        let encrypted = encrypt_secret(&payload)
            .expect("placeholder encryption should work");
        let decrypted = decrypt_secret(&encrypted)
            .expect("placeholder decryption should work");

        assert_eq!(decrypted.as_bytes(), b"payload");
    }
}
