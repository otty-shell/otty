use std::io::ErrorKind;
use std::process::ExitStatus;
use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender, TryRecvError},
};
use std::time::{Duration, Instant};

use mio::{Events, Interest, Poll, Registry, Token, Waker};

use crate::TerminalRequest;
use crate::error::{Error, Result};

const PTY_IO_TOKEN: Token = Token(0);
const PTY_CHILD_TOKEN: Token = Token(1);
const RUNTIME_WAKE_TOKEN: Token = Token(2);
const DEFAULT_EVENT_CAPACITY: usize = 128;

/// Events sent from the runtime driver into terminal implementations.
pub enum RuntimeEvent<'a> {
    /// Initial registration of the underlying PTY session with `mio`.
    RegisterSession {
        /// `mio` registry used to register the PTY session.
        registry: &'a Registry,
        /// Initial interest mask for the PTY I/O handle.
        interest: Interest,
        /// Token assigned to I/O readiness events.
        io_token: Token,
        /// Token assigned to child-exit readiness events.
        child_token: Token,
    },
    /// Update registration of the PTY session with a new interest mask.
    ReregisterSession {
        /// `mio` registry used to reregister the PTY session.
        registry: &'a Registry,
        /// Updated interest mask for the PTY I/O handle.
        interest: Interest,
        /// Token assigned to I/O readiness events.
        io_token: Token,
        /// Token assigned to child-exit readiness events.
        child_token: Token,
    },
    /// Remove the PTY session from the `mio` registry.
    DeregisterSession {
        /// `mio` registry used to deregister the PTY session.
        registry: &'a Registry,
    },
    /// The PTY is readable according to the OS event source.
    ReadReady,
    /// The PTY is writable according to the OS event source.
    WriteReady,
    /// Periodic maintenance tick used by terminal implementations.
    Maintain,
    /// A high-level terminal request coming from the `Runtime` command channel.
    Request(TerminalRequest),
}

/// Proxy that used by front-ends to submit [`TerminalRequest`]s to the terminal.
pub struct RuntimeRequestProxy {
    sender: Sender<TerminalRequest>,
    waker: Arc<Waker>,
}

impl Clone for RuntimeRequestProxy {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            waker: Arc::clone(&self.waker),
        }
    }
}

impl RuntimeRequestProxy {
    /// Submit a new request and wake the runtime loop.
    pub fn send(&self, request: TerminalRequest) -> Result<()> {
        self.sender
            .send(request)
            .map_err(|_| Error::RuntimeChannelClosed)?;
        self.waker.wake().map_err(Error::Wake)?;
        Ok(())
    }
}

/// Minimal interface accepted by the [`Runtime`] driver.
///
/// Implementors receive [`RuntimeEvent`]s and can expose additional runtime
/// behaviour, such as pending output, child-process status and dynamic poll
/// timeouts.
pub trait RuntimeClient {
    /// Handle a single runtime event emitted by the [`Runtime`] loop.
    fn handle_runtime_event(&mut self, _event: RuntimeEvent<'_>) -> Result<()> {
        Ok(())
    }

    /// Report whether there are bytes buffered for writing to the PTY.
    ///
    /// When this method returns `true`, the runtime will request writable
    /// readiness from `mio` in addition to readable readiness.
    fn has_pending_output(&self) -> bool {
        false
    }

    /// Check whether the child process attached to the PTY has exited.
    ///
    /// Implementors should return `Ok(Some(status))` once the child exit status
    /// becomes available, and then continue returning `Ok(Some(status))` or
    /// cache the value internally.
    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
        Ok(None)
    }

    /// Deadline for the next maintenance tick.
    ///
    /// If this returns `Some(instant)`, the runtime will use it to compute the
    /// maximum blocking duration for the next `poll` call.
    fn pool_timeout(&self) -> Option<Instant> {
        None
    }
}

/// Hooks that run immediately before and after each poll iteration.
pub trait RuntimeHooks<T: RuntimeClient + ?Sized> {
    /// Called right before polling for OS events.
    ///
    /// This is a good place for front-ends to inject bookkeeping logic that
    /// should run frequently but does not depend on I/O readiness.
    fn before_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }

    /// Called right after completing a full poll iteration.
    ///
    /// This hook runs after queued requests have been drained and the client
    /// has processed any I/O events for the current loop iteration.
    fn after_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }
}

impl<T: RuntimeClient + ?Sized> RuntimeHooks<T> for () {}

