#[cfg(unix)]
use std::os::fd::RawFd;

use crate::{PtySize, SessionError};

pub trait Session: Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError>;

    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError>;

    fn resize(&mut self, size: PtySize) -> Result<(), SessionError>;

    fn close(&mut self) -> Result<u32, SessionError>;

    fn try_wait(&mut self) -> Result<Option<u32>, SessionError>;

    #[cfg(unix)]
    fn as_raw_fd(&self) -> Option<RawFd>;
}
