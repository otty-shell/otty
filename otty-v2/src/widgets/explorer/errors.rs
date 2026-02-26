use thiserror::Error;

/// Errors emitted by explorer services and command parsing.
#[derive(Debug, Error)]
pub(crate) enum ExplorerError {
    #[error("Explorer I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid editor command: {0}")]
    InvalidEditorCommand(#[from] shell_words::ParseError),
    #[error("Editor command is empty.")]
    EmptyEditorCommand,
}
