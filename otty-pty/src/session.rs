#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::{fs::File, process::ExitStatus};

use crate::{PtySize, SessionError};

pub trait Session: Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError>;

    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError>;

    fn resize(&mut self, size: PtySize) -> Result<(), SessionError>;

    fn close(&mut self) -> Result<i32, SessionError>;

    fn try_wait(&mut self) -> Result<i32, SessionError>;

    fn as_pollable(&mut self) -> Option<&mut dyn PollableSessionExt> { None }
}

// pub trait PollableSessionExt {
//     /// Зарегистрировать все источники в poller’е (I/O + уведомление о child-exit).
//     unsafe fn register(&mut self, registry: &mio::Registry, interest: mio::Interest, mode: mio::PollMode) -> std::io::Result<()>;
//     fn reregister(&mut self, poll: &polling::Poller, interest: polling::Event, mode: polling::PollMode) -> std::io::Result<()>;
//     fn deregister(&mut self, poll: &polling::Poller) -> std::io::Result<()>;

//     /// Неблокирующая попытка получить код выхода, если пришло «exit-событие».
//     fn get_next_exit(&mut self) -> Result<Option<std::process::ExitStatus>, SessionError>;
// }
