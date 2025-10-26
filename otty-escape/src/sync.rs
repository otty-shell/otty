use std::time::{Duration, Instant};

/// Maximum time before a synchronized update is aborted.
pub(crate) const SYNC_UPDATE_TIMEOUT: Duration = Duration::from_millis(150);

/// Interface for creating timeouts and checking their expiry.
///
/// This is internally used by the [`crate::parser::Parser`] to handle synchronized
/// updates.
pub trait Timeout: Default {
    /// Sets the timeout for the next synchronized update.
    ///
    /// The `duration` parameter specifies the duration of the timeout. Once the
    /// specified duration has elapsed, the synchronized update rotuine can be
    /// performed.
    fn set_timeout(&mut self, duration: Duration);
    /// Clear the current timeout.
    fn clear_timeout(&mut self);
    /// Returns whether a timeout is currently active and has not yet expired.
    fn pending_timeout(&self) -> bool;
}

/// I
#[derive(Default)]
pub struct SyncHandler {
    timeout: Option<Instant>,
}

impl SyncHandler {
    /// Synchronized update expiration time.
    #[inline]
    pub fn sync_timeout(&self) -> Option<Instant> {
        self.timeout
    }
}

impl Timeout for SyncHandler {
    #[inline]
    fn set_timeout(&mut self, duration: Duration) {
        self.timeout = Some(Instant::now() + duration);
    }

    #[inline]
    fn clear_timeout(&mut self) {
        self.timeout = None;
    }

    #[inline]
    fn pending_timeout(&self) -> bool {
        self.timeout.is_some()
    }
}
