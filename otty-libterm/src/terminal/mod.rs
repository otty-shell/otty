pub mod mode;
pub mod cell;
pub mod color;
pub mod index;
pub mod actor;
pub mod surface;
pub mod options;
pub mod snapshot;
pub mod surface_actor;

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use cursor_icon::CursorIcon;

use crate::{
    escape::{Action, CursorShape, CursorStyle, EscapeParser, Hyperlink, KeyboardMode},
    pty::{Pollable, PtySize, Session, SessionError},
};
use crate::grid::Scroll;
use crate::{
    Result, RuntimeClient, RuntimeEvent,
};

use actor::SurfaceActor;
use surface::SurfaceSnapshotSource;
use surface_actor::TerminalSurfaceActor;
use mode::TerminalMode;
use snapshot::TerminalSnapshot;
use options::TerminalOptions;

const DEFAULT_READ_BUFFER_CAPACITY: usize = 1024;

/// Events emitted by terminal implementations to interested clients.
pub enum TerminalEvent<'a> {
    SurfaceChanged { snapshot: TerminalSnapshot<'a> },
    ChildExit { status: ExitStatus },
    TitleChanged { title: String },
    Bell,
    CursorShapeChanged { shape: CursorShape },
    CursorStyleChanged { style: Option<CursorStyle> },
    CursorIconChanged { icon: CursorIcon },
    Hyperlink { link: Option<Hyperlink> },
}

/// Commands that the runtime understands for mutating the terminal state.
#[derive(Debug, Clone)]
pub enum TerminalRequest {
    /// Write raw bytes into the PTY.
    Write(Vec<u8>),
    /// Emit a mouse report (front-ends may encode the bytes directly).
    MouseReport(Vec<u8>),
    /// Resize the PTY/session.
    Resize(PtySize),
    /// Scroll the display viewport.
    ScrollDisplay(Scroll),
    /// Close the session and terminate the event loop.
    Shutdown,
}

const MAX_SYNC_ACTIONS: usize = 10_000;
const SYNC_TIMEOUT: Duration = Duration::from_millis(10);
const IDLE_TICK: Duration = Duration::from_millis(10);

pub(crate) struct SyncState {
    active: bool,
    buffer: Vec<Action>,
    deadline: Option<Instant>,
}

impl SyncState {
    fn new() -> Self {
        let mut state = Self {
            active: false,
            buffer: Vec::with_capacity(MAX_SYNC_ACTIONS),
            deadline: None,
        };
        state.refresh_deadline();
        state
    }

    fn begin(&mut self) {
        self.active = true;
        self.buffer.clear();
        self.refresh_deadline();
    }

    fn end(&mut self) -> Vec<Action> {
        self.active = false;
        self.refresh_deadline();
        std::mem::take(&mut self.buffer)
    }

    fn cancel(&mut self) -> Vec<Action> {
        self.active = false;
        self.refresh_deadline();
        std::mem::take(&mut self.buffer)
    }

