pub mod builder;
pub mod channel;
pub mod options;
pub mod size;
pub mod surface_actor;

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cursor_icon::CursorIcon;
use flume::{Receiver, Sender};

use crate::Result;
use crate::terminal::channel::{
    ChannelSendError, TerminalEvents, TerminalHandle, map_send_error,
};
use crate::terminal::size::TerminalSize;
use crate::terminal::surface_actor::TerminalSurfaceActor;
use crate::{
    escape::{Action, CursorShape, CursorStyle, EscapeParser, Hyperlink},
    pty::{Pollable, Session, SessionError},
    surface::{
        Point, Scroll, SelectionType, Side, SnapshotOwned, SurfaceActor,
        SurfaceModel,
    },
};

use options::TerminalOptions;

/// Owned frame wrapper shared with terminal consumers.
pub type SnapshotArc = Arc<SnapshotOwned>;

const DEFAULT_READ_BUFFER_CAPACITY: usize = 1024;

/// Events emitted by terminal implementations to interested clients.
pub enum TerminalEvent {
    /// The in-memory surface contents have changed.
    ///
    /// Front-ends typically respond by re-rendering the provided frame.
    Frame { frame: SnapshotArc },
    /// The child process attached to the PTY has exited.
    ChildExit { status: ExitStatus },
    /// The terminal's window or tab title has changed.
    TitleChanged { title: String },
    /// Reset the terminal's window or tab title to its default value.
    ResetTitle,
    /// An audible bell was requested by the remote application.
    Bell,
    /// The visual shape of the cursor has changed.
    CursorShapeChanged { shape: CursorShape },
    /// The cursor style (e.g. blinking mode) has changed.
    CursorStyleChanged { style: Option<CursorStyle> },
    /// The pointing device cursor/icon has changed.
    CursorIconChanged { icon: CursorIcon },
    /// The currently active hyperlink under the cursor has changed.
    Hyperlink { link: Option<Hyperlink> },
}

/// Commands that the runtime understands for mutating the terminal state.
#[derive(Debug, Clone)]
pub enum TerminalRequest {
    /// Write raw bytes into the PTY.
    WriteBytes(Vec<u8>),
    /// Resize the PTY/session.
    Resize(TerminalSize),
    /// Scroll the display viewport.
    ScrollDisplay(Scroll),
    /// Initialize the selection range on the surface.
    StartSelection {
        ty: SelectionType,
        point: Point,
        direction: Side,
    },
    /// Update the active selection range on the surface.
    UpdateSelection { point: Point, direction: Side },
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
    /// Create a new sync state with a fresh deadline.
    fn new() -> Self {
        let mut state = Self {
            active: false,
            buffer: Vec::with_capacity(MAX_SYNC_ACTIONS),
            deadline: None,
        };
        state.refresh_deadline();
        state
    }

    /// Enter synchronized-update mode, buffering subsequent actions.
    fn begin(&mut self) {
        self.active = true;
        self.buffer.clear();
        self.refresh_deadline();
    }

    /// End synchronized-update mode and drain buffered actions.
    fn end(&mut self) -> Vec<Action> {
        self.active = false;
        self.refresh_deadline();
        std::mem::take(&mut self.buffer)
    }

    /// Cancel synchronized-update mode and drain buffered actions.
    fn cancel(&mut self) -> Vec<Action> {
        self.active = false;
        self.refresh_deadline();
        std::mem::take(&mut self.buffer)
    }

    /// Try to push a new action into the sync buffer.
    ///
    /// Returns the action back on overflow so that callers can fall back to
    /// immediate processing.
    #[allow(clippy::result_large_err)]
    fn push(&mut self, action: Action) -> std::result::Result<(), Action> {
        if self.buffer.len() >= MAX_SYNC_ACTIONS {
            return Err(action);
        }

        self.buffer.push(action);
        self.refresh_deadline();
        Ok(())
    }

    /// Check whether synchronized-update mode is currently active.
    fn is_active(&self) -> bool {
        self.active
    }

    /// Check whether the current deadline has expired.
    fn is_expired(&self) -> bool {
        self.deadline
            .is_some_and(|deadline| Instant::now() > deadline)
    }

