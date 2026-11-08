//! Why the generator did not create a widget.

use std::path::PathBuf;
use std::{error, fmt, io};

/// Why a widget was not created.
///
/// Every error except [`NewWidgetError::Write`] comes before the first
/// write, so nothing in `app/src` has changed.
#[derive(Debug)]
pub enum NewWidgetError {
    /// The name is not snake_case ASCII.
    NotSnakeCase(String),
    /// The name is a Rust keyword, which cannot name a module.
    Keyword(String),
    /// Something with the widget's name exists already, e.g. its folder.
    NameTaken(String),
    /// An existing file lacks the line the generator puts its code by.
    AnchorNotFound {
        /// The file, relative to `app/src`.
        file: &'static str,
        /// What the generator looked for.
        anchor: String,
    },
    /// A file could not be read.
    Read {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        source: io::Error,
    },
    /// A file could not be written; the files written before it stay.
    Write {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        source: io::Error,
    },
}

impl fmt::Display for NewWidgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NewWidgetError::NotSnakeCase(name) => write!(
                f,
                "`{name}` is not snake_case: use lowercase ASCII letters, \
                 digits and single underscores, starting with a letter"
            ),
            NewWidgetError::Keyword(name) => {
                write!(f, "`{name}` is a Rust keyword")
            },
            NewWidgetError::NameTaken(what) => {
                write!(f, "{what} already exists")
            },
            NewWidgetError::AnchorNotFound { file, anchor } => {
                write!(f, "{file}: cannot find {anchor}; nothing was changed")
            },
            NewWidgetError::Read { path, source } => {
                let path = path.display();
                write!(f, "cannot read {path}: {source}; nothing was changed")
            },
            NewWidgetError::Write { path, source } => {
                let path = path.display();
                write!(f, "cannot write {path}: {source}")
            },
        }
    }
}

impl error::Error for NewWidgetError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            NewWidgetError::Read { source, .. }
            | NewWidgetError::Write { source, .. } => Some(source),
            _ => None,
        }
    }
}
