//! How OTTY tells keys apart without keeping them (spec Decision 2): a key
//! found in an agent's file is turned into a [`KeyFingerprint`], and the
//! key itself is dropped at once.

use std::fmt;

use sha2::{Digest, Sha256};

/// A hash of a key: the same key gives the same fingerprint, so a key used
/// in two places is one account, and the key can not be read back from it.
///
/// It is SHA-256, a cryptographic hash whose result never changes between
/// versions, so a fingerprint stays the same from run to run and the key
/// can not be found from it. It is kept in memory only.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyFingerprint([u8; 32]);

impl KeyFingerprint {
    /// The fingerprint of `key`, without the spaces around it.
    ///
    /// ```
    /// use otty_agents::KeyFingerprint;
    ///
    /// assert_eq!(
    ///     KeyFingerprint::of("sk-made-up-key"),
    ///     KeyFingerprint::of(" sk-made-up-key\n"),
    /// );
    /// assert_ne!(
    ///     KeyFingerprint::of("sk-made-up-key"),
    ///     KeyFingerprint::of("sk-other-key"),
    /// );
    /// ```
    pub fn of(key: &str) -> Self {
        let hash = Sha256::digest(key.trim().as_bytes());

        Self(hash.into())
    }
}

impl fmt::Debug for KeyFingerprint {
    /// Only the first bytes, enough to tell two fingerprints apart in a
    /// test failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start: String = self.0[..4]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        write!(f, "KeyFingerprint({start}…)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-made-up-0000-key-a1f3";

    #[test]
    fn the_same_key_has_the_same_fingerprint_and_another_key_another() {
        assert_eq!(KeyFingerprint::of(KEY), KeyFingerprint::of(KEY));
        assert_eq!(
            KeyFingerprint::of(KEY),
            KeyFingerprint::of(&format!("  {KEY}\n"))
        );
        assert_ne!(
            KeyFingerprint::of(KEY),
            KeyFingerprint::of("sk-made-up-0000-key-a1f4")
        );
    }

    /// The hash never changes between versions, so the fingerprint of a
    /// key is the same in every run: the known SHA-256 of "" and of "a".
    #[test]
    fn the_fingerprint_is_sha_256() {
        let hex = |fingerprint: KeyFingerprint| -> String {
            fingerprint
                .0
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        };

        assert_eq!(
            hex(KeyFingerprint::of("")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(KeyFingerprint::of("a")),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }

    #[test]
    fn the_debug_output_shows_only_the_start_of_the_fingerprint() {
        let debug = format!("{:?}", KeyFingerprint::of("a"));

        assert_eq!(debug, "KeyFingerprint(ca978112…)");
    }
}