    /// Refresh the internal deadline based on the current mode.
    fn refresh_deadline(&mut self) {
        let timeout = if self.active { SYNC_TIMEOUT } else { IDLE_TICK };
        self.deadline = Some(Instant::now() + timeout);
    }
}

/// High level engine that connects a PTY session with the escape parser and
/// in-memory surface model.
pub struct TerminalEngine<P, E, S> {
    session: P,
    parser: E,
    surface: S,
    size: TerminalSize,
    read_buffer: Vec<u8>,
    exit_status: Option<ExitStatus>,
    event_tx: Sender<TerminalEvent>,
    request_rx: Receiver<TerminalRequest>,
    pending_input: VecDeque<u8>,
    pending_requests: VecDeque<TerminalRequest>,
    events: VecDeque<TerminalEvent>,
    sync_state: SyncState,
}

impl<P, E, S> TerminalEngine<P, E, S>
where
    P: Session,
    E: EscapeParser,
    S: SurfaceActor + SurfaceModel,
{
    pub fn new(
        session: P,
        parser: E,
        surface: S,
        options: TerminalOptions,
    ) -> Result<(Self, TerminalHandle, TerminalEvents)> {
        let (event_tx, event_rx, request_tx, request_rx) =
            channel::build_channels(&options.channel_config);

        let handle = TerminalHandle::new(request_tx);
        let events = TerminalEvents::new(event_rx);

        let mut read_buffer = vec![
            0u8;
            options
                .read_buffer_capacity
                .max(DEFAULT_READ_BUFFER_CAPACITY)
        ];
        if read_buffer.is_empty() {
            read_buffer.resize(DEFAULT_READ_BUFFER_CAPACITY, 0);
        }

        Ok((
            Self {
                session,
                parser,
                surface,
                read_buffer,
                size: TerminalSize::default(),
                exit_status: None,
                event_tx,
                request_rx,
                pending_input: VecDeque::new(),
                pending_requests: VecDeque::new(),
                events: VecDeque::new(),
                sync_state: SyncState::new(),
            },
            handle,
            events,
        ))
    }

    /// Push a request into the engine for processing.
    pub fn queue_request(&mut self, request: TerminalRequest) -> Result<()> {
        self.pending_requests.push_back(request);
        Ok(())
    }

    /// Process readable PTY data and emit any resulting events.
    pub fn on_readable(&mut self) -> Result<bool> {
        self.process_pending_requests()?;

        let mut updated = false;

        loop {
            match self.session.read(self.read_buffer.as_mut_slice()) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = &self.read_buffer[..count];
                    let parser = &mut self.parser;
                    let surface = &mut self.surface;
                    {
                        let mut actor = TerminalSurfaceActor {
                            surface,
                            events: &mut self.events,
                            pending_input: &mut self.pending_input,
                            sync_state: &mut self.sync_state,
                        };
                        parser.advance(chunk, &mut actor);
                        let _ = actor.flush_sync_timeout();
                    }
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
                Err(SessionError::IO(ref err))
                    if is_session_closed_error(err) =>
                {
                    self.capture_exit()?;
                    break;
                },
                Err(err) => return Err(err.into()),
            }
        }

        if updated {
            self.emit_frame()?;
        }

        self.capture_exit()?;

        self.flush_event_queue()?;

        Ok(updated)
    }

    /// Flush any buffered PTY output.
    pub fn on_writable(&mut self) -> Result<bool> {
        self.process_pending_requests()?;
        self.flush_pending_input()?;
        self.flush_event_queue()?;
        Ok(!self.pending_input.is_empty())
    }

    /// Handle periodic maintenance ticks (e.g. sync-mode timeouts).
    pub fn tick(&mut self) -> Result<()> {
        self.process_pending_requests()?;

        let flushed = {
            let mut actor = TerminalSurfaceActor {
                surface: &mut self.surface,
                events: &mut self.events,
                pending_input: &mut self.pending_input,
                sync_state: &mut self.sync_state,
            };
            actor.flush_sync_timeout()
        };

        if flushed {
            self.emit_frame()?;
        }

        self.capture_exit()?;

        self.flush_event_queue()?;

        Ok(())
    }

    /// Return whether there is buffered output waiting to be written.
    pub fn has_pending_output(&self) -> bool {
        !self.pending_input.is_empty()
            || self
                .pending_requests
                .iter()
                .any(|req| matches!(req, TerminalRequest::WriteBytes(_)))
    }

    /// Inspect the active terminal geometry.
    pub fn size(&self) -> TerminalSize {
        self.size
    }

    /// Deadline for the next maintenance tick, based on sync mode.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.sync_state.deadline
    }

    pub fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
        self.capture_exit()
    }

    fn process_pending_requests(&mut self) -> Result<()> {
        while let Ok(request) = self.request_rx.try_recv() {
            self.pending_requests.push_back(request);
        }

        while let Some(request) = self.pending_requests.pop_front() {
            self.process_request(request)?;
        }
        Ok(())
    }

    pub fn process_request(&mut self, request: TerminalRequest) -> Result<()> {
        use TerminalRequest::*;

        match request {
            WriteBytes(bytes) => {
                self.enqueue_input(bytes);
                self.flush_pending_input()?;
            },
            Resize(size) => self.resize(size)?,
            ScrollDisplay(direction) => {
                self.surface.scroll_display(direction);
                self.emit_frame()?;
            },
            StartSelection {
                ty,
                point,
                direction,
            } => {
                self.surface.start_selection(ty, point, direction);
                self.emit_frame()?;
            },
            UpdateSelection { point, direction } => {
                self.surface.update_selection(point, direction);
                self.emit_frame()?;
            },
            Shutdown => {
                let _ = self.close();
            },
        }

        Ok(())
    }

    /// Flush buffered output into the PTY session.
    fn flush_pending_input(&mut self) -> Result<()> {
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
    fn resize(&mut self, size: TerminalSize) -> Result<()> {
        self.session.resize(size.into())?;
        self.surface.resize(size);
        self.size = size;
        self.emit_frame()
    }

    /// Terminate the session and return the reported exit status code.
    fn close(&mut self) -> Result<i32> {
        let code = self.session.close()?;
        let status = to_exit_status(code);
        self.emit_child_exit(status)?;
        self.exit_status = Some(status);
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

    fn emit_frame(&mut self) -> Result<()> {
        let frame = self.surface.snapshot_owned();
        self.surface.reset_damage();
        self.events.push_back(TerminalEvent::Frame {
            frame: Arc::new(frame),
        });
        Ok(())
    }

    fn emit_child_exit(&mut self, status: ExitStatus) -> Result<()> {
        self.events.push_back(TerminalEvent::ChildExit { status });
        Ok(())
    }

    fn flush_event_queue(&mut self) -> Result<()> {
        while let Some(event) = self.events.pop_front() {
            match self.event_tx.try_send(event) {
                Ok(()) => {},
                Err(err) => match map_send_error(err) {
                    ChannelSendError::Full => {
                        return Err(crate::Error::EventChannelFull);
                    },
                    ChannelSendError::Disconnected => {
                        return Err(crate::Error::EventChannelClosed);
                    },
                },
            }
        }
        Ok(())
    }
}

fn to_exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatusExt::from_raw(code)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatusExt::from_raw(code as u32)
    }
}

