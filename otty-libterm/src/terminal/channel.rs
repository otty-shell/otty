use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use flume::{
    Receiver, Sender, TryRecvError as FlumeTryRecvError,
    TrySendError as FlumeTrySendError,
};

use crate::terminal::{TerminalEvent, TerminalRequest};

const DEFAULT_WRITE_CHUNK: usize = 4096;
const DEFAULT_EVENT_CAPACITY: usize = 64;
const DEFAULT_REQUEST_CAPACITY: usize = 256;

/// Channel sizing options for terminal request/event plumbing.
#[derive(Clone, Debug)]
pub struct ChannelConfig {
    /// Capacity for lossless event notifications (`None` means unbounded).
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

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            event_capacity: Some(DEFAULT_EVENT_CAPACITY),
            request_capacity: Some(DEFAULT_REQUEST_CAPACITY),
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

pub(super) enum EventDelivery {
    FrameReady,
    Lossless(TerminalEvent),
}

#[derive(Default)]
pub(super) struct FrameMailbox {
    frame: Mutex<Option<crate::SnapshotArc>>,
    notification_pending: AtomicBool,
    replaced_frames: AtomicU64,
    lossless_depth: AtomicUsize,
    max_lossless_depth: AtomicUsize,
}

impl FrameMailbox {
    fn replace(&self, frame: crate::SnapshotArc) {
        let mut slot = self
            .frame
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.replace(frame).is_some() {
            self.replaced_frames.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn take(&self) -> Option<crate::SnapshotArc> {
        self.frame
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    fn take_unnotified(&self) -> Option<crate::SnapshotArc> {
        if self.notification_pending.load(Ordering::Acquire) {
            return None;
        }

        self.take()
    }

    fn unread_frame_depth(&self) -> usize {
        usize::from(
            self.frame
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_some(),
        )
    }

    fn reserve_lossless_send(&self) -> usize {
        self.lossless_depth.fetch_add(1, Ordering::AcqRel) + 1
    }

    fn record_max_lossless_depth(&self, depth: usize) {
        self.max_lossless_depth.fetch_max(depth, Ordering::Relaxed);
    }

    fn record_lossless_receive(&self) {
        let _ = self.lossless_depth.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |depth| Some(depth.saturating_sub(1)),
        );
    }
}

pub(super) struct TerminalEventSendFailure {
    kind: ChannelSendError,
    event: Option<TerminalEvent>,
}

impl TerminalEventSendFailure {
    pub(super) fn into_parts(
        self,
    ) -> (ChannelSendError, Option<TerminalEvent>) {
        (self.kind, self.event)
    }
}

impl std::fmt::Debug for TerminalEventSendFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TerminalEventSendFailure")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

pub(super) struct TerminalEventSender {
    sender: Sender<EventDelivery>,
    mailbox: Arc<FrameMailbox>,
}

impl TerminalEventSender {
    pub(super) fn try_send(
        &self,
        event: TerminalEvent,
    ) -> std::result::Result<(), TerminalEventSendFailure> {
        let TerminalEvent::Frame { frame } = event else {
            return self.send_lossless(event);
        };

        self.mailbox.replace(frame);
        if self
            .mailbox
            .notification_pending
            .swap(true, Ordering::AcqRel)
        {
            return Ok(());
        }

        match self.sender.try_send(EventDelivery::FrameReady) {
            Ok(()) => Ok(()),
            Err(FlumeTrySendError::Full(_)) => {
                self.mailbox
                    .notification_pending
                    .store(false, Ordering::Release);
                Ok(())
            },
            Err(FlumeTrySendError::Disconnected(_)) => {
                self.mailbox
                    .notification_pending
                    .store(false, Ordering::Release);
                Err(TerminalEventSendFailure {
                    kind: ChannelSendError::Disconnected,
                    event: None,
                })
            },
        }
    }

    fn send_lossless(
        &self,
        event: TerminalEvent,
    ) -> std::result::Result<(), TerminalEventSendFailure> {
        let depth = self.mailbox.reserve_lossless_send();

        match self.sender.try_send(EventDelivery::Lossless(event)) {
            Ok(()) => {
                self.mailbox.record_max_lossless_depth(depth);
                Ok(())
            },
            Err(FlumeTrySendError::Full(EventDelivery::Lossless(event))) => {
                self.mailbox.record_lossless_receive();
                Err(TerminalEventSendFailure {
                    kind: ChannelSendError::Full,
                    event: Some(event),
                })
            },
            Err(FlumeTrySendError::Disconnected(EventDelivery::Lossless(
                event,
            ))) => {
                self.mailbox.record_lossless_receive();
                Err(TerminalEventSendFailure {
                    kind: ChannelSendError::Disconnected,
                    event: Some(event),
                })
            },
            Err(FlumeTrySendError::Full(EventDelivery::FrameReady))
            | Err(FlumeTrySendError::Disconnected(EventDelivery::FrameReady)) =>
            {
                self.mailbox.record_lossless_receive();
                Err(TerminalEventSendFailure {
                    kind: ChannelSendError::Disconnected,
                    event: None,
                })
            },
        }
    }
}

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
#[derive(Clone)]
pub struct TerminalEvents {
    receiver: Arc<Receiver<EventDelivery>>,
    mailbox: Arc<FrameMailbox>,
}

impl std::fmt::Debug for TerminalEvents {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TerminalEvents")
            .field("replaced_frames", &self.replaced_frame_count())
            .finish_non_exhaustive()
    }
}

impl TerminalEvents {
    pub(super) fn new(
        receiver: Receiver<EventDelivery>,
        mailbox: Arc<FrameMailbox>,
    ) -> Self {
        Self {
            receiver: Arc::new(receiver),
            mailbox,
        }
    }

