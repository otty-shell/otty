use std::io::ErrorKind;
use std::process::ExitStatus;
use std::time::Duration;

use mio::{Events, Interest, Poll, Token};
use otty_escape::{Actor, Parser};
use otty_pty::{Pollable, PtySize, Session, SessionError};
use otty_surface::{Surface, SurfaceConfig};

use crate::TerminalMode;
use crate::error::{LibTermError, Result};
use crate::options::TerminalOptions;

const PTY_IO_TOKEN: Token = Token(0);
const PTY_CHILD_TOKEN: Token = Token(1);

pub trait PtySession: Session + Pollable {}
impl<T> PtySession for T where T: Session + Pollable {}

/// Trait describing the surface actor used by the terminal runtime.
pub trait TerminalSurface: Actor + Send {
    /// Construct a new surface instance using the provided configuration.
    fn create(config: SurfaceConfig) -> Self
    where
        Self: Sized;

    /// Resize the backing surface to match the new geometry.
    fn resize(&mut self, columns: usize, rows: usize);

    /// Borrow the inner [`Surface`] for read-only access.
    fn surface(&self) -> &Surface;

    /// Borrow the inner [`Surface`] mutably.
    fn surface_mut(&mut self) -> &mut Surface;
}

impl TerminalSurface for Surface {
    fn create(config: SurfaceConfig) -> Self {
        Surface::new(config)
    }

    fn resize(&mut self, columns: usize, rows: usize) {
        Surface::resize(self, columns, rows);
    }

    fn surface(&self) -> &Surface {
        self
    }

    fn surface_mut(&mut self) -> &mut Surface {
        self
    }
}

/// High level runtime that connects a PTY session with the escape parser and
/// in-memory surface model.
pub struct Terminal<S: TerminalSurface> {
    session: Box<dyn PtySession>,
    poll: Poll,
    events: Events,
    parser: Parser,
    surface: S,
    read_buffer: Vec<u8>,
    options: TerminalOptions,
    exit_status: Option<ExitStatus>,
    running: bool,
    mode: TerminalMode,
}

impl<S: TerminalSurface> Terminal<S> {
    /// Construct a terminal runtime from an arbitrary PTY session.
    pub fn with_session<T>(
        session: T,
        surface_config: SurfaceConfig,
        options: TerminalOptions,
    ) -> Result<Self>
    where
        T: PtySession + 'static,
    {
        let surface = S::create(surface_config);
        Self::with_session_and_surface(session, surface, options)
    }

    /// Construct a terminal runtime reusing a pre-configured surface actor.
    pub fn with_session_and_surface<T>(
        session: T,
        surface: S,
        options: TerminalOptions,
    ) -> Result<Self>
    where
        T: PtySession + 'static,
    {
        Self::from_boxed_session(Box::new(session), surface, options)
    }

    fn from_boxed_session(
        mut session: Box<dyn PtySession>,
        surface: S,
        options: TerminalOptions,
    ) -> Result<Self> {
        let poll = Poll::new()?;
        session.register(
            poll.registry(),
            Interest::READABLE,
            PTY_IO_TOKEN,
            PTY_CHILD_TOKEN,
        )?;

        let events = Events::with_capacity(128);
        let parser = Parser::new();
        let mut read_buffer = vec![0u8; options.read_buffer_capacity.max(1024)];

        if read_buffer.is_empty() {
            read_buffer.resize(1024, 0);
        }

        Ok(Self {
            session,
            poll,
            events,
            parser,
            surface,
            read_buffer,
            options,
            exit_status: None,
            running: true,
            mode: TerminalMode::default(),
        })
    }

    /// Drive one iteration of the mio poll loop using the configured timeout.
    pub fn poll_once(&mut self) -> Result<PollOutcome> {
        self.poll_once_with_timeout(Some(self.options.poll_timeout))
    }

