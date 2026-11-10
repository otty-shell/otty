use otty_app_i18n::{self as i18n, Key};
use thiserror::Error;

/// Why a name typed for a launch or a folder can not be used in its folder.
#[derive(Debug, Error)]
pub(crate) enum TitleError {
    #[error("{}", i18n::t(Key::ErrTitleEmpty))]
    TitleEmpty,
    #[error("{}", i18n::t(Key::ErrTitleDuplicate))]
    TitleDuplicate,
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
