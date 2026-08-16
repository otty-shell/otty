use thiserror::Error;

use crate::i18n::{self, Key};

/// Errors emitted during quick launch operations.
#[derive(Debug, Error)]
pub(crate) enum QuickLaunchError {
    #[error("{}{}", i18n::t(Key::ErrIoPrefix), .0)]
    Io(#[from] std::io::Error),
    #[error("{}{}", i18n::t(Key::ErrJsonPrefix), .0)]
    Json(#[from] serde_json::Error),
    #[error("{}", i18n::t(Key::ErrTitleEmpty))]
    TitleEmpty,
    #[error("{}", i18n::t(Key::ErrTitleDuplicate))]
    TitleDuplicate,
    #[error("{message}")]
    Validation { message: String },
}

/// Errors emitted by quick launch wizard validation.
#[derive(Debug, Error)]
pub(crate) enum QuickLaunchWizardError {
    #[error("{}", i18n::t(Key::ErrTitleRequired))]
    TitleRequired,
    #[error("{}", i18n::t(Key::ErrProgramRequired))]
    ProgramRequired,
    #[error("{}", i18n::t(Key::ErrHostRequired))]
    HostRequired,
    #[error("{}", i18n::t(Key::ErrInvalidPort))]
    InvalidPort,
    #[error("{}", i18n::t(Key::ErrMissingCustomDraft))]
    MissingCustomDraft,
    #[error("{}", i18n::t(Key::ErrMissingSshDraft))]
    MissingSshDraft,
}

/// Build a human-readable error message for a failed launch.
pub(crate) fn quick_launch_error_message(
    command: &super::types::QuickLaunch,
    error: &dyn std::fmt::Display,
) -> String {
    i18n::launch_failed_body(command.title(), &error.to_string())
}
