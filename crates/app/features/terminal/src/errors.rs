use thiserror::Error;

/// Errors emitted by the terminal workspace widget.
#[derive(Debug, Error)]
pub(crate) enum TerminalWorkspaceError {
    /// The terminal could not start; `message` is the reason it gave. The
    /// text is only the reason, so a notification can put it after its own
    /// words.
    #[error("{message}")]
    Init { message: String },
}
