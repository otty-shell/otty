use thiserror::Error;

/// Errors emitted while reading or writing settings.
#[derive(Debug, Error)]
pub(crate) enum SettingsError {
    #[error("settings IO failed")]
    Io(#[from] std::io::Error),
    #[error("settings JSON failed")]
    Json(#[from] serde_json::Error),
}