/// Mio-backed driver that pumps PTY and child-process events for a terminal runtime.
pub struct Runtime {
    poll: Poll,
    events: Events,
    command_tx: Sender<TerminalRequest>,
    command_rx: Receiver<TerminalRequest>,
    waker: Arc<Waker>,
}

impl Runtime {
    /// Construct a new event loop with the default capacity.
    pub fn new() -> Result<Self> {
        Self::with_capacity(DEFAULT_EVENT_CAPACITY)
    }

    /// Construct a new event loop with a custom event capacity.
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        let poll = Poll::new()?;
        let waker = Arc::new(Waker::new(poll.registry(), RUNTIME_WAKE_TOKEN)?);
        let (command_tx, command_rx) = mpsc::channel();
        Ok(Self {
            poll,
            events: Events::with_capacity(capacity),
            command_tx,
            command_rx,
            waker,
        })
    }

    /// Acquire a handle that can be used to send requests into the runtime.
    pub fn proxy(&self) -> RuntimeRequestProxy {
        RuntimeRequestProxy {
            sender: self.command_tx.clone(),
            waker: Arc::clone(&self.waker),
        }
    }

    /// Drive a [`RuntimeClient`] until shutdown or child exit.
    pub fn run<C, H>(&mut self, mut client: C, mut hooks: H) -> Result<()>
    where
        C: RuntimeClient,
        H: RuntimeHooks<C>,
    {
        let mut interest = Interest::READABLE;
        client.handle_runtime_event(RuntimeEvent::RegisterSession {
            registry: self.poll.registry(),
            interest,
            io_token: PTY_IO_TOKEN,
            child_token: PTY_CHILD_TOKEN,
        })?;

        let mut shutdown_requested = false;
        let mut exit_detected = false;

        loop {
            if shutdown_requested || exit_detected {
                break;
            }

            hooks.before_poll(&mut client)?;
            shutdown_requested |= self.drain_runtime_requests(&mut client)?;
            client.handle_runtime_event(RuntimeEvent::Maintain)?;
            let now = Instant::now();
            let timeout = client
                .pool_timeout()
                .map(|deadline| deadline.saturating_duration_since(now));

            self.poll_once(timeout)?;

            for event in self.events.iter() {
                match event.token() {
                    PTY_IO_TOKEN => {
                        if event.is_readable() {
                            client.handle_runtime_event(
                                RuntimeEvent::ReadReady,
                            )?;
                        }
                        if event.is_writable() {
                            client.handle_runtime_event(
                                RuntimeEvent::WriteReady,
                            )?;
                        }
                    },
                    PTY_CHILD_TOKEN => {
                        if client.check_child_exit()?.is_some() {
                            exit_detected = true;
                        }
                    },
                    RUNTIME_WAKE_TOKEN => {},
                    _ => {},
                };
            }

            shutdown_requested |= self.drain_runtime_requests(&mut client)?;
            client.handle_runtime_event(RuntimeEvent::Maintain)?;

            if !exit_detected && client.check_child_exit()?.is_some() {
                exit_detected = true;
            }

            hooks.after_poll(&mut client)?;

            if exit_detected || shutdown_requested {
                break;
            }

            let mut desired_interest = Interest::READABLE;
            if client.has_pending_output() {
                desired_interest |= Interest::WRITABLE;
            }

            if desired_interest != interest {
                client.handle_runtime_event(
                    RuntimeEvent::ReregisterSession {
                        registry: self.poll.registry(),
                        interest: desired_interest,
                        io_token: PTY_IO_TOKEN,
                        child_token: PTY_CHILD_TOKEN,
                    },
                )?;
                interest = desired_interest;
            }
        }

        client.handle_runtime_event(RuntimeEvent::DeregisterSession {
            registry: self.poll.registry(),
        })?;

        Ok(())
    }

    fn poll_once(&mut self, timeout: Option<Duration>) -> Result<()> {
        self.events.clear();
        loop {
            match self.poll.poll(&mut self.events, timeout) {
                Ok(()) => break,
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => return Err(Error::Poll(err)),
            }
        }

        Ok(())
    }

    fn drain_runtime_requests<C>(&mut self, client: &mut C) -> Result<bool>
    where
        C: RuntimeClient + ?Sized,
    {
        let mut shutdown_requested = false;
        loop {
            match self.command_rx.try_recv() {
                Ok(request) => {
                    if matches!(request, TerminalRequest::Shutdown) {
                        shutdown_requested = true;
                    }
                    client
                        .handle_runtime_event(RuntimeEvent::Request(request))?;
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    shutdown_requested = true;
                    break;
                },
            }
        }

        Ok(shutdown_requested)
    }
}
