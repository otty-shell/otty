//! Reads the small JSON files the agents keep, such as their login files.

use std::path::Path;

use serde::de::DeserializeOwned;

/// The file at `path` read as JSON. `None` when it is missing or broken.
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;

    serde_json::from_slice(&bytes).ok()
}
