use thiserror::Error;

/// Errors emitted by the terminal workspace widget.
#[derive(Debug, Error)]
pub(crate) enum TerminalWorkspaceError {
    /// Shell integration I/O failed.
    #[error("shell integration IO failed")]
    Io(#[from] std::io::Error),

    /// Terminal initialisation failed.
    #[error("terminal init failed: {message}")]
    Init { message: String },
}
