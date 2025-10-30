use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("error from inner engine of a local pty")]
    InnerPty(#[from] anyhow::Error),

    #[error("local pty I/O error")]
    IO(#[from] io::Error),
}
