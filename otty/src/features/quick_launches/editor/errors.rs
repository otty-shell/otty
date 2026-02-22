use thiserror::Error;

/// Errors emitted by quick launch editor validation and persistence.
#[derive(Debug, Error)]
pub(crate) enum QuickLaunchEditorError {
    #[error("Title is required.")]
    TitleRequired,
    #[error("Title already exists in this folder.")]
    TitleDuplicate,
    #[error("Program is required.")]
    ProgramRequired,
    #[error("Host is required.")]
    HostRequired,
    #[error("Port must be a number.")]
    InvalidPort,
    #[error("Custom command draft is missing.")]
    MissingCustomDraft,
    #[error("SSH command draft is missing.")]
    MissingSshDraft,
    #[error("Missing target folder.")]
    MissingTargetFolder,
    #[error("Missing parent folder.")]
    MissingParentFolder,
    #[error("Command no longer exists.")]
    MissingCommand,
    #[error("Validation failed: {message}")]
    Validation { message: String },
}
