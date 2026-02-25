use thiserror::Error;

/// Errors emitted while working with quick launch data and storage.
#[derive(Debug, Error)]
pub(crate) enum QuickLaunchError {
    #[error("quick launches IO failed")]
    Io(#[from] std::io::Error),
    #[error("quick launches JSON failed")]
    Json(#[from] serde_json::Error),
    #[error("Title cannot be empty.")]
    TitleEmpty,
    #[error("Title already exists in this folder.")]
    TitleDuplicate,
    #[error("Validation failed: {message}")]
    Validation { message: String },
}
