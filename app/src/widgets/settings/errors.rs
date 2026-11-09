use thiserror::Error;

/// Errors emitted while reading or writing settings.
#[derive(Debug, Error)]
pub(crate) enum SettingsError {
    /// Filesystem operation failed.
    #[error("settings IO failed: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization or deserialization failed.
    #[error("settings JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}
