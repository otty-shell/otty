use std::io;

use thiserror::Error;

use otty_pty::SessionError;

/// Errors originating from the `otty-libterm` runtime.
#[derive(Debug, Error)]
pub enum Error {
    #[error("pty session error: {0}")]
    Session(#[from] SessionError),

    #[error("poll error: {0}")]
    Poll(io::Error),

    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("failed to wake event loop: {0}")]
    Wake(io::Error),

    #[error("runtime command channel closed")]
    RuntimeChannelClosed,
}

/// Convenient result alias for fallible operations in this crate.
pub type Result<T> = std::result::Result<T, Error>;
