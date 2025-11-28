use std::io;
use std::num::ParseIntError;

use otty_libterm::Error as LibtermError;
use otty_libterm::pty::SessionError;
use thiserror::Error;

/// Errors originating from the `otty-iced` widget.
#[derive(Debug, Error)]
pub enum Error {
    #[error("otty-libterm error: {0}")]
    Backend(#[from] LibtermError),

    #[error("otty-pty error: {0}")]
    Session(#[from] SessionError),

    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("input color string is in non valid format")]
    InvalidColorString,

    #[error("parsing color string to rgb error: {0}")]
    ParsingColorString(#[from] ParseIntError),
}

/// Convenient result alias for fallible operations in this crate.
pub type Result<T> = std::result::Result<T, Error>;
