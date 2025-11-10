use std::io::ErrorKind;
use std::process::ExitStatus;
use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender, TryRecvError},
};
use std::time::Duration;

use cursor_icon::CursorIcon;
use mio::{Events, Interest, Poll, Registry, Token, Waker};
use otty_escape::{CursorShape, CursorStyle, Hyperlink};

use crate::TerminalRequest;
use crate::{
    TerminalSnapshot,
    error::{LibTermError, Result},
    pty::PtySize,
    surface::ScrollDirection,
};

const PTY_IO_TOKEN: Token = Token(0);
const PTY_CHILD_TOKEN: Token = Token(1);
const RUNTIME_WAKE_TOKEN: Token = Token(2);
const DEFAULT_EVENT_CAPACITY: usize = 128;

/// Events sent from the runtime driver into terminal implementations.
pub enum RuntimeEvent<'a> {
    RegisterSession {
        registry: &'a Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    },
    ReregisterSession {
        registry: &'a Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    },
    DeregisterSession {
        registry: &'a Registry,
    },
    ReadReady,
    WriteReady,
    Maintain,
    Request(TerminalRequest),
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

/// Minimal interface accepted by the [`Runtime`] driver.
pub trait RuntimeClient {
    fn handle_runtime_event(&mut self, _event: RuntimeEvent<'_>) -> Result<()> {
        Ok(())
    }

    fn has_pending_output(&self) -> bool { false }

    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
        Ok(None)
    }
}

/// Hooks that run immediately before and after each poll iteration.
pub trait PollHookHandler<T: RuntimeClient + ?Sized> {
    fn before_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }

    fn after_poll(&mut self, _terminal: &mut T) -> Result<()> {
        Ok(())
    }
}

impl<T: RuntimeClient + ?Sized> PollHookHandler<T> for () {}

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
            poll_timeout: Some(Duration::from_millis(150)),
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

    // TODO: need refactor
    /// Run the event loop, delegating polling hooks to the provided handler.
    pub fn run<C, H>(&mut self, client: &mut C, hooks: &mut H) -> Result<()>
    where
        C: RuntimeClient + ?Sized,
        H: PollHookHandler<C> + ?Sized,
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

            hooks.before_poll(client)?;
            shutdown_requested |= self.drain_runtime_requests(client)?;
            client.handle_runtime_event(RuntimeEvent::Maintain)?;
            self.poll_once()?;

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

            shutdown_requested |= self.drain_runtime_requests(client)?;
            client.handle_runtime_event(RuntimeEvent::Maintain)?;

            if !exit_detected && client.check_child_exit()?.is_some() {
                exit_detected = true;
            }

            hooks.after_poll(client)?;

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
