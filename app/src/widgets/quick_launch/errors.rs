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
    /// A custom command has no program.
    #[error("{}", i18n::t(Key::ErrProgramRequired))]
    ProgramRequired,
    /// An SSH command has no host.
    #[error("{}", i18n::t(Key::ErrHostRequired))]
    HostRequired,
    /// An SSH command uses port zero.
    #[error("{}", i18n::t(Key::ErrSshPortPositive))]
    SshPortNotPositive,
    /// An SSH connection attempt failed.
    #[error("{}", i18n::ssh_connection_failed(.0))]
    SshConnectionFailed(String),
    /// A configured working directory does not exist.
    #[error("{}", i18n::working_directory_not_found(.0))]
    WorkingDirectoryNotFound(String),
    /// A configured working path is not a directory.
    #[error("{}", i18n::working_directory_not_directory(.0))]
    WorkingDirectoryNotDirectory(String),
    /// A configured SSH identity file does not exist.
    #[error("{}", i18n::identity_file_not_found(.0))]
    IdentityFileNotFound(String),
    /// A configured SSH identity path is not a file.
    #[error("{}", i18n::identity_file_not_file(.0))]
    IdentityFileNotFile(String),
    /// A program name could not be resolved through `PATH`.
    #[error("{}", i18n::program_not_found_in_path(.0))]
    ProgramNotFoundInPath(String),
    /// An explicit program path does not exist.
    #[error("{}", i18n::program_not_found(.0))]
    ProgramNotFound(String),
    /// An explicit program path points to a directory.
    #[error("{}", i18n::program_is_directory(.0))]
    ProgramIsDirectory(String),
    /// An explicit program path is not executable.
    #[error("{}", i18n::program_not_executable(.0))]
    ProgramNotExecutable(String),
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