    /// Blocking receive.
    pub fn recv(&self) -> ChannelRecvResult<TerminalEvent> {
        loop {
            if let Some(frame) = self.mailbox.take_unnotified() {
                return Ok(TerminalEvent::Frame { frame });
            }

            let delivery = self
                .receiver
                .recv()
                .map_err(|_| ChannelRecvError::Disconnected)?;
            if let Some(event) = self.resolve_delivery(delivery) {
                return Ok(event);
            }
        }
    }

    /// Async receive.
    pub async fn recv_async(&self) -> ChannelRecvResult<TerminalEvent> {
        loop {
            if let Some(frame) = self.mailbox.take_unnotified() {
                return Ok(TerminalEvent::Frame { frame });
            }

            let delivery = self
                .receiver
                .recv_async()
                .await
                .map_err(|_| ChannelRecvError::Disconnected)?;
            if let Some(event) = self.resolve_delivery(delivery) {
                return Ok(event);
            }
        }
    }

    /// Non-blocking receive.
    pub fn try_recv(&self) -> ChannelTryRecvResult<TerminalEvent> {
        if let Some(frame) = self.mailbox.take_unnotified() {
            return Ok(TerminalEvent::Frame { frame });
        }

        loop {
            let delivery =
                self.receiver.try_recv().map_err(map_try_recv_error)?;
            if let Some(event) = self.resolve_delivery(delivery) {
                return Ok(event);
            }
        }
    }

    /// Return the number of unread frames replaced by a newer revision.
    pub fn replaced_frame_count(&self) -> u64 {
        self.mailbox.replaced_frames.load(Ordering::Relaxed)
    }

    /// Return the current replaceable frame mailbox depth (zero or one).
    pub fn unread_frame_depth(&self) -> usize {
        self.mailbox.unread_frame_depth()
    }

    /// Return the current number of queued lossless events.
    pub fn lossless_queue_depth(&self) -> usize {
        self.mailbox.lossless_depth.load(Ordering::Acquire)
    }

    /// Return the largest observed lossless queue depth.
    pub fn max_lossless_queue_depth(&self) -> usize {
        self.mailbox.max_lossless_depth.load(Ordering::Relaxed)
    }

    fn resolve_delivery(
        &self,
        delivery: EventDelivery,
    ) -> Option<TerminalEvent> {
        match delivery {
            EventDelivery::Lossless(event) => {
                self.mailbox.record_lossless_receive();
                Some(event)
            },
            EventDelivery::FrameReady => {
                self.mailbox
                    .notification_pending
                    .store(false, Ordering::Release);
                self.mailbox
                    .take()
                    .map(|frame| TerminalEvent::Frame { frame })
            },
        }
    }
}

pub(super) fn build_channels(
    config: &ChannelConfig,
) -> (
    TerminalEventSender,
    Receiver<EventDelivery>,
    Arc<FrameMailbox>,
    Sender<TerminalRequest>,
    Receiver<TerminalRequest>,
) {
    let mailbox = Arc::new(FrameMailbox::default());
    let (event_tx, event_rx) = match config.event_capacity {
        Some(cap) => flume::bounded(cap.max(1)),
        None => flume::unbounded(),
    };

    let (request_tx, request_rx) = match config.request_capacity {
        Some(cap) => flume::bounded(cap),
        None => flume::unbounded(),
    };

    (
        TerminalEventSender {
            sender: event_tx,
            mailbox: mailbox.clone(),
        },
        event_rx,
        mailbox,
        request_tx,
        request_rx,
    )
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

    fn successful_exit() -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(0)
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(0)
        }
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
    fn default_channels_are_bounded() {
        let config = ChannelConfig::default();

        assert!(config.event_capacity.is_some());
        assert!(config.request_capacity.is_some());
    }

