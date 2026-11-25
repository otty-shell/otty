//! The text of a key, read from its place for one use and dropped right
//! after (spec Decision 5). It is never shown, logged or given to a
//! plugin: only what a plugin may learn of it is.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD_INDIFFERENT;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A key read from its place, without the spaces around it. Printing it
/// shows nothing of the key.
#[derive(Clone)]
pub struct KeyText {
    text: String,
}

impl KeyText {
    /// The key itself, for the one header it is sent in.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The key `text` without the spaces around it; `None` when it is
    /// blank.
    ///
    /// ```
    /// use otty_plugin_host::KeyText;
    ///
    /// let key = KeyText::new("  ex-made-up-key\n").expect("a filled key");
    ///
    /// assert_eq!(key.as_str(), "ex-made-up-key");
    /// assert!(KeyText::new(" ").is_none());
    /// ```
    pub fn new(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }

        Some(Self {
            text: text.to_string(),
        })
    }

    /// SHA-256 of the key, lowercase hex: the fingerprint OTTY joins
    /// accounts by (`KeyFingerprint` of `otty-agents`).
    pub fn fingerprint(&self) -> String {
        Sha256::digest(self.text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// Whether the key starts with `prefix`, such as `sk-ant-admin`.
    pub(crate) fn has_prefix(&self, prefix: &str) -> bool {
        self.text.starts_with(prefix)
    }

    /// The claims of a JWT, its middle part decoded, as JSON text; `None`
    /// when the key is not a JWT. Never the signature or the token.
    pub(crate) fn jwt_claims(&self) -> Option<String> {
        let mut parts = self.text.split('.');
        let (Some(_), Some(payload), Some(_), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };

        let bytes = URL_SAFE_NO_PAD_INDIFFERENT.decode(payload).ok()?;
        let claims: Value = serde_json::from_slice(&bytes).ok()?;

        claims.is_object().then(|| claims.to_string())
    }
}

impl fmt::Debug for KeyText {
    /// Never the key.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("KeyText(hidden)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up JWT: `{"alg":"none"}`, the claims below, a made-up
    /// signature.
    const JWT: &str = "eyJhbGciOiJub25lIn0.eyJlbWFpbCI6ImRldkBleGFtcGxlLmNvbSIsImV4cCI6MTkwMDAwMDAwMH0.made-up-signature";

    fn key(text: &str) -> KeyText {
        KeyText::new(text).expect("a filled key")
    }

    /// The fingerprints of `otty-agents` (`KeyFingerprint::of`): SHA-256
    /// of the key without the spaces around it, the known hashes of "a"
    /// and of a made-up key.
    #[test]
    fn the_fingerprint_is_the_one_otty_agents_gives() {
        assert_eq!(
            key(" a\n").fingerprint(),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
        assert_eq!(
            key("ex-made-up-key").fingerprint(),
            "27c8d30ec451e808415d2307396308459e728ccef0adc5a41908fccae8a2e1ae"
        );
    }

    #[test]
    fn a_blank_key_is_no_key() {
        assert!(KeyText::new("  \n").is_none());
        assert_eq!(key("  sk-made-up\n").as_str(), "sk-made-up");
    }

    #[test]
    fn a_prefix_is_checked_on_the_key_without_spaces() {
        let admin = key("  sk-ant-admin01-made-up");

        assert!(admin.has_prefix("sk-ant-admin"));
        assert!(!admin.has_prefix("sk-ant-api"));
    }

    #[test]
    fn the_claims_of_a_jwt_are_its_middle_part_only() {
        let claims = key(JWT).jwt_claims().expect("a JWT");

        assert_eq!(claims, r#"{"email":"dev@example.com","exp":1900000000}"#);
        assert!(!claims.contains("made-up-signature"));
        assert!(!claims.contains("eyJ"));
    }

    #[test]
    fn a_key_that_is_not_a_jwt_has_no_claims() {
        assert_eq!(key("sk-made-up-key").jwt_claims(), None);
        assert_eq!(key("a.b.c").jwt_claims(), None);
        assert_eq!(key("a.eyJ4IjoxfQ.c.d").jwt_claims(), None);
        // A middle part that is JSON but not an object.
        assert_eq!(key("a.WzFd.c").jwt_claims(), None);
    }

    #[test]
    fn the_debug_output_holds_no_key() {
        let debug = format!("{:?}", key("sk-made-up-key"));

        assert!(!debug.contains("sk-made-up-key"), "{debug}");
    }
}
