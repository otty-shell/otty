use thiserror::Error;

/// Errors emitted by the terminal feature.
#[derive(Debug, Error)]
pub(crate) enum TerminalError {
    #[error("shell integration IO failed")]
    Io(#[from] std::io::Error),
    #[error("terminal init failed: {message}")]
    Init { message: String },
}
