use std::io::ErrorKind;
use std::process::ExitStatus;
use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender, TryRecvError},
};
use std::time::Duration;

use mio::{Events, Interest, Poll, Token, Waker};

use crate::{
    error::{LibTermError, Result},
    pty::PtySize,
};

const PTY_IO_TOKEN: Token = Token(0);
const PTY_CHILD_TOKEN: Token = Token(1);
const RUNTIME_WAKE_TOKEN: Token = Token(2);
const DEFAULT_EVENT_CAPACITY: usize = 128;

/// Commands that the runtime understands for mutating the terminal state.
#[derive(Debug, Clone)]
pub enum TerminalRequest {
    /// Write raw bytes into the PTY.
    Write(Vec<u8>),
    /// Resize the PTY/session.
    Resize(PtySize),
    /// Close the session and terminate the event loop.
    Shutdown,
}

/// Events emitted by terminal implementations to interested clients.
#[derive(Debug)]
pub enum TerminalEvent<'a, SurfaceHandle> {
    SurfaceChanged { surface: &'a SurfaceHandle },
    ChildExit { status: &'a ExitStatus },
}

/// Handle used by front-ends to submit [`TerminalRequest`]s to the runtime.
pub struct RuntimeHandle {
    sender: Sender<TerminalRequest>,
    waker: Arc<Waker>,
}

impl Clone for RuntimeHandle {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            waker: Arc::clone(&self.waker),
        }
    }
}

impl RuntimeHandle {
    /// Submit a new request and wake the runtime loop.
    pub fn send(&self, request: TerminalRequest) -> Result<()> {
        self.sender
            .send(request)
            .map_err(|_| LibTermError::RuntimeChannelClosed)?;
        self.waker.wake().map_err(LibTermError::Wake)?;
        Ok(())
    }
}

/// Register/unregister the underlying pollable resources with Mio.
pub trait SessionRegistration {
    fn register_session(
        &mut self,
        registry: &mio::Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()>;

    fn reregister_session(
        &mut self,
        registry: &mio::Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()>;

    fn deregister_session(&mut self, registry: &mio::Registry) -> Result<()>;
}

/// Handle IO readiness notifications from Mio.
pub trait IoHandler {
    fn handle_read_ready(&mut self) -> Result<bool>;

    fn handle_write_ready(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Report lifecycle state to the runtime driver.
pub trait LifecycleControl {
    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>>;
}

/// Provide optional maintenance hooks and poll configuration.
pub trait RuntimeMaintenance {
    fn maintain(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Process commands submitted via [`RuntimeHandle`].
pub trait RequestHandler {
    fn handle_request(&mut self, request: TerminalRequest) -> Result<()> {
        let _ = request;
        Ok(())
    }
}

/// Report whether the target has buffered output waiting to be flushed.
pub trait PendingOutput {
    fn has_pending_output(&self) -> bool {
        false
    }
}

/// Minimal trait object accepted by the [`Runtime`] driver.
pub trait RuntimeTarget:
    SessionRegistration
    + IoHandler
    + LifecycleControl
    + RuntimeMaintenance
    + RequestHandler
    + PendingOutput
{
}

impl<T> RuntimeTarget for T where
    T: SessionRegistration
        + IoHandler
        + LifecycleControl
        + RuntimeMaintenance
        + RequestHandler
        + PendingOutput
{
}

/// Callback interface for consuming [`TerminalEvent`]s emitted by terminal instances.
pub trait TerminalClient<SurfaceHandle> {
    /// Handle a single terminal event produced by the terminal.
    fn handle_event(
        &mut self,
        _event: TerminalEvent<'_, SurfaceHandle>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Hooks that run immediately before and after each poll iteration.
pub trait PollHookHandler<T: RuntimeTarget + ?Sized> {
    fn before_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }

    fn after_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }
}

impl<T: RuntimeTarget + ?Sized> PollHookHandler<T> for () {}

/// Mio-backed driver that pumps PTY and child-process events for a terminal runtime.
pub struct Runtime {
    poll: Poll,
    events: Events,
    command_tx: Sender<TerminalRequest>,
    command_rx: Receiver<TerminalRequest>,
    waker: Arc<Waker>,
    poll_timeout: Option<Duration>,
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
            poll_timeout: Some(Duration::from_millis(16)),
        })
    }

    pub fn set_poll_timeout(&mut self, timeout: Option<Duration>) {
        self.poll_timeout = timeout;
    }

    /// Acquire a handle that can be used to send requests into the runtime.
    #[must_use]
    pub fn handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            sender: self.command_tx.clone(),
            waker: Arc::clone(&self.waker),
        }
    }

    /// Run the event loop, delegating polling hooks to the provided handler.
    pub fn run<T, H>(&mut self, terminal: &mut T, hooks: &mut H) -> Result<()>
    where
        T: RuntimeTarget + ?Sized,
        H: PollHookHandler<T> + ?Sized,
    {
        let mut interest = Interest::READABLE;
        terminal.register_session(
            self.poll.registry(),
            interest,
            PTY_IO_TOKEN,
            PTY_CHILD_TOKEN,
        )?;

        let run_result = (|| -> Result<()> {
            let mut shutdown_requested = false;
            let mut exit_detected = false;

            loop {
                if shutdown_requested || exit_detected {
                    break;
                }

                hooks.before_poll(terminal)?;
                shutdown_requested |= self.drain_runtime_requests(terminal)?;
                terminal.maintain()?;
                self.poll_once()?;

                for event in self.events.iter() {
                    match event.token() {
                        PTY_IO_TOKEN => {
                            if event.is_readable() {
                                terminal.handle_read_ready()?;
                            }
                            if event.is_writable() {
                                terminal.handle_write_ready()?;
                            }
                        },
                        PTY_CHILD_TOKEN => {
                            if terminal.check_child_exit()?.is_some() {
                                exit_detected = true;
                            }
                        },
                        RUNTIME_WAKE_TOKEN => {},
                        _ => {},
                    };
                }

                shutdown_requested |= self.drain_runtime_requests(terminal)?;
                terminal.maintain()?;

                if !exit_detected && terminal.check_child_exit()?.is_some() {
                    exit_detected = true;
                }

                hooks.after_poll(terminal)?;

                if exit_detected || shutdown_requested {
                    break;
                }

                let mut desired_interest = Interest::READABLE;
                if terminal.has_pending_output() {
                    desired_interest |= Interest::WRITABLE;
                }

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

        run_result?;
        deregister_result?;

        Ok(())
    }

    fn poll_once(&mut self) -> Result<()> {
        self.events.clear();
        loop {
            match self.poll.poll(&mut self.events, self.poll_timeout) {
                Ok(()) => break,
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => return Err(LibTermError::Poll(err)),
            }
        }

        Ok(())
    }

    fn drain_runtime_requests<T>(&mut self, terminal: &mut T) -> Result<bool>
    where
        T: RuntimeTarget + ?Sized,
    {
        let mut shutdown_requested = false;
        loop {
            match self.command_rx.try_recv() {
                Ok(request) => {
                    if matches!(request, TerminalRequest::Shutdown) {
                        shutdown_requested = true;
                    }
                    terminal.handle_request(request)?;
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
