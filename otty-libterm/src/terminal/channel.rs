use flume::{
    Receiver, Sender, TryRecvError as FlumeTryRecvError,
    TrySendError as FlumeTrySendError,
};

use crate::terminal::{TerminalEvent, TerminalRequest};

const DEFAULT_WRITE_CHUNK: usize = 4096;

/// Channel sizing options for terminal request/event plumbing.
#[derive(Clone, Debug)]
pub struct ChannelConfig {
    /// Capacity for the event channel (`None` means unbounded).
    pub event_capacity: Option<usize>,
    /// Capacity for the request channel (`None` means unbounded).
    pub request_capacity: Option<usize>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            event_capacity: None,
            request_capacity: None,
        }
    }
}

impl ChannelConfig {
    /// Use the same bounded capacity for requests and events.
    pub fn bounded(capacity: usize) -> Self {
        Self {
            event_capacity: Some(capacity),
            request_capacity: Some(capacity),
        }
    }
}

/// Error returned when sending into a bounded or closed channel fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelSendError {
    Full,
    Disconnected,
}

/// Error returned when receiving from a channel fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelRecvError {
    Disconnected,
}

/// Error returned when a non-blocking receive fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelTryRecvError {
    Empty,
    Disconnected,
}

pub type ChannelSendResult = std::result::Result<(), ChannelSendError>;
pub type ChannelRecvResult<T> = std::result::Result<T, ChannelRecvError>;
pub type ChannelTryRecvResult<T> = std::result::Result<T, ChannelTryRecvError>;

/// Helper for batching/coalescing write requests and chunking large pastes.
pub struct WriteBatcher<'a> {
    handle: &'a TerminalHandle,
    buffer: Vec<u8>,
    chunk_size: usize,
}

impl<'a> WriteBatcher<'a> {
    pub(crate) fn new(handle: &'a TerminalHandle, chunk_size: usize) -> Self {
        Self {
            handle,
            buffer: Vec::new(),
            chunk_size,
        }
    }

    /// Stage additional bytes to be sent on the next flush.
    pub fn push(&mut self, bytes: impl AsRef<[u8]>) {
        self.buffer.extend_from_slice(bytes.as_ref());
    }

    /// Flush the staged bytes in chunks; preserves unsent data on backpressure.
    pub fn flush(&mut self) -> ChannelSendResult {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let mut start = 0;
        while start < self.buffer.len() {
            let end = (start + self.chunk_size).min(self.buffer.len());
            let chunk = &self.buffer[start..end];
            if let Err(err) = self
                .handle
                .send(TerminalRequest::WriteBytes(chunk.to_vec()))
            {
                // Keep remaining bytes so callers can retry once the channel drains.
                self.buffer.drain(0..start);
                return Err(err);
            }
            start = end;
        }

        self.buffer.clear();
        Ok(())
    }
}

/// Cloneable handle for queuing [`TerminalRequest`]s.
#[derive(Clone, Debug)]
pub struct TerminalHandle {
    sender: Sender<TerminalRequest>,
}

impl TerminalHandle {
    pub(crate) fn new(sender: Sender<TerminalRequest>) -> Self {
        Self { sender }
    }

    /// Try to send a request without blocking.
    pub fn send(&self, request: TerminalRequest) -> ChannelSendResult {
        self.sender.try_send(request).map_err(map_send_error)
    }

    /// Send a large payload by chunking it into multiple `WriteBytes` requests.
    pub fn send_bytes_chunked(
        &self,
        bytes: impl AsRef<[u8]>,
        chunk_size: usize,
    ) -> ChannelSendResult {
        let mut batcher = self.batcher_with_chunk_size(chunk_size.max(1));
        batcher.push(bytes);
        batcher.flush()
    }