    /// Drive one iteration of the mio poll loop with a custom timeout.
    pub fn poll_once_with_timeout(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<PollOutcome> {
        let mut outcome = PollOutcome::default();

        loop {
            match self.poll.poll(&mut self.events, timeout) {
                Ok(()) => break,
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => return Err(LibTermError::Poll(err)),
            }
        }

        let events: Vec<_> = self.events.iter().cloned().collect();

        for event in events {
            if event.token() == PTY_IO_TOKEN && event.is_readable() {
                outcome.surface_changed |= self.drain_pty()?;
            }

            if event.token() == PTY_CHILD_TOKEN {
                if let Some(status) = self.capture_exit()? {
                    outcome.exit_status.get_or_insert(status);
                }
            }
        }

        if let Some(status) = self.capture_exit()? {
            outcome.exit_status.get_or_insert(status);
        }

        if outcome.exit_status.is_some() {
            self.running = false;
        }

        Ok(outcome)
    }

    /// Run the terminal event loop, delegating front-end duties to the provided client.
    pub fn run<C>(&mut self, client: &mut C) -> Result<()>
    where
        C: TerminalClient<S> + ?Sized,
    {
        let mut exit_notified = false;

        while self.running {
            client.before_poll(self)?;
            if !self.running {
                break;
            }

            let outcome = self.poll_once()?;

            if outcome.surface_changed {
                client.on_surface_change(self.surface())?;
            }

            if let Some(status) = outcome.exit_status {
                client.on_child_exit(&status)?;
                exit_notified = true;
                break;
            }

            client.after_poll(self)?;
        }

        if !exit_notified {
            if let Some(status) = self.exit_status {
                client.on_child_exit(&status)?;
            }
        }

        Ok(())
    }

    /// Write a chunk of bytes to the PTY session.
    pub fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        let mut written = 0usize;

        while written < bytes.len() {
            match self.session.write(&bytes[written..]) {
                Ok(0) => break,
                Ok(count) => written += count,
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::Interrupted =>
                {
                    continue;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::WouldBlock =>
                {
                    break;
                },
                Err(err) => return Err(err.into()),
            }
        }

        Ok(written)
    }

    /// Request a PTY resize and mirror the new geometry in the surface model.
    pub fn resize(&mut self, size: PtySize) -> Result<()> {
        self.session.resize(size)?;
        self.surface.resize(size.cols as usize, size.rows as usize);
        Ok(())
    }

    /// Terminate the session and return the reported exit status code.
    pub fn close(&mut self) -> Result<i32> {
        let code = self.session.close()?;
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            self.exit_status = Some(ExitStatusExt::from_raw(code));
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            self.exit_status = Some(ExitStatusExt::from_raw(code as u32));
        }
        self.running = false;
        Ok(code)
    }

    /// Access the active surface for inspection or rendering.
    pub fn surface(&self) -> &Surface {
        self.surface.surface()
    }

    /// Mutably access the surface.
    pub fn surface_mut(&mut self) -> &mut Surface {
        self.surface.surface_mut()
    }

    /// Borrow the underlying surface actor.
    pub fn surface_actor(&self) -> &S {
        &self.surface
    }

    /// Mutably borrow the underlying surface actor.
    pub fn surface_actor_mut(&mut self) -> &mut S {
        &mut self.surface
    }

    /// Check whether the child process is still running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Retrieve the cached exit status if the child process has terminated.
    pub fn exit_status(&self) -> Option<&ExitStatus> {
        self.exit_status.as_ref()
    }

    fn drain_pty(&mut self) -> Result<bool> {
        let mut updated = false;

        loop {
            match self.session.read(self.read_buffer.as_mut_slice()) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = &self.read_buffer[..count];
                    self.parser.advance(chunk, &mut self.surface);
                    updated = true;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::Interrupted =>
                {
                    continue;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::WouldBlock =>
                {
                    break;
                },
                Err(err) => return Err(err.into()),
            }
        }

        Ok(updated)
    }

    fn capture_exit(&mut self) -> Result<Option<ExitStatus>> {
        match self.session.try_get_child_exit_status() {
            Ok(Some(status)) => {
                self.exit_status = Some(status);
                Ok(self.exit_status)
            },
            Ok(None) => Ok(None),
            Err(SessionError::IO(err))
                if err.kind() == ErrorKind::WouldBlock =>
            {
                Ok(None)
            },
            Err(SessionError::IO(err))
                if err.kind() == ErrorKind::Interrupted =>
            {
                Ok(None)
            },
            Err(err) => Err(err.into()),
        }
    }
}

/// Outcome from a single poll iteration.
#[derive(Clone, Debug, Default)]
pub struct PollOutcome {
    pub surface_changed: bool,
    pub exit_status: Option<ExitStatus>,
}

/// Callback interface for driving the runtime from a front-end.
pub trait TerminalClient<S: TerminalSurface> {
    /// Executed before the runtime blocks on `mio::Poll`.
    fn before_poll(&mut self, _terminal: &mut Terminal<S>) -> Result<()> {
        Ok(())
    }

    /// Executed after the runtime finishes handling poll events.
    fn after_poll(&mut self, _terminal: &mut Terminal<S>) -> Result<()> {
        Ok(())
    }

    /// Called when PTY output mutates the in-memory surface.
    fn on_surface_change(&mut self, _surface: &Surface) -> Result<()> {
        Ok(())
    }

    /// Called when the child process exits or is terminated.
    fn on_child_exit(&mut self, _status: &ExitStatus) -> Result<()> {
        Ok(())
    }
}
