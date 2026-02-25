use thiserror::Error;

/// Errors emitted by quick launch editor validation and persistence.
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
