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
use crate::pty::Pollable;
use crate::terminal::TerminalEngine;

const PTY_IO_TOKEN: Token = Token(0);
const PTY_CHILD_TOKEN: Token = Token(1);
const RUNTIME_WAKE_TOKEN: Token = Token(2);
const DEFAULT_EVENT_CAPACITY: usize = 128;

/// Driver abstraction for PTY + escape + surface plumbing.
///
/// # Custom loops
/// Use this trait directly when wiring manual or async runtimes:
/// ```ignore
/// // Manual poller example.
/// loop {
///     poller.wait_readable(|| driver.on_readable())?;
///     if driver.has_pending_output() {
///         poller.wait_writable(|| driver.on_writable())?;
///     }
///     driver.tick()?;
/// }
/// ```
///
/// ```ignore
/// // Tokio sketch (readiness + tick).
/// loop {
///     tokio::select! {
///         _ = readable() => driver.on_readable()?,
///         _ = writable(), if driver.has_pending_output() => driver.on_writable()?,
///         _ = tick() => driver.tick()?,
///     }
/// }
/// ```
pub trait Driver {
    /// Register the underlying session with `mio`.
    fn register(
        &mut self,
        registry: &Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()>;

    /// Update interest mask for the registered session handles.
    fn reregister(
        &mut self,
        registry: &Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()>;

    /// Remove the session handles from the registry.
    fn deregister(&mut self, registry: &Registry) -> Result<()>;

    /// Handle readable PTY readiness.
    fn on_readable(&mut self) -> Result<()>;

    /// Handle writable PTY readiness.
    fn on_writable(&mut self) -> Result<()>;

    /// Run periodic maintenance (e.g. sync timeouts).
    fn tick(&mut self) -> Result<()>;

    /// Queue a high-level terminal request.
    fn queue(&mut self, request: TerminalRequest) -> Result<()>;

    /// Whether there is buffered output that needs writable interest.
    fn has_pending_output(&self) -> bool;

    /// Check if the child process has exited.
    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>>;

    /// Deadline for the next tick; used to compute poll timeout.
    fn next_deadline(&self) -> Option<Instant>;
}

/// Hooks that run immediately before and after each poll iteration.
pub trait RuntimeHooks<T: Driver + ?Sized> {
    /// Called right before polling for OS events.
    fn before_poll(&mut self, _driver: &mut T) -> Result<()> {
        Ok(())
    }

    /// Called right after completing a full poll iteration.
    fn after_poll(&mut self, _driver: &mut T) -> Result<()> {
        Ok(())
    }
}

impl<T: Driver + ?Sized> RuntimeHooks<T> for () {}

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

/// Mio-backed driver that pumps PTY and child-process events for a terminal runtime.
///
/// - Use `Runtime::run(engine_driver)` to get a blocking, mio-based loop.
/// - For tokio or custom pollers, construct your own loop that drives a
///   `Driver` by calling `on_readable`, `on_writable`, `tick`, and `queue`.
/// - Wake tokens remain compatible with the `Session`/`Pollable` traits.
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

    /// Drive a [`Driver`] until shutdown or child exit.
    pub fn run<D, H>(&mut self, driver: &mut D, mut hooks: H) -> Result<()>
    where
        D: Driver,
        H: RuntimeHooks<D>,
    {
        let mut interest = Interest::READABLE;
        driver.register(
            self.poll.registry(),
            interest,
            PTY_IO_TOKEN,
            PTY_CHILD_TOKEN,
        )?;

        let mut shutdown_requested = false;
        let mut exit_detected = false;

        loop {
            hooks.before_poll(driver)?;
            shutdown_requested |= self.drain_runtime_requests(driver)?;
            driver.tick()?;

            let now = Instant::now();
            let timeout = driver
                .next_deadline()
                .map(|deadline| deadline.saturating_duration_since(now));

            self.poll_once(timeout)?;

            for event in self.events.iter() {
                match event.token() {
                    PTY_IO_TOKEN => {
                        if event.is_readable() {
                            driver.on_readable()?;
                        }
                        if event.is_writable() {
                            driver.on_writable()?;
                        }
                    },
                    PTY_CHILD_TOKEN => {
                        if driver.check_child_exit()?.is_some() {
                            exit_detected = true;
                        }
                    },
                    RUNTIME_WAKE_TOKEN => {},
                    _ => {},
                };
            }

            shutdown_requested |= self.drain_runtime_requests(driver)?;
            driver.tick()?;

            if !exit_detected && driver.check_child_exit()?.is_some() {
                exit_detected = true;
            }

            hooks.after_poll(driver)?;

            let mut desired_interest = Interest::READABLE;
            if driver.has_pending_output() {
                desired_interest |= Interest::WRITABLE;
            }

            if desired_interest != interest {
                driver.reregister(
                    self.poll.registry(),
                    desired_interest,
                    PTY_IO_TOKEN,
                    PTY_CHILD_TOKEN,
                )?;
                interest = desired_interest;
            }

            if exit_detected || shutdown_requested {
                break;
            }
        }

        driver.deregister(self.poll.registry())?;

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

    fn drain_runtime_requests<D>(&mut self, driver: &mut D) -> Result<bool>
    where
        D: Driver + ?Sized,
    {
        let mut shutdown_requested = false;
        loop {
            match self.command_rx.try_recv() {
                Ok(request) => {
                    if matches!(request, TerminalRequest::Shutdown) {
                        shutdown_requested = true;
                    }
                    driver.queue(request)?;
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

impl<P, E, S> Driver for TerminalEngine<P, E, S>
where
    P: crate::pty::Session + Pollable,
    E: crate::escape::EscapeParser,
    S: crate::surface::SurfaceActor + crate::surface::SurfaceModel,
{
    fn register(
        &mut self,
        registry: &Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()> {
        self.register_session(registry, interest, io_token, child_token)
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()> {
        self.reregister_session(registry, interest, io_token, child_token)
    }

    fn deregister(&mut self, registry: &Registry) -> Result<()> {
        self.deregister_session(registry)
    }

    fn on_readable(&mut self) -> Result<()> {
        let _ = TerminalEngine::on_readable(self)?;
        Ok(())
    }

    fn on_writable(&mut self) -> Result<()> {
        let _ = TerminalEngine::on_writable(self)?;
        Ok(())
    }

    fn tick(&mut self) -> Result<()> {
        TerminalEngine::tick(self)
    }

    fn queue(&mut self, request: TerminalRequest) -> Result<()> {
        TerminalEngine::queue_request(self, request)
    }

    fn has_pending_output(&self) -> bool {
        TerminalEngine::has_pending_output(self)
    }

    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
        TerminalEngine::check_child_exit(self)
    }

    fn next_deadline(&self) -> Option<Instant> {
        TerminalEngine::next_deadline(self)
    }
}

#[cfg(test)]
mod tests {
    use std::process::ExitStatus;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    use mio::{Interest, Registry, Token};

    use crate::tests::exit_ok;
    use crate::{Result, TerminalRequest};

    use super::{Driver, Runtime};

    #[derive(Default)]
    struct StubDriver {
        registrations: Vec<Interest>,
        reregistrations: Vec<Interest>,
        requests: Vec<TerminalRequest>,
        exit_status: Option<ExitStatus>,
        pending_output: Arc<AtomicBool>,
        deadline: Option<Instant>,
        readable_count: usize,
        writable_count: usize,
        tick_count: usize,
        deregistered: bool,
    }

    impl StubDriver {
        fn with_pending_output() -> Self {
            Self {
                pending_output: Arc::new(AtomicBool::new(true)),
                deadline: Some(Instant::now()),
                ..Default::default()
            }
        }

        fn mark_exit(&mut self, status: ExitStatus) {
            self.exit_status = Some(status);
        }
    }

    impl Driver for StubDriver {
        fn register(
            &mut self,
            _registry: &Registry,
            interest: Interest,
            _io_token: Token,
            _child_token: Token,
        ) -> Result<()> {
            self.registrations.push(interest);
            Ok(())
        }

        fn reregister(
            &mut self,
            _registry: &Registry,
            interest: Interest,
            _io_token: Token,
            _child_token: Token,
        ) -> Result<()> {
            self.reregistrations.push(interest);
            Ok(())
        }

        fn deregister(&mut self, _registry: &Registry) -> Result<()> {
            self.deregistered = true;
            Ok(())
        }

        fn on_readable(&mut self) -> Result<()> {
            self.readable_count += 1;
            Ok(())
        }

        fn on_writable(&mut self) -> Result<()> {
            self.pending_output.store(false, Ordering::SeqCst);
            self.writable_count += 1;
            Ok(())
        }

        fn tick(&mut self) -> Result<()> {
            self.tick_count += 1;
            Ok(())
        }

        fn queue(&mut self, request: TerminalRequest) -> Result<()> {
            if matches!(request, TerminalRequest::WriteBytes(_)) {
                self.pending_output.store(true, Ordering::SeqCst);
            }
            self.requests.push(request);
            Ok(())
        }

        fn has_pending_output(&self) -> bool {
            self.pending_output.load(Ordering::SeqCst)
        }

        fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
            Ok(self.exit_status)
        }

        fn next_deadline(&self) -> Option<Instant> {
            self.deadline
        }
    }

    #[test]
    fn runtime_processes_requests_and_shutdown() -> Result<()> {
        let mut runtime = Runtime::new()?;
        let proxy = runtime.proxy();
        proxy.send(TerminalRequest::WriteBytes(b"hi".to_vec()))?;
        proxy.send(TerminalRequest::Shutdown)?;

        let mut driver = StubDriver {
            deadline: Some(Instant::now()),
            ..Default::default()
        };

        runtime.run(&mut driver, ())?;

        assert_eq!(driver.requests.len(), 2);
        assert!(
            driver
                .requests
                .iter()
                .any(|req| matches!(req, TerminalRequest::Shutdown))
        );
        assert!(driver.deregistered);

        Ok(())
    }

    #[test]
    fn runtime_detects_child_exit() -> Result<()> {
        let mut runtime = Runtime::new()?;
        let mut driver = StubDriver {
            deadline: Some(Instant::now()),
            ..Default::default()
        };
        driver.mark_exit(exit_ok());

        runtime.run(&mut driver, ())?;

        assert!(driver.deregistered);
        Ok(())
    }

    #[test]
    fn runtime_toggles_writable_interest() -> Result<()> {
        let mut runtime = Runtime::new()?;
        let proxy = runtime.proxy();
        proxy.send(TerminalRequest::WriteBytes(b"bytes".to_vec()))?;
        proxy.send(TerminalRequest::Shutdown)?;

        let mut driver = StubDriver::with_pending_output();

        runtime.run(&mut driver, ())?;

        assert!(!driver.reregistrations.is_empty());
        assert!(
            driver
                .reregistrations
                .iter()
                .any(|interest| interest.is_writable())
        );

        Ok(())
    }

    #[test]
    fn runtime_ticks_with_deadline() -> Result<()> {
        let mut runtime = Runtime::new()?;
        let proxy = runtime.proxy();
        proxy.send(TerminalRequest::Shutdown)?;

        let mut driver = StubDriver {
            deadline: Some(Instant::now()),
            ..Default::default()
        };

        runtime.run(&mut driver, ())?;

        assert!(driver.tick_count > 0);
        Ok(())
    }
}
