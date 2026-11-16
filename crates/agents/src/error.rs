//! Why a source gave no answer.

/// Why a source gave no answer. Any of these moves on to the next source
/// (spec Decision 5).
///
/// The text of each case is short and meant for the user.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// The source can not be used here, for example its file or program is
    /// missing. The text says what is missing.
    #[error("not available: {0}")]
    NotAvailable(String),
    /// The login is missing or expired.
    #[error("not logged in")]
    NotLoggedIn,
    /// The server refused the key the source was given, such as an admin
    /// key that was revoked or is not an admin key.
    #[error("the key was refused")]
    KeyRefused,
    /// The server could not be reached. The text says why.
    #[error("network error: {0}")]
    Network(String),
    /// The answer is not what the code expects. The text says what was
    /// wrong.
    #[error("unexpected answer: {0}")]
    UnexpectedAnswer(String),
    /// A folder or file that exists could not be read, such as a folder
    /// without the permission to read it. The text names it and says why.
    #[error("can not read {0}")]
    Unreadable(String),
    /// The program the source started stopped before it answered, such as
    /// a program that crashed. The text says which one.
    #[error("stopped unexpectedly: {0}")]
    ProgramStopped(String),
}
