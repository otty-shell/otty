use std::io;

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

    #[error("failed to parse ssh host")]
    HostParsing,
}
