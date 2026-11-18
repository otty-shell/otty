//! Reads the small JSON files the agents keep, such as their login files.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

/// The file at `path` read as JSON. `None` when it is missing or broken.
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;

    serde_json::from_slice(&bytes).ok()
}

/// The text without the spaces around it, when it is there and not blank.
///
/// This is the one rule of the agents for whether a key, a token or a name
/// in their files is filled.
pub(crate) fn filled(text: Option<&str>) -> Option<&str> {
    let text = text?.trim();

    (!text.is_empty()).then_some(text)
}

/// Reads a text, which may be missing or `null`, as whether it is
/// [`filled`]. The text itself is dropped at once, so a key or a token read
/// this way is never kept.
pub(crate) fn is_filled_text<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<bool, D::Error> {
    let text = Option::<String>::deserialize(deserializer)?;

    Ok(filled(text.as_deref()).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filled_text_is_trimmed_and_a_blank_one_is_none() {
        assert_eq!(filled(Some(" sk-made-up ")), Some("sk-made-up"));
        assert_eq!(filled(Some("  ")), None);
        assert_eq!(filled(Some("")), None);
        assert_eq!(filled(None), None);
    }
}
