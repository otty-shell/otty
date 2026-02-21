use thiserror::Error;

/// Errors emitted while setting up shell integration files.
#[derive(Debug, Error)]
pub(crate) enum ShellError {
    #[error("shell integration IO failed")]
    Io(#[from] std::io::Error),
}

/// Errors emitted while initializing terminal widgets.
#[derive(Debug, Error)]
pub(crate) enum TerminalError {
    #[error("terminal init failed: {message}")]
    Init { message: String },
}
