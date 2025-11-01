use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};

use nix::libc;
use nix::pty::{Winsize, openpty};
use signal_hook::{
    low_level::{self, pipe},
    SigId
};

use crate::session::Session;
use crate::{PtySize, SessionError, UnixSessionExt};

pub struct UnixSessionBuilder {
    cmd: Command,
    size: PtySize,
    work_dir: Option<PathBuf>,
    controlling_tty: bool,
}

pub fn unix(program: &str) -> UnixSessionBuilder {
    UnixSessionBuilder {
        cmd: Command::new(program),
        size: PtySize::default(),
        work_dir: None,
        controlling_tty: false,
    }
}

impl UnixSessionBuilder {
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

    pub fn with_cwd(mut self, path: &Path) -> Self {
        self.work_dir = Some(path.to_path_buf());
        self
    }

    pub fn set_controling_tty_enable(mut self) -> Self {
        self.controlling_tty = true;
        self
    }

    pub fn spawn(mut self) -> Result<impl Session + UnixSessionExt, SessionError> {
        let result = openpty(Some(&self.size.into()), None)?;
        let master = unsafe { File::from_raw_fd(result.master.into_raw_fd()) };
        let slave = unsafe { File::from_raw_fd(result.slave.into_raw_fd()) };
        let raw_master = master.as_raw_fd();
        let raw_slave = slave.as_raw_fd();

        let work_dir = self.work_dir;

        unsafe {
            let stdin_slave = slave.try_clone()?;
            let stderr_slave = slave.try_clone()?;

            self.cmd
                .stdin(stdin_slave)
                .stderr(stderr_slave)
                .stdout(slave)
                .pre_exec(move || {
                    if libc::setsid() == -1 {
                        return Err(io::Error::last_os_error());
                    }

                    if let Some(dir) = &work_dir {
                        env::set_current_dir(dir)?;
                    }

                    if self.controlling_tty {
                        // Set the pty as the controlling terminal.
                        // Failure to do this means that delivery of
                        // SIGWINCH won't happen when we resize the
                        // terminal, among other undesirable effects.
                        if libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                            return Err(io::Error::last_os_error());
                        }
                    }

                    for signo in &[
                        libc::SIGCHLD,
                        libc::SIGHUP,
                        libc::SIGINT,
                        libc::SIGQUIT,
                        libc::SIGTERM,
                        libc::SIGALRM,
                    ] {
                        libc::signal(*signo, libc::SIG_DFL);
                    }

                    libc::close(raw_master);
                    libc::close(raw_slave);

                    Ok(())
                });
        };

        let (signal_pipe, signal_pipe_id) = register_signal_handler()?;

        let child = self.cmd.spawn()?;

        set_nonblocking(raw_master)?;

        Ok(UnixSession::new(
            master,
            child,
            signal_pipe,
            signal_pipe_id,
        ))
    }
}

fn register_signal_handler() -> Result<(UnixStream, SigId), SessionError>{
    let (pipe_writer, pipe) = UnixStream::pair()?;
    let pipe_id = pipe::register(libc::SIGCHLD, pipe_writer)?;
    pipe.set_nonblocking(true)?;
    Ok((pipe, pipe_id))
}

fn set_nonblocking(raw_fd: i32) -> Result<(), SessionError> {
    unsafe {
        let flags = libc::fcntl(raw_fd, libc::F_GETFL, 0);
        let result = libc::fcntl(raw_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        if result != 0 {
            return Err(SessionError::IO(io::Error::last_os_error()));
        }

        Ok(())
    }
}

pub struct UnixSession {
    master: File,
    child: Child,
    signal_pipe: UnixStream,
    signal_pipe_id: SigId,
}

impl Session for UnixSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError> {
        Ok(self.master.read(buf)?)
    }

    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError> {
        Ok(self.master.write(input)?)
    }

    fn resize(&mut self, size: PtySize) -> Result<(), SessionError> {
        let size: Winsize = size.into();
        let res = unsafe {
            libc::ioctl(
                self.master.as_raw_fd(),
                libc::TIOCSWINSZ,
                &size as *const _,
            )
        };

        if res < 0 {
            return Err(SessionError::Resize(io::Error::last_os_error()));
        }

        Ok(())
    }

    fn close(&mut self) -> Result<i32, SessionError> {
        low_level::unregister(self.signal_pipe_id);

        self.child.kill()?;

        let status = self.child.wait()?;
        Ok(status.code().unwrap_or(-1))
    }

    fn try_wait(&mut self) -> Result<i32, SessionError> {
        let status = self.child.wait()?;
        Ok(status.code().unwrap_or(-1))
    }
}

impl UnixSessionExt for UnixSession {
    fn master_fd(&self) -> &File {
        &self.master
    }

    fn signal_pipe(&self) -> &UnixStream {
        &self.signal_pipe
    }
    
    fn get_next_exit(&mut self) -> Result<Option<ExitStatus>, SessionError> {
        let mut tmp = [0u8; 1];
        match self.signal_pipe.read(&mut tmp) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(e) => return Err(SessionError::IO(e)),
            _ => {}
        }

        let status = self.child.try_wait()?;
        Ok(status)
    }
}

impl UnixSession {
    fn new(
        master: File,
        child: Child,
        signal_pipe: UnixStream,
        signal_pipe_id: SigId,
    ) -> Self {
        Self {
            master,
            child,
            signal_pipe,
            signal_pipe_id
        }
    }
}
