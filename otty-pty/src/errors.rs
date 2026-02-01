use std::io;
use std::time::Duration;

#[cfg(unix)]
use nix::errno::Errno;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[cfg(unix)]
    #[error("error from *nix bindings")]
    Nix(#[from] Errno),

    #[error("pty I/O error: {0}")]
    IO(#[from] io::Error),

    #[error("ssh error: {0}")]
    SSH2(#[from] ssh2::Error),

    #[error("failed to resize pty: {0}")]
    Resize(io::Error),

    #[error("ssh host resolved to no addresses")]
    NoAddresses,

    #[error("session launch cancelled")]
    Cancelled,

    #[error("session timed out while {step} after {duration:?}")]
    Timeout {
        step: &'static str,
        duration: Duration,
    },

    #[error("internal error: {0}")]
    Internal(String),
}
