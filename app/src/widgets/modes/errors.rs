use thiserror::Error;

/// Errors emitted while reading or writing `modes.json`.
#[derive(Debug, Error)]
pub(crate) enum ModesError {
    /// Filesystem operation failed.
    #[error("modes file IO failed: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parsing or serialization failed.
    #[error("modes file JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    /// The JSON has no `pinned` list of mode names.
    #[error("modes file has no \"pinned\" list of mode names")]
    NoPinnedList,
}
