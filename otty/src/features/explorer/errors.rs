use thiserror::Error;

/// Errors emitted while parsing the configured editor command.
#[derive(Debug, Error)]
pub(crate) enum EditorCommandParseError {
    #[error("Invalid editor command: {0}")]
    Invalid(#[from] shell_words::ParseError),
    #[error("Editor command is empty.")]
    Empty,
}
