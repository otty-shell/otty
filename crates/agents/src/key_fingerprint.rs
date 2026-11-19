//! How OTTY tells keys apart without keeping them (spec Decision 2): a key
//! found in an agent's file is turned into a [`KeyFingerprint`] and the
//! last characters it is shown by, and the key itself is dropped at once.

use std::fmt;

use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

use crate::json_file::filled;

/// How many characters at the end of a key are shown, as the providers'
/// own pages show them.
const SHOWN_ENDING: usize = 4;
/// The shortest key whose ending is shown: real keys are much longer, and
/// the ending of a short one would show too much of it.
const SHORTEST_SHOWN_KEY: usize = 16;

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

/// A key seen in an agent's file: its fingerprint and the last characters
/// it is shown by. The key itself is not kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeenKey {
    fingerprint: KeyFingerprint,
    ending: Option<String>,
}

impl SeenKey {
    /// The fingerprint of the key.
    pub(crate) fn fingerprint(&self) -> KeyFingerprint {
        self.fingerprint
    }

    /// The last characters of the key, such as `a1f3`; `None` for a key
    /// too short to show any of it.
    pub(crate) fn ending(&self) -> Option<&str> {
        self.ending.as_deref()
    }

    /// What is kept of `key`; `None` when it is blank.
    pub(crate) fn of(key: &str) -> Option<Self> {
        let key = filled(Some(key))?;
        let length = key.chars().count();

        let ending = (length >= SHORTEST_SHOWN_KEY)
            .then(|| key.chars().skip(length - SHOWN_ENDING).collect());

        Some(Self {
            fingerprint: KeyFingerprint::of(key),
            ending,
        })
    }
}

/// Reads a key, which may be missing or `null`, as a [`SeenKey`]. The key
/// itself is dropped at once, so it is never kept.
pub(crate) fn read_seen_key<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SeenKey>, D::Error> {
    let key = Option::<String>::deserialize(deserializer)?;

    Ok(key.as_deref().and_then(SeenKey::of))
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

    #[test]
    fn a_seen_key_keeps_its_fingerprint_and_its_last_characters() {
        let seen = SeenKey::of(KEY).expect("the key is filled");

        assert_eq!(seen.fingerprint(), KeyFingerprint::of(KEY));
        assert_eq!(seen.ending(), Some("a1f3"));
    }

    #[test]
    fn a_short_key_shows_none_of_it_and_a_blank_one_is_none() {
        let short = SeenKey::of("sk-short").expect("the key is filled");

        assert_eq!(short.ending(), None);
        assert_eq!(SeenKey::of("   "), None);
    }

    #[test]
    fn no_key_is_in_the_debug_output() {
        let seen = SeenKey::of(KEY).expect("the key is filled");

        let debug = format!("{seen:?}");

        assert!(!debug.contains(KEY), "{debug}");
    }
}
