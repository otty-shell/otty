use thiserror::Error;

/// Errors emitted during quick launch operations.
#[derive(Debug, Error)]
pub(crate) enum QuickLaunchError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Title must not be empty.")]
    TitleEmpty,
    #[error("A sibling with this title already exists.")]
    TitleDuplicate,
    #[error("{message}")]
    Validation { message: String },
}

/// Errors emitted by quick launch wizard validation.
#[derive(Debug, Error)]
pub(crate) enum QuickLaunchWizardError {
    #[error("Title is required.")]
    TitleRequired,
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
}

/// Build a human-readable error message for a failed launch.
pub(crate) fn quick_launch_error_message(
    command: &super::model::QuickLaunch,
    error: &dyn std::fmt::Display,
) -> String {
    format!("Command: {}\nError: {error}", command.title(),)
}