    /// Create a helper that batches/coalesces writes and flushes in chunks.
    pub fn batcher(&self) -> WriteBatcher<'_> {
        self.batcher_with_chunk_size(DEFAULT_WRITE_CHUNK)
    }

    /// Create a helper with a custom chunk size.
    pub fn batcher_with_chunk_size(
        &self,
        chunk_size: usize,
    ) -> WriteBatcher<'_> {
        let effective_chunk = chunk_size.max(1);
        WriteBatcher::new(self, effective_chunk)
    }

    /// Send a request in an async context.
    pub async fn send_async(
        &self,
        request: TerminalRequest,
    ) -> ChannelSendResult {
        self.sender
            .send_async(request)
            .await
            .map_err(|_| ChannelSendError::Disconnected)
    }
}

/// Receiver for terminal events with sync + async helpers.
#[derive(Debug)]
pub struct TerminalEvents {
    receiver: Receiver<TerminalEvent>,
}

impl TerminalEvents {
    pub(crate) fn new(receiver: Receiver<TerminalEvent>) -> Self {
        Self { receiver }
    }

    /// Blocking receive.
    pub fn recv(&self) -> ChannelRecvResult<TerminalEvent> {
        self.receiver
            .recv()
            .map_err(|_| ChannelRecvError::Disconnected)
    }

    /// Async receive.
    pub async fn recv_async(&self) -> ChannelRecvResult<TerminalEvent> {
        self.receiver
            .recv_async()
            .await
            .map_err(|_| ChannelRecvError::Disconnected)
    }

    /// Non-blocking receive.
    pub fn try_recv(&self) -> ChannelTryRecvResult<TerminalEvent> {
        self.receiver.try_recv().map_err(map_try_recv_error)
    }
}

pub(crate) fn build_channels(
    config: &ChannelConfig,
) -> (
    Sender<TerminalEvent>,
    Receiver<TerminalEvent>,
    Sender<TerminalRequest>,
    Receiver<TerminalRequest>,
) {
    let (event_tx, event_rx) = match config.event_capacity {
        Some(cap) => flume::bounded(cap),
        None => flume::unbounded(),
    };

    let (request_tx, request_rx) = match config.request_capacity {
        Some(cap) => flume::bounded(cap),
        None => flume::unbounded(),
    };

    (event_tx, event_rx, request_tx, request_rx)
}

pub(crate) fn map_send_error<T>(err: FlumeTrySendError<T>) -> ChannelSendError {
    match err {
        FlumeTrySendError::Full(_) => ChannelSendError::Full,
        FlumeTrySendError::Disconnected(_) => ChannelSendError::Disconnected,
    }
}

