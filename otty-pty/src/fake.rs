//! Unix PTY backend that launches local child processes and exposes them
//! through the shared `Session` abstraction.

use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};

use mio::Token;
use mio::unix::SourceFd;
use nix::libc;
use nix::pty::{Winsize, openpty};
use signal_hook::{
    SigId,
    low_level::{self, pipe},
};

use crate::{Pollable, PtySize, Session, SessionError};

/// Local pseudo terminal session that owns the spawned child process.
pub struct UnixSession {
    master: File,
    child: Child,
    signal_pipe: UnixStream,
    signal_pipe_id: SigId,
}

impl Session for UnixSession {
    /// Poll the child process for a new exit status without blocking the
    /// event loop thread.
    fn try_get_child_exit_status(
        &mut self,
    ) -> Result<Option<ExitStatus>, SessionError> {
        let mut tmp = [0u8; 1];
        match self.signal_pipe.read(&mut tmp) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(SessionError::IO(e)),
            _ => Ok(self.child.try_wait()?),
        }
    }

    /// Read bytes produced by the child process from the PTY master.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError> {
        loop {
            match self.master.read(buf) {
                Ok(n) => return Ok(n),
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                    continue;
                },
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    return Err(SessionError::IO(err));
                },
                Err(err) => return Err(SessionError::IO(err)),
            }
        }
    }

    /// Write bytes into the PTY master so the child process receives them on
    /// its stdin.
    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError> {
        loop {
            match self.master.write(input) {
                Ok(n) => return Ok(n),
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                    continue;
                },
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    return Err(SessionError::IO(err));
                },
                Err(err) => return Err(SessionError::IO(err)),
            }
        }
    }

    /// Resize the pseudo terminal to match the front-end viewport.
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

    /// Terminate the child process and report its exit code.
    fn close(&mut self) -> Result<i32, SessionError> {
        low_level::unregister(self.signal_pipe_id);

        if let Some(status) = self.child.try_wait()? {
            return Ok(status.code().unwrap_or_default());
        }

        // Try send SIGTERM and wait for graceful shutdown
        if let Ok(pid_raw) = i32::try_from(self.child.id()) {
            let result = unsafe { libc::kill(pid_raw, libc::SIGTERM) };
            if result == 0 {
                if let Some(status) = self.child.try_wait()? {
                    return Ok(status.code().unwrap_or_default());
                }
            } else {
                // Check that the process with targed pid is exists
                let err = io::Error::last_os_error();
                if err.raw_os_error() != Some(libc::ESRCH) {
                    return Err(SessionError::IO(err));
                }
            }
        }

        match self.child.kill() {
            Ok(()) => (),
            Err(err) if err.kind() == io::ErrorKind::InvalidInput => (),
            Err(err) => return Err(SessionError::IO(err)),
        }

        let status = self.child.wait()?;
        Ok(status.code().unwrap_or_default())
    }
}

impl Pollable for UnixSession {
    /// Register the PTY master and SIGCHLD notification pipe with Mio.
    fn register(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<(), SessionError> {
        let master_fd = self.master.as_raw_fd();
        let mut master_source = SourceFd(&master_fd);

        registry.register(&mut master_source, io_token, interest)?;

        let signal_pipe = self.signal_pipe.as_raw_fd();
        let mut signal_pipe_source = SourceFd(&signal_pipe);

        registry.register(
            &mut signal_pipe_source,
            child_token,
            mio::Interest::READABLE,
        )?;

        Ok(())
    }

    /// Update Mio's interest set for the PTY master and signal pipe.
    fn reregister(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<(), SessionError> {
        let master_fd = self.master.as_raw_fd();
        let mut master_source = SourceFd(&master_fd);

        registry.reregister(&mut master_source, io_token, interest)?;

        let signal_pipe = self.signal_pipe.as_raw_fd();
        let mut signal_pipe_source = SourceFd(&signal_pipe);

        registry.reregister(
            &mut signal_pipe_source,
            child_token,
            mio::Interest::READABLE,
        )?;

        Ok(())
    }

    /// Remove the tracked file descriptors from the Mio registry.
    fn deregister(
        &mut self,
        registry: &mio::Registry,
    ) -> Result<(), SessionError> {
        let master_fd = self.master.as_raw_fd();
        let mut master_source = SourceFd(&master_fd);
        registry.deregister(&mut master_source)?;

        let signal_pipe = self.signal_pipe.as_raw_fd();
        let mut signal_pipe_source = SourceFd(&signal_pipe);
        registry.deregister(&mut signal_pipe_source)?;

        Ok(())
    }
}

impl Drop for UnixSession {
    fn drop(&mut self) {
        let _ = self.close().expect("failed to close unix session");
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
            signal_pipe_id,
        }
    }
}

/// Builder for launching local commands attached to a pseudo terminal.
pub struct UnixSessionBuilder {
    cmd: Command,
    size: PtySize,
    work_dir: Option<PathBuf>,
    controlling_tty: bool,
}

/// Start building a Unix PTY session for the provided executable.
pub fn unix(program: &str) -> UnixSessionBuilder {
    UnixSessionBuilder {
        cmd: Command::new(program),
        size: PtySize::default(),
        work_dir: None,
        controlling_tty: false,
    }
}

impl UnixSessionBuilder {
    /// Append a single argument to the command line.
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.cmd.arg(arg);
        self
    }

