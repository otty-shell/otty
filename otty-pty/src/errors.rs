use std::io;

#[cfg(unix)]
use nix::errno::Errno;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[cfg(unix)]
    #[error("error from *nix bindings")]
    Nix(#[from] Errno),

    #[error("error from local pty I/O")]
    IO(#[from] io::Error),

    #[error("error from ssh2 lib")]
    SSH2(#[from] ssh2::Error),

    #[error("failed to resize pty")]
    Resize(io::Error),

    #[error("failed to parse ssh host")]
    HostParsing,
}