#[cfg(unix)]
fn is_session_closed_error(err: &std::io::Error) -> bool {
    const EIO_RAW: i32 = 5;
    err.kind() == ErrorKind::UnexpectedEof
        || err.raw_os_error() == Some(EIO_RAW)
}

#[cfg(not(unix))]
fn is_session_closed_error(err: &std::io::Error) -> bool {
    err.kind() == ErrorKind::UnexpectedEof
}

impl<P, E, S> TerminalEngine<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceActor + SurfaceModel,
{
    /// Register the underlying session with a mio registry.
    pub fn register_session(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: mio::Token,
        child_token: mio::Token,
    ) -> Result<()> {
        self.session
            .register(registry, interest, io_token, child_token)?;
        Ok(())
    }

    /// Update registered interest for the session handles.
    pub fn reregister_session(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: mio::Token,
        child_token: mio::Token,
    ) -> Result<()> {
        self.session
            .reregister(registry, interest, io_token, child_token)?;
        Ok(())
    }

    /// Deregister the session handles from the mio registry.
    pub fn deregister_session(
        &mut self,
        registry: &mio::Registry,
    ) -> Result<()> {
        self.session.deregister(registry)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::surface::{Surface, SurfaceConfig};
    use crate::terminal::channel::ChannelConfig;
    use crate::tests::{
        EioSession, FakeSession, PartialSession, StubParser, assert_frame,
        collect_events, exit_ok,
    };
    use crate::{DefaultParser, Error};

    use super::*;

    #[test]
    fn partial_writes_keep_pending_output_until_drained() -> Result<()> {
        let session = PartialSession::with_behavior(4, true);
        let parser = StubParser::default();
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());

        let (mut engine, _handle, _events) = TerminalEngine::new(
            session,
            parser,
            surface,
            TerminalOptions {
                channel_config: ChannelConfig::bounded(16),
                ..TerminalOptions::default()
            },
        )?;

        engine
            .queue_request(TerminalRequest::WriteBytes(b"abcdefgh".to_vec()))?;

        assert!(engine.has_pending_output());

        let pending_after_first = engine.on_writable()?;
        assert!(pending_after_first);
        assert_eq!(engine.session.writes.len(), 1);
        assert_eq!(engine.session.writes[0], b"abcd");
        assert_eq!(engine.pending_input.len(), 4);
        assert!(engine.has_pending_output());

        engine.session.blocked = false;
        let pending_after_second = engine.on_writable()?;
        assert!(!pending_after_second);
        assert_eq!(engine.session.writes.len(), 2);
        assert_eq!(engine.session.writes[1], b"efgh");
        assert!(engine.pending_input.is_empty());
        assert!(!engine.has_pending_output());

        Ok(())
    }