    /// Append a list of arguments to the command line.
    pub fn with_args(mut self, args: &[String]) -> Self {
        for arg in args {
            self.cmd.arg(arg.as_str());
        }
        self
    }

    /// Set an environment variable for the spawned child process.
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.cmd.env(key, value);
        self
    }

    /// Remove an environment variable from the child process environment.
    pub fn with_env_remove(mut self, key: &str) -> Self {
        self.cmd.env_remove(key);
        self
    }

    /// Advertise the initial PTY size that should be used for the child
    /// process.
    pub fn with_size(mut self, size: PtySize) -> Self {
        self.size = size;
        self
    }

    /// Change the working directory of the spawned child process.
    pub fn with_cwd(mut self, path: &Path) -> Self {
        self.work_dir = Some(path.to_path_buf());
        self
    }

    /// Request that the PTY be installed as the controlling terminal for the
    /// new process.
    pub fn set_controling_tty_enable(mut self) -> Self {
        self.controlling_tty = true;
        self
    }

    /// Spawn the configured command and return an interactive PTY session that
    /// can be registered with Mio.
    pub fn spawn(mut self) -> Result<UnixSession, SessionError> {
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

        Ok(UnixSession::new(master, child, signal_pipe, signal_pipe_id))
    }
}

fn register_signal_handler() -> Result<(UnixStream, SigId), SessionError> {
    let (pipe_writer, pipe) = UnixStream::pair()?;
    let pipe_id = pipe::register(libc::SIGCHLD, pipe_writer)?;
    pipe.set_nonblocking(true)?;
    Ok((pipe, pipe_id))
}

fn set_nonblocking(raw_fd: i32) -> Result<(), SessionError> {
    unsafe {
        let flags = libc::fcntl(raw_fd, libc::F_GETFL, 0);
        let result =
            libc::fcntl(raw_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        if result != 0 {
            return Err(SessionError::IO(io::Error::last_os_error()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::ErrorKind;
    use std::thread;
    use std::time::Duration;

    use super::{Session, SessionError, unix};
    use nix::errno::Errno;

    fn read_output(session: &mut impl Session) -> Result<String, SessionError> {
        let mut buffer = [0u8; 1024];
        let mut collected = Vec::new();

        for _ in 0..100 {
            match session.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    collected.extend_from_slice(&buffer[..n]);
                    if n < buffer.len() {
                        break;
                    }
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::Interrupted =>
                {
                    continue;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::WouldBlock =>
                {
                    if !collected.is_empty() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                },
                Err(err) => return Err(err),
            }
        }

        Ok(String::from_utf8_lossy(&collected).into_owned())
    }

    fn write_input(
        session: &mut impl Session,
        data: &[u8],
    ) -> Result<(), SessionError> {
        let mut offset = 0;

        while offset < data.len() {
            match session.write(&data[offset..]) {
                Ok(0) => thread::sleep(Duration::from_millis(10)),
                Ok(n) => {
                    offset += n;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::Interrupted =>
                {
                    continue;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::WouldBlock =>
                {
                    thread::sleep(Duration::from_millis(10));
                },
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    #[test]
    fn unix_session_echoes_output() {
        let mut session = match unix("/bin/cat").spawn() {
            Ok(session) => session,
            Err(SessionError::Nix(Errno::EACCES)) => {
                eprintln!("skipping test; PTY allocation denied (EACCES)");
                return;
            },
            Err(err) => panic!("failed to spawn session: {err:?}"),
        };

        write_input(&mut session, b"otty-test\n")
            .expect("failed to send payload to child");

        let output = read_output(&mut session).expect("failed to read output");
        assert!(
            output.contains("otty-test"),
            "expected echoed output, got: {output:?}"
        );

        assert_eq!(session.close().expect("failed to close"), 0);
    }

    #[test]
    fn unix_session_respects_environment()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut session = match unix("/bin/sh")
            .with_arg("-c")
            .with_arg("printf '%s' \"$OTTY_ENV_TEST\"")
            .with_env("OTTY_ENV_TEST", "42")
            .spawn()
        {
            Ok(session) => session,
            Err(SessionError::Nix(Errno::EACCES)) => {
                eprintln!("skipping test; PTY allocation denied (EACCES)");
                return Ok(());
            },
            Err(err) => return Err(err.into()),
        };

        let output = read_output(&mut session)?;
        assert_eq!(output.trim(), "42");

        assert_eq!(session.close()?, 0);
        Ok(())
    }
}
