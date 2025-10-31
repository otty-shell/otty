use std::io::{Read, Write};
#[cfg(unix)]
use std::os::fd::RawFd;

use nix::libc::openpty;
use portable_pty::{Child, PtySystem, CommandBuilder, MasterPty, NativePtySystem};

use crate::{PtySize, SessionError};
use crate::session::Session;

pub struct LocalSession {
    master_pty: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
}

impl Session for LocalSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError> {
        Ok(self.reader.read(buf)?)
    }

    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError> {
        Ok(self.writer.write(input)?)
    }

    fn resize(&mut self, size: PtySize) -> Result<(), SessionError> {
        self.master_pty.resize(size.into())?;
        Ok(())
    }

    fn close(&mut self) -> Result<u32, SessionError> {
        if let Some(status) = self.child.try_wait()? {
            return Ok(status.exit_code());
        }

        let mut killer = self.child.clone_killer();
        killer.kill()?;

        let status = self.child.wait()?;
        Ok(status.exit_code())
    }

    fn try_wait(&mut self) -> Result<Option<u32>, SessionError> {
        Ok(self.child.try_wait()?.map(|status| status.exit_code()))
    }

    #[cfg(unix)]
    fn as_raw_fd(&self) -> Option<RawFd> {
        self.master_pty.as_raw_fd()
    }
}

pub struct LocalSessionBuilder {
    cmd: CommandBuilder,
    size: PtySize,
}

pub fn local(program: &str) -> LocalSessionBuilder {
    LocalSessionBuilder {
        cmd: CommandBuilder::new(program),
        size: PtySize::default(),
    }
}

impl LocalSessionBuilder {
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.cmd.arg(arg);
        self
    }

    pub fn with_args(mut self, args: &[String]) -> Self {
        for arg in args {
            self.cmd.arg(arg.as_str());
        }
        self
    }

    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.cmd.env(key, value);
        self
    }

    pub fn with_env_remove(mut self, key: &str) -> Self {
        self.cmd.env_remove(key);
        self
    }

    pub fn with_size(mut self, size: PtySize) -> Self {
        self.size = size;
        self
    }

    pub fn with_cwd<P>(mut self, path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        self.cmd.cwd(path.as_ref().as_os_str());
        self
    }

    pub fn spawn(self) -> Result<impl Session, SessionError> {
        let system = NativePtySystem::default();
        let pair = system.openpty(self.size.into())?;
        let child = pair.slave.spawn_command(self.cmd)?;

        drop(pair.slave);

        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        Ok(LocalSession {
            master_pty: pair.master,
            child,
            reader,
            writer,
        })
    }
}