    fn push(&mut self, action: Action) -> std::result::Result<(), Action> {
        if self.buffer.len() >= MAX_SYNC_ACTIONS {
            return Err(action);
        }
        self.buffer.push(action);
        self.refresh_deadline();
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn is_expired(&self) -> bool {
        self.deadline
            .is_some_and(|deadline| Instant::now() > deadline)
    }

    fn refresh_deadline(&mut self) {
        let timeout = if self.active { SYNC_TIMEOUT } else { IDLE_TICK };
        self.deadline = Some(Instant::now() + timeout);
    }
}

/// Callback interface for consuming [`TerminalEvent`]s emitted by terminal instances.
pub trait TerminalClient {
    /// Handle a single terminal event produced by the terminal.
    fn handle_event(&mut self, _event: TerminalEvent) -> Result<()> {
        Ok(())
    }
}

/// High level runtime that connects a PTY session with the escape parser and
/// in-memory surface model.
pub struct Terminal<P, E, S> {
    session: P,
    parser: E,
    surface: S,
    read_buffer: Vec<u8>,
    exit_status: Option<ExitStatus>,
    mode: TerminalMode,
    keyboard_mode: KeyboardMode,
    keyboard_stack: Vec<KeyboardMode>,
    pending_input: VecDeque<u8>,
    event_client: Option<Box<dyn TerminalClient + 'static>>,
    sync_state: SyncState,
}

impl<P, E, S> Terminal<P, E, S>
where
    P: Session,
    E: EscapeParser,
    S: SurfaceActor + SurfaceSnapshotSource,
{
    pub fn new(
        session: P,
        surface: S,
        parser: E,
        options: TerminalOptions,
    ) -> Result<Self> {
        let mut read_buffer = vec![
            0u8;
            options
                .read_buffer_capacity
                .max(DEFAULT_READ_BUFFER_CAPACITY)
        ];
        if read_buffer.is_empty() {
            read_buffer.resize(DEFAULT_READ_BUFFER_CAPACITY, 0);
        }

        Ok(Self {
            session,
            parser,
            surface,
            read_buffer,
            exit_status: None,
            mode: TerminalMode::default(),
            keyboard_mode: KeyboardMode::default(),
            keyboard_stack: Vec::new(),
            pending_input: VecDeque::new(),
            event_client: None,
            sync_state: SyncState::new(),
        })
    }

    /// Attach a client that will receive [`TerminalEvent`] callbacks directly from this terminal.
    pub fn set_event_client<C>(&mut self, client: C)
    where
        C: TerminalClient + 'static,
    {
        self.event_client = Some(Box::new(client));
    }

    /// Remove the currently attached event client.
    pub fn take_event_client(
        &mut self,
    ) -> Option<Box<dyn TerminalClient + 'static>> {
        self.event_client.take()
    }

    /// Check whether the terminal has an event client attached.
    pub fn has_event_client(&self) -> bool {
        self.event_client.is_some()
    }

    /// Flush any buffered PTY output when operating without the [`Runtime`](crate::Runtime).
    pub fn flush_pending_input(&mut self) -> Result<()> {
        while !self.pending_input.is_empty() {
            let chunk = {
                let slice = self.pending_input.make_contiguous();
                slice.to_vec()
            };

            if chunk.is_empty() {
                break;
            }

            let total = chunk.len();
            let written = self.write(&chunk)?;

            if written == 0 {
                break;
            }

            self.pending_input
                .drain(0..written.min(self.pending_input.len()));

            if written < total {
                break;
            }
        }

        Ok(())
    }

    /// Drop expired synchronized-update buffers even if no new PTY data arrives.
    pub fn flush_sync_timeout(&mut self) -> Result<bool> {
        let mut event_client = self.event_client.take();
        let flushed = {
            let mut actor = TerminalSurfaceActor {
                surface: &mut self.surface,
                mode: &mut self.mode,
                keyboard_mode: &mut self.keyboard_mode,
                keyboard_stack: &mut self.keyboard_stack,
                client: &mut event_client,
                pending_input: &mut self.pending_input,
                sync_state: &mut self.sync_state,
            };
            actor.flush_sync_timeout()
        };
        self.event_client = event_client;
        if flushed {
            self.emit_surface_change()?;
        }
        Ok(flushed)
    }

