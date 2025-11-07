use std::io::ErrorKind;
use std::process::ExitStatus;
use std::time::Duration;

use mio::{Events, Interest, Poll, Token};

use crate::error::{LibTermError, Result};

pub const PTY_IO_TOKEN: Token = Token(0);
pub const PTY_CHILD_TOKEN: Token = Token(1);
const DEFAULT_EVENT_CAPACITY: usize = 128;

/// Interface implemented by terminal backends that can be driven by [`TerminalEventLoop`].
pub trait TerminalLoopTarget {
    /// Handle provided to front-ends when the surface changes.
    type SurfaceHandle;

    /// Register any pollable resources with the supplied registry.
    fn register_session(
        &mut self,
        registry: &mio::Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()>;

    /// Update the registration for the session's resources.
    fn reregister_session(
        &mut self,
        registry: &mio::Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()>;

    /// Remove the registered resources from the poller.
    fn deregister_session(&mut self, registry: &mio::Registry) -> Result<()>;

    /// Drain PTY output and update the surface state.
    fn handle_read_ready(&mut self) -> Result<bool>;

    /// Check whether the child process has exited.
    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>>;

    /// Poll timeout used for the main event loop.
    fn poll_timeout(&self) -> Option<Duration>;

    /// Whether the terminal session is still running.
    fn is_running(&self) -> bool;

    /// Borrow the underlying surface handle.
    fn surface_handle(&self) -> &Self::SurfaceHandle;

    /// Retrieve the cached exit status, if one is available.
    fn exit_status(&self) -> Option<&ExitStatus>;

    /// Initial interest set for the session registration.
    fn initial_interest(&self) -> Interest {
        Interest::READABLE
    }

    /// Desired interest set after each poll iteration.
    fn desired_interest(&self) -> Interest {
        Interest::READABLE
    }
}

/// Callback interface for driving terminal backends from a front-end.
pub trait TerminalClient<T: TerminalLoopTarget + ?Sized> {
    /// Executed before the runtime blocks on `mio::Poll`.
    fn before_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }

    /// Executed after the runtime finishes handling poll events.
    fn after_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }

    /// Called when PTY output mutates the in-memory surface.
    fn on_surface_change(&mut self, _surface: &T::SurfaceHandle) -> Result<()> {
        Ok(())
    }

    /// Called when the child process exits or is terminated.
    fn on_child_exit(&mut self, _status: &ExitStatus) -> Result<()> {
        Ok(())
    }
}

/// Mio-backed driver that pumps PTY and child-process events for a terminal runtime.
pub struct TerminalEventLoop {
    poll: Poll,
    events: Events,
}

impl TerminalEventLoop {
    /// Construct a new event loop with the default capacity.
    pub fn new() -> Result<Self> {
        Self::with_capacity(DEFAULT_EVENT_CAPACITY)
    }

    /// Construct a new event loop with a custom event capacity.
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            events: Events::with_capacity(capacity.max(16)),
        })
    }

    /// Run the event loop, delegating front-end duties to the provided client.
    pub fn run<T, C>(&mut self, terminal: &mut T, client: &mut C) -> Result<()>
    where
        T: TerminalLoopTarget + ?Sized,
        C: TerminalClient<T> + ?Sized,
    {
        let mut interest = terminal.initial_interest();
        terminal.register_session(
            self.poll.registry(),
            interest,
            PTY_IO_TOKEN,
            PTY_CHILD_TOKEN,
        )?;

        let mut exit_notified = false;

        let run_result = (|| -> Result<()> {
            while terminal.is_running() {
                client.before_poll(terminal)?;
                if !terminal.is_running() {
                    break;
                }

                self.poll_once(terminal)?;

                let mut surface_changed = false;
                let mut exit_status: Option<ExitStatus> = None;

                for event in self.events.iter() {
                    if event.token() == PTY_IO_TOKEN && event.is_readable() {
                        surface_changed |= terminal.handle_read_ready()?;
                    }

                    if event.token() == PTY_CHILD_TOKEN {
                        if let Some(status) = terminal.check_child_exit()? {
                            exit_status.get_or_insert(status);
                        }
                    }
                }

                if exit_status.is_none() {
                    if let Some(status) = terminal.check_child_exit()? {
                        exit_status = Some(status);
                    }
                }

                if surface_changed {
                    client.on_surface_change(terminal.surface_handle())?;
                }

                if let Some(status) = exit_status {
                    client.on_child_exit(&status)?;
                    exit_notified = true;
                    break;
                }

                client.after_poll(terminal)?;

                let desired_interest = terminal.desired_interest();
                if desired_interest != interest {
                    terminal.reregister_session(
                        self.poll.registry(),
                        desired_interest,
                        PTY_IO_TOKEN,
                        PTY_CHILD_TOKEN,
                    )?;
                    interest = desired_interest;
                }
            }

            Ok(())
        })();

        let deregister_result =
            terminal.deregister_session(self.poll.registry());

        if !exit_notified {
            if let Some(status) = terminal.exit_status() {
                client.on_child_exit(status)?;
            }
        }

        run_result?;
        deregister_result?;

        Ok(())
    }

    fn poll_once<T>(&mut self, terminal: &T) -> Result<()>
    where
        T: TerminalLoopTarget + ?Sized,
    {
        self.events.clear();
        loop {
            match self.poll.poll(&mut self.events, terminal.poll_timeout()) {
                Ok(()) => break,
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => return Err(LibTermError::Poll(err)),
            }
        }

        Ok(())
    }
}
