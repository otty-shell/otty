//! Reads the small JSON files the agents keep, such as their login files.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

/// The file at `path` read as JSON. `None` when it is missing or broken.
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;

    serde_json::from_slice(&bytes).ok()
}

/// Reads a text, which may be missing or `null`, as whether it is there and
/// not empty. The text itself is dropped at once, so a key or a token read
/// this way is never kept.
pub(crate) fn is_filled_text<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<bool, D::Error> {
    let text = Option::<String>::deserialize(deserializer)?;

    Ok(text.is_some_and(|text| !text.is_empty()))
}