    #[test]
    fn unread_frame_is_replaced_while_child_exit_remains_lossless() {
        let config = ChannelConfig::bounded(4);
        let (event_tx, event_rx, mailbox, ..) = build_channels(&config);
        let events = TerminalEvents::new(event_rx, mailbox);
        let first = Arc::new(crate::surface::SnapshotOwned::default());
        let second = Arc::new(crate::surface::SnapshotOwned::default());

        event_tx
            .try_send(TerminalEvent::Frame {
                frame: first.clone(),
            })
            .expect("first frame");
        event_tx
            .try_send(TerminalEvent::Frame {
                frame: second.clone(),
            })
            .expect("replacement frame");
        event_tx
            .try_send(TerminalEvent::ChildExit {
                status: successful_exit(),
            })
            .expect("lossless exit");

        assert_eq!(events.unread_frame_depth(), 1);
        assert_eq!(events.lossless_queue_depth(), 1);
        assert_eq!(events.max_lossless_queue_depth(), 1);

        let received = events.recv().expect("latest frame");
        assert!(matches!(
            received,
            TerminalEvent::Frame { frame } if Arc::ptr_eq(&frame, &second)
        ));
        assert!(matches!(
            events.recv().expect("child exit"),
            TerminalEvent::ChildExit { .. }
        ));
        assert!(matches!(events.try_recv(), Err(ChannelTryRecvError::Empty)));
        assert_eq!(events.replaced_frame_count(), 1);
        assert_eq!(events.unread_frame_depth(), 0);
        assert_eq!(events.lossless_queue_depth(), 0);
    }

    #[test]
    fn frame_remains_available_when_lossless_queue_is_full() {
        let config = ChannelConfig::bounded(1);
        let (event_tx, event_rx, mailbox, ..) = build_channels(&config);
        let events = TerminalEvents::new(event_rx, mailbox);
        let frame = Arc::new(crate::surface::SnapshotOwned::default());

        event_tx
            .try_send(TerminalEvent::TitleChanged {
                title: String::from("title"),
            })
            .expect("fill lossless queue");
        event_tx
            .try_send(TerminalEvent::Frame {
                frame: frame.clone(),
            })
            .expect("frame mailbox does not depend on queue capacity");

        assert!(matches!(
            events.try_recv().expect("mailbox frame"),
            TerminalEvent::Frame { frame: received }
                if Arc::ptr_eq(&received, &frame)
        ));
        assert!(matches!(
            events.try_recv().expect("queued title"),
            TerminalEvent::TitleChanged { title } if title == "title"
        ));
    }

    #[test]
    fn lossless_depth_tracks_current_and_peak_queue_usage() {
        let config = ChannelConfig::bounded(4);
        let (event_tx, event_rx, mailbox, ..) = build_channels(&config);
        let events = TerminalEvents::new(event_rx, mailbox);

        for title in ["first", "second"] {
            event_tx
                .try_send(TerminalEvent::TitleChanged {
                    title: title.to_string(),
                })
                .expect("lossless event");
        }

        assert_eq!(events.lossless_queue_depth(), 2);
        assert_eq!(events.max_lossless_queue_depth(), 2);
        let _ = events.recv().expect("first lossless event");
        assert_eq!(events.lossless_queue_depth(), 1);
        let _ = events.recv().expect("second lossless event");
        assert_eq!(events.lossless_queue_depth(), 0);
        assert_eq!(events.max_lossless_queue_depth(), 2);
    }

    #[test]
    fn full_and_disconnected_lossless_sends_return_the_unsent_event() {
        let config = ChannelConfig::bounded(1);
        let (event_tx, event_rx, mailbox, ..) = build_channels(&config);
        let events = TerminalEvents::new(event_rx, mailbox);
        event_tx
            .try_send(TerminalEvent::TitleChanged {
                title: String::from("first"),
            })
            .expect("first lossless event");

        let failure = event_tx
            .try_send(TerminalEvent::ChildExit {
                status: successful_exit(),
            })
            .expect_err("bounded lossless queue should report backpressure");
        let (kind, event) = failure.into_parts();
        assert_eq!(kind, ChannelSendError::Full);
        assert!(matches!(event, Some(TerminalEvent::ChildExit { .. })));

        drop(events);
        let failure = event_tx
            .try_send(TerminalEvent::TitleChanged {
                title: String::from("disconnected"),
            })
            .expect_err("closed lossless queue");
        let (kind, event) = failure.into_parts();
        assert_eq!(kind, ChannelSendError::Disconnected);
        assert!(matches!(
            event,
            Some(TerminalEvent::TitleChanged { title })
                if title == "disconnected"
        ));
    }

    #[tokio::test]
    async fn async_receive_resolves_frame_notification() {
        let config = ChannelConfig::bounded(1);
        let (event_tx, event_rx, mailbox, ..) = build_channels(&config);
        let events = TerminalEvents::new(event_rx, mailbox);
        let frame = Arc::new(crate::surface::SnapshotOwned::default());
        event_tx
            .try_send(TerminalEvent::Frame {
                frame: frame.clone(),
            })
            .expect("frame");

        assert!(matches!(
            events.recv_async().await.expect("async frame"),
            TerminalEvent::Frame { frame: received }
                if Arc::ptr_eq(&received, &frame)
        ));
    }
}