    #[test]
    fn has_pending_output_includes_queued_write_request() -> Result<()> {
        let session = PartialSession::with_behavior(4, true);
        let parser = StubParser::default();
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());

        let (mut engine, handle, _events) = TerminalEngine::new(
            session,
            parser,
            surface,
            TerminalOptions::default(),
        )?;

        handle
            .send(TerminalRequest::WriteBytes(b"long-payload".to_vec()))
            .expect("request channel open");

        engine.process_pending_requests()?;
        assert!(engine.has_pending_output());

        engine.session.blocked = false;
        engine.on_writable()?;
        engine.session.blocked = false;
        engine.on_writable()?;
        assert!(!engine.has_pending_output());

        Ok(())
    }

    #[test]
    fn emits_frame_before_child_exit() -> Result<()> {
        let session = FakeSession::with_reads(vec![b"data".to_vec()])
            .with_exit(exit_ok());
        let parser = StubParser::with_actions(vec![Action::Print('a')]);
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let options = TerminalOptions {
            channel_config: ChannelConfig::default(),
            ..TerminalOptions::default()
        };
        let (mut engine, _handle, events) =
            TerminalEngine::new(session, parser, surface, options)?;

        engine.on_readable()?;

        let first = events.recv().expect("first event");
        match first {
            TerminalEvent::Frame { frame } => assert_frame(frame),
            _ => panic!("expected frame first"),
        }

        let second = events.recv().expect("second event");
        assert!(matches!(second, TerminalEvent::ChildExit { .. }));

        Ok(())
    }

    #[test]
    fn bounded_event_channel_surfaces_backpressure() {
        let session = FakeSession::with_reads(vec![b"payload".to_vec()])
            .with_exit(exit_ok());
        let parser = StubParser::with_actions(vec![Action::Print('x')]);
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let options = TerminalOptions {
            channel_config: ChannelConfig {
                event_capacity: Some(1),
                request_capacity: None,
            },
            ..TerminalOptions::default()
        };

        let (mut engine, _handle, _events) =
            TerminalEngine::new(session, parser, surface, options)
                .expect("construct engine");

        let err = engine.on_readable().expect_err("channel backpressure");
        assert!(matches!(err, Error::EventChannelFull));
    }

    #[test]
    fn handle_requests_flow_into_engine() -> Result<()> {
        let session = FakeSession::default();
        let parser = StubParser::default();
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let (mut engine, handle, events) = TerminalEngine::new(
            session,
            parser,
            surface,
            TerminalOptions::default(),
        )?;

        handle
            .send(TerminalRequest::Resize(TerminalSize::default()))
            .expect("request channel open");

        engine.tick()?;

        let event = events.recv().expect("frame after resize");
        match event {
            TerminalEvent::Frame { frame } => assert_frame(frame),
            _ => panic!("expected frame"),
        }

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn eio_is_treated_as_exit() -> Result<()> {
        let session = EioSession::with_exit(exit_ok());
        let parser = StubParser::default();
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let (mut engine, _handle, events) = TerminalEngine::new(
            session,
            parser,
            surface,
            TerminalOptions::default(),
        )?;

        let res = engine.on_readable();
        assert!(res.is_ok());

        let event = events.recv().expect("child exit");
        assert!(matches!(event, TerminalEvent::ChildExit { .. }));

        Ok(())
    }

    #[test]
    fn parses_bytes_into_title_event_and_frame() -> anyhow::Result<()> {
        let session =
            FakeSession::with_reads(vec![b"\x1b]0;hello\x07hi".to_vec()]);
        let parser = DefaultParser::default();
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let (mut engine, _handle, events) = TerminalEngine::new(
            session,
            parser,
            surface,
            TerminalOptions::default(),
        )?;

        engine.on_readable()?;

        let collected = collect_events(&events);
        assert!(!collected.is_empty());
        assert!(matches!(
            collected.first(),
            Some(TerminalEvent::TitleChanged { title }) if title == "hello"
        ));
        let frame = match collected.last() {
            Some(TerminalEvent::Frame { frame }) => frame,
            _ => panic!("expected frame event last"),
        };
        let view = frame.view();
        assert!(
            view.cells.len() >= 2,
            "frame should expose visible cells after print"
        );
        assert_eq!(view.cells[0].cell.c, 'h');
        assert_eq!(view.cells[1].cell.c, 'i');

        Ok(())
    }

    #[test]
    fn propagates_action_events_before_frame_delivery() -> anyhow::Result<()> {
        let actions = vec![
            Action::SetWindowTitle("title".to_string()),
            Action::Bell,
            Action::SetCursorShape(CursorShape::Beam),
            Action::SetCursorStyle(Some(CursorStyle {
                shape: CursorShape::Underline,
                blinking: true,
            })),
            Action::SetHyperlink(Some(Hyperlink {
                id: Some("id".into()),
                uri: "https://example.test".into(),
            })),
        ];
        let session = FakeSession::with_reads(vec![b"payload".to_vec()]);
        let parser = StubParser::with_actions(actions);
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let (mut engine, _handle, events) = TerminalEngine::new(
            session,
            parser,
            surface,
            TerminalOptions::default(),
        )?;

        engine.on_readable()?;

        let collected = collect_events(&events);
        assert!(
            matches!(collected.last(), Some(TerminalEvent::Frame { .. })),
            "frame should be emitted after action-driven events"
        );
        assert!(collected.iter().any(|ev| matches!(ev, TerminalEvent::Bell)));
        assert!(collected.iter().any(|ev| matches!(ev, TerminalEvent::CursorShapeChanged { shape } if *shape == CursorShape::Beam)));
        assert!(collected.iter().any(|ev| matches!(ev, TerminalEvent::CursorStyleChanged { style: Some(style) } if style.shape == CursorShape::Underline)));
        assert!(collected.iter().any(|ev| matches!(ev, TerminalEvent::Hyperlink { link: Some(link) } if link.uri == "https://example.test")));
        assert!(matches!(
            collected.first(),
            Some(TerminalEvent::TitleChanged { title }) if title == "title"
        ));

        Ok(())
    }

    #[test]
    fn frame_is_emitted_before_child_exit() -> anyhow::Result<()> {
        let session = FakeSession::with_reads(vec![b"data".to_vec()])
            .with_exit(exit_ok());
        let parser = DefaultParser::default();
        let surface =
            Surface::new(SurfaceConfig::default(), &TerminalSize::default());
        let options = TerminalOptions::default();
        let (mut engine, _handle, events) =
            TerminalEngine::new(session, parser, surface, options)?;

        engine.on_readable()?;

        let first = events.recv().expect("frame before exit");
        assert!(matches!(first, TerminalEvent::Frame { .. }));
        let second = events.recv().expect("child exit after frame");
        assert!(matches!(second, TerminalEvent::ChildExit { .. }));

        Ok(())
    }
}