pub(crate) fn map_try_recv_error(
    err: FlumeTryRecvError,
) -> ChannelTryRecvError {
    match err {
        FlumeTryRecvError::Empty => ChannelTryRecvError::Empty,
        FlumeTryRecvError::Disconnected => ChannelTryRecvError::Disconnected,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, io, process::ExitStatus};

    use anyhow::Result;
    use otty_escape::{Action, EscapeActor, EscapeParser};
    use otty_pty::Session;
    use otty_surface::{Surface, SurfaceConfig};

    use crate::{
        Error, FrameArc, TerminalEngine, TerminalOptions, TerminalSize,
    };

    use super::*;

    #[test]
    fn emits_frame_before_child_exit() -> Result<()> {
        let session = FakeSession::with_reads(vec![b"data".to_vec()])
            .with_exit(exit_status_ok());
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
            .with_exit(exit_status_ok());
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
        let session = EioSession::with_exit(exit_status_ok());
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
    fn batcher_chunks_large_payloads() {
        let (tx, rx) = flume::bounded(10);
        let handle = TerminalHandle::new(tx);
        let mut batcher = handle.batcher_with_chunk_size(3);
        batcher.push(b"abcdef");
        batcher.push(b"ghi");

        batcher.flush().expect("flush succeeds");

        let collected: Vec<Vec<u8>> = (0..3)
            .map(|_| match rx.recv().expect("request available") {
                TerminalRequest::WriteBytes(bytes) => bytes,
                other => panic!("unexpected request: {other:?}"),
            })
            .collect();

        assert_eq!(
            collected,
            vec![b"abc".to_vec(), b"def".to_vec(), b"ghi".to_vec()]
        );
    }

    fn assert_frame(frame: FrameArc) {
        let view = frame.view();
        assert!(view.visible_cell_count > 0);
    }

    #[derive(Default)]
    struct StubParser {
        actions: VecDeque<Action>,
    }

    impl StubParser {
        fn with_actions(actions: Vec<Action>) -> Self {
            Self {
                actions: actions.into(),
            }
        }
    }

    impl EscapeParser for StubParser {
        fn advance<A: EscapeActor>(&mut self, _bytes: &[u8], actor: &mut A) {
            while let Some(action) = self.actions.pop_front() {
                actor.handle(action);
            }
        }
    }

    #[derive(Default)]
    struct FakeSession {
        reads: VecDeque<Vec<u8>>,
        written: Vec<Vec<u8>>,
        exit_status: Option<ExitStatus>,
    }

    impl FakeSession {
        fn with_reads(reads: Vec<Vec<u8>>) -> Self {
            Self {
                reads: reads.into(),
                written: Vec::new(),
                exit_status: None,
            }
        }

        fn with_exit(mut self, status: ExitStatus) -> Self {
            self.exit_status = Some(status);
            self
        }
    }

    impl Session for FakeSession {
        fn read(
            &mut self,
            buf: &mut [u8],
        ) -> Result<usize, otty_pty::SessionError> {
            if let Some(mut chunk) = self.reads.pop_front() {
                let len = chunk.len().min(buf.len());
                buf[..len].copy_from_slice(&chunk[..len]);
                if len < chunk.len() {
                    chunk.drain(0..len);
                    self.reads.push_front(chunk);
                }
                return Ok(len);
            }
            Err(io::Error::from(io::ErrorKind::WouldBlock).into())
        }

        fn write(
            &mut self,
            input: &[u8],
        ) -> Result<usize, otty_pty::SessionError> {
            self.written.push(input.to_vec());
            Ok(input.len())
        }

        fn resize(
            &mut self,
            _size: otty_pty::PtySize,
        ) -> Result<(), otty_pty::SessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<i32, otty_pty::SessionError> {
            Ok(0)
        }

        fn try_get_child_exit_status(
            &mut self,
        ) -> Result<Option<ExitStatus>, otty_pty::SessionError> {
            Ok(self.exit_status)
        }
    }

    #[cfg(unix)]
    struct EioSession {
        exit_status: Option<ExitStatus>,
    }

    #[cfg(unix)]
    impl EioSession {
        fn with_exit(status: ExitStatus) -> Self {
            Self {
                exit_status: Some(status),
            }
        }
    }

    #[cfg(unix)]
    impl Session for EioSession {
        fn read(
            &mut self,
            _buf: &mut [u8],
        ) -> Result<usize, otty_pty::SessionError> {
            Err(io::Error::from_raw_os_error(5).into())
        }

        fn write(
            &mut self,
            _input: &[u8],
        ) -> Result<usize, otty_pty::SessionError> {
            Ok(0)
        }

        fn resize(
            &mut self,
            _size: otty_pty::PtySize,
        ) -> Result<(), otty_pty::SessionError> {
            Ok(())
        }

        fn close(&mut self) -> Result<i32, otty_pty::SessionError> {
            Ok(0)
        }

        fn try_get_child_exit_status(
            &mut self,
        ) -> Result<Option<ExitStatus>, otty_pty::SessionError> {
            Ok(self.exit_status)
        }
    }

    fn exit_status_ok() -> ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            return ExitStatusExt::from_raw(0);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            return ExitStatusExt::from_raw(0);
        }
    }
}
