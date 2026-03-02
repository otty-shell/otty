use thiserror::Error;

/// Errors emitted while reading, writing, or validating settings.
#[derive(Debug, Error)]
pub(crate) enum SettingsError {
    /// Filesystem operation failed.
    #[error("settings IO failed")]
    Io(#[from] std::io::Error),
    /// JSON serialization or deserialization failed.
    #[error("settings JSON failed")]
    Json(#[from] serde_json::Error),
    /// A field value did not pass validation.
    #[error("validation error: {message}")]
    Validation { message: String },
}
