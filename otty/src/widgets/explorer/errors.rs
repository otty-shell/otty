use thiserror::Error;

/// Errors emitted by explorer directory operations.
#[derive(Debug, Error)]
pub(crate) enum ExplorerError {
    #[error("Explorer I/O failed: {0}")]
    Io(#[from] std::io::Error),
}
