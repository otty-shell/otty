use std::sync::Arc;

use flume::{
    Receiver, Sender, TryRecvError as FlumeTryRecvError,
    TrySendError as FlumeTrySendError,
};

use crate::surface::SnapshotOwned;
use crate::terminal::latest_snapshot::LatestSnapshot;
use crate::terminal::{TerminalEvent, TerminalRequest};

const DEFAULT_WRITE_CHUNK: usize = 4096;

/// Channel sizing options for terminal request/event plumbing.
#[derive(Default, Clone, Debug)]
pub struct ChannelConfig {
    /// Capacity for the event channel (`None` means unbounded).
    pub event_capacity: Option<usize>,
    /// Capacity for the request channel (`None` means unbounded).
    pub request_capacity: Option<usize>,
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

/// Cloneable handle to a terminal: it queues [`TerminalRequest`]s and hands
/// out the newest snapshot.
#[derive(Clone, Debug)]
pub struct TerminalHandle {
    sender: Sender<TerminalRequest>,
    latest_snapshot: LatestSnapshot,
}

impl TerminalHandle {
    pub(crate) fn new(sender: Sender<TerminalRequest>) -> Self {
        Self {
            sender,
            latest_snapshot: LatestSnapshot::default(),
        }
    }

    /// Returns the snapshot slot the terminal publishes into; clones share it.
    pub(crate) fn latest_snapshot(&self) -> LatestSnapshot {
        self.latest_snapshot.clone()
    }

    /// Take the newest snapshot, if nobody took it yet.
    ///
    /// A snapshot is a full screen copy, so the terminal keeps only the newest
    /// one and tells about it with [`TerminalEvent::SnapshotReady`]. `None`
    /// means no snapshot is waiting: for example two notifications came in a
    /// row and the first take already got the newest snapshot.
    ///
    /// The caller owns the taken snapshot. Clones of a handle take from one
    /// place, so a snapshot goes to whoever takes it first.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_libterm::{TerminalEvent, TerminalEvents, TerminalHandle};
    ///
    /// fn draw_newest(events: &TerminalEvents, handle: &TerminalHandle) {
    ///     while let Ok(event) = events.try_recv() {
    ///         if !matches!(event, TerminalEvent::SnapshotReady) {
    ///             continue;
    ///         }
    ///
    ///         if let Some(snapshot) = handle.take_snapshot() {
    ///             let cells = snapshot.view().cells;
    ///             println!("drawing {} cells", cells.len());
    ///         }
    ///     }
    /// }
    /// ```
    pub fn take_snapshot(&self) -> Option<SnapshotOwned> {
        self.latest_snapshot.take()
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
#[derive(Debug, Clone)]
pub struct TerminalEvents {
    receiver: Arc<Receiver<TerminalEvent>>,
}

impl TerminalEvents {
    pub(crate) fn new(receiver: Receiver<TerminalEvent>) -> Self {
        Self {
            receiver: Arc::new(receiver),
        }
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
    use super::*;
    use crate::TerminalSize;
    use crate::surface::{SnapshotOwned, Surface, SurfaceConfig, SurfaceModel};

    /// A snapshot as wide as `columns`, to tell snapshots apart.
    fn snapshot(columns: usize) -> SnapshotOwned {
        let size = TerminalSize {
            cols: columns as u16,
            ..TerminalSize::default()
        };

        Surface::new(SurfaceConfig::default(), &size).snapshot_owned()
    }

    fn handle_with_slot() -> (TerminalHandle, LatestSnapshot) {
        let (tx, _rx) = flume::unbounded();
        let handle = TerminalHandle::new(tx);
        let latest_snapshot = handle.latest_snapshot();

        (handle, latest_snapshot)
    }

    fn events() -> (Sender<TerminalEvent>, TerminalEvents) {
        let (tx, rx) = flume::unbounded();

        (tx, TerminalEvents::new(rx))
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

    #[test]
    fn take_snapshot_finds_nothing_before_the_first_snapshot() {
        let (handle, _latest_snapshot) = handle_with_slot();

        assert!(handle.take_snapshot().is_none());
    }

    #[test]
    fn take_snapshot_returns_the_newest_snapshot_once() {
        let (handle, latest_snapshot) = handle_with_slot();
        latest_snapshot.publish(snapshot(80));
        latest_snapshot.publish(snapshot(120));

        let taken = handle.take_snapshot().expect("a snapshot is waiting");

        assert_eq!(taken.view().size.columns, 120);
        assert!(handle.take_snapshot().is_none());
    }

    #[test]
    fn clones_of_a_handle_take_the_snapshot_from_one_place() {
        let (handle, latest_snapshot) = handle_with_slot();
        let other = handle.clone();
        latest_snapshot.publish(snapshot(80));

        assert!(other.take_snapshot().is_some());
        assert!(handle.take_snapshot().is_none());
    }

    #[test]
    fn recv_passes_events_through_in_order() {
        let (tx, events) = events();
        tx.send(TerminalEvent::SnapshotReady).expect("channel open");
        tx.send(TerminalEvent::Bell).expect("channel open");

        let first = events.recv().expect("first event");
        let second = events.recv().expect("second event");

        assert!(matches!(first, TerminalEvent::SnapshotReady));
        assert!(matches!(second, TerminalEvent::Bell));
    }

    #[test]
    fn recv_reports_a_closed_channel() {
        let (tx, events) = events();
        drop(tx);

        assert_eq!(events.recv().err(), Some(ChannelRecvError::Disconnected));
    }

    #[test]
    fn try_recv_tells_an_empty_channel_from_a_closed_one() {
        let (tx, events) = events();
        assert_eq!(events.try_recv().err(), Some(ChannelTryRecvError::Empty));

        tx.send(TerminalEvent::Bell).expect("channel open");
        let event = events.try_recv().expect("bell event");
        assert!(matches!(event, TerminalEvent::Bell));

        drop(tx);
        assert_eq!(
            events.try_recv().err(),
            Some(ChannelTryRecvError::Disconnected)
        );
    }

    #[tokio::test]
    async fn recv_async_passes_events_and_reports_a_closed_channel() {
        let (tx, events) = events();
        tx.send(TerminalEvent::Bell).expect("channel open");

        let event = events.recv_async().await.expect("bell event");
        assert!(matches!(event, TerminalEvent::Bell));

        drop(tx);
        assert_eq!(
            events.recv_async().await.err(),
            Some(ChannelRecvError::Disconnected)
        );
    }
}
