use std::fs::File;

use crate::{PtySize, SessionError};

pub trait Session: Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError>;

    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError>;

    fn resize(&mut self, size: PtySize) -> Result<(), SessionError>;

    fn close(&mut self) -> Result<i32, SessionError>;

    fn try_wait(&mut self) -> Result<i32, SessionError>;

    fn raw(&self) -> &File;
}