    fn read(&mut self) -> Result<bool> {
        let mut updated = false;

        loop {
            match self.session.read(self.read_buffer.as_mut_slice()) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = &self.read_buffer[..count];
                    let parser = &mut self.parser;
                    let surface = &mut self.surface;
                    let mode = &mut self.mode;
                    let keyboard_mode = &mut self.keyboard_mode;
                    let keyboard_stack = &mut self.keyboard_stack;
                    let mut event_client = self.event_client.take();
                    {
                        let mut actor = TerminalSurfaceActor {
                            surface,
                            mode,
                            keyboard_mode,
                            keyboard_stack,
                            client: &mut event_client,
                            pending_input: &mut self.pending_input,
                            sync_state: &mut self.sync_state,
                        };
                        parser.advance(chunk, &mut actor);
                        let _ = actor.flush_sync_timeout();
                    }
                    self.event_client = event_client;
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

        if updated {
            self.emit_surface_change()?;
        }

        Ok(updated)
    }

    /// Capture the current terminal state without mutating it.
    pub fn snapshot(&mut self) -> TerminalSnapshot {
        let surface = self.surface.capture_snapshot();
        TerminalSnapshot::new(surface, self.mode, self.keyboard_mode)
    }

    pub fn process_request(&mut self, request: TerminalRequest) -> Result<()> {
        match request {
            TerminalRequest::Write(bytes) => {
                self.enqueue_input(bytes);
                self.flush_pending_input()?;
            },
            TerminalRequest::MouseReport(bytes) => {
                self.enqueue_input(bytes);
                self.flush_pending_input()?;
            },
            TerminalRequest::Resize(size) => self.resize(size)?,
            TerminalRequest::ScrollDisplay(direction) => {
                self.surface.scroll_display(direction);
                self.emit_surface_change()?;
            },
            TerminalRequest::Shutdown => {
                self.close()?;
            },
        }

        Ok(())
    }

    /// Write a chunk of bytes to the PTY session.
    fn write(&mut self, bytes: &[u8]) -> Result<usize> {
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
    fn resize(&mut self, size: PtySize) -> Result<()> {
        self.session.resize(size)?;
        self.surface.resize(size.cols as usize, size.rows as usize);
        self.emit_surface_change()
    }

    /// Terminate the session and return the reported exit status code.
    fn close(&mut self) -> Result<i32> {
        let code = self.session.close()?;
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            let status: ExitStatus = ExitStatusExt::from_raw(code);
            self.emit_child_exit(status)?;
            self.exit_status = Some(status);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            let status: ExitStatus = ExitStatusExt::from_raw(code as u32);
            self.emit_child_exit(status)?;
            self.exit_status = Some(status);
        }
        Ok(code)
    }

    fn capture_exit(&mut self) -> Result<Option<ExitStatus>> {
        match self.session.try_get_child_exit_status() {
            Ok(Some(status)) => {
                self.exit_status = Some(status);
                self.emit_child_exit(status)?;
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

    fn enqueue_input(&mut self, data: Vec<u8>) {
        if data.is_empty() {
            return;
        }
        self.pending_input.extend(data);
    }

    fn emit_surface_change(&mut self) -> Result<()> {
        if let Some(mut client) = self.event_client.take() {
            let snapshot = self.snapshot();
            client.handle_event(TerminalEvent::SurfaceChanged { snapshot })?;
            self.event_client = Some(client);
        }
        Ok(())
    }

    fn emit_child_exit(&mut self, status: ExitStatus) -> Result<()> {
        if let Some(mut client) = self.event_client.take() {
            client.handle_event(TerminalEvent::ChildExit { status })?;
            self.event_client = Some(client);
        }

        Ok(())
    }
}

impl<P, E, S> RuntimeClient for Terminal<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceActor + SurfaceSnapshotSource,
{
    fn handle_runtime_event(&mut self, event: RuntimeEvent<'_>) -> Result<()> {
        match event {
            RuntimeEvent::RegisterSession {
                registry,
                interest,
                io_token,
                child_token,
            } => {
                self.session.register(
                    registry,
                    interest,
                    io_token,
                    child_token,
                )?;
            },
            RuntimeEvent::ReregisterSession {
                registry,
                interest,
                io_token,
                child_token,
            } => {
                self.session.reregister(
                    registry,
                    interest,
                    io_token,
                    child_token,
                )?;
            },
            RuntimeEvent::DeregisterSession { registry } => {
                self.session.deregister(registry)?;
            },
            RuntimeEvent::ReadReady => {
                let _ = self.read()?;
            },
            RuntimeEvent::WriteReady => {
                self.flush_pending_input()?;
            },
            RuntimeEvent::Maintain => {
                self.flush_pending_input()?;
                let _ = self.flush_sync_timeout()?;
            },
            RuntimeEvent::Request(request) => {
                self.process_request(request)?;
            },
        }
        Ok(())
    }

    fn has_pending_output(&self) -> bool {
        !self.pending_input.is_empty()
    }

    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
        self.capture_exit()
    }

    fn pool_timeout(&self) -> Option<Instant> {
        self.sync_state.deadline
    }
}
