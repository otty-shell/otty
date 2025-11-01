use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use nix::libc;
use nix::pty::{Winsize, openpty};

use crate::session::Session;
use crate::{PtySize, SessionError};

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

    pub fn spawn(mut self) -> Result<impl Session, SessionError> {
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

        let child = self.cmd.spawn()?;

        unsafe {
            let flags = libc::fcntl(raw_master, libc::F_GETFL, 0);
            if flags == -1 {
                return Err(SessionError::IO(io::Error::last_os_error()));
            }

            if libc::fcntl(raw_master, libc::F_SETFL, flags | libc::O_NONBLOCK)
                == -1
            {
                return Err(SessionError::IO(io::Error::last_os_error()));
            }
        }

        Ok(UnixSession::new(master, child))
    }
}

pub struct UnixSession {
    master: File,
    child: Child,
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
        if let Some(status) = self.child.try_wait()? {
            return Ok(status.code().unwrap_or(-1));
        }

        self.child.kill()?;

        let status = self.child.wait()?;
        Ok(status.code().unwrap_or(-1))
    }

    fn try_wait(&mut self) -> Result<i32, SessionError> {
        let status = self.child.wait()?;
        Ok(status.code().unwrap_or(-1))
    }

    fn raw(&self) -> &File {
        &self.master
    }
}

impl UnixSession {
    fn new(master: File, child: Child) -> Self {
        Self { master, child }
    }
}
