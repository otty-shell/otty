use thiserror::Error;

/// Errors emitted by the terminal workspace widget.
#[derive(Debug, Error)]
pub(crate) enum TerminalWorkspaceError {
    /// Terminal initialisation failed.
    #[error("terminal init failed: {message}")]
    Init { message: String },
}
