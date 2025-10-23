use std::time::{Duration, Instant};

/// Interface for creating timeouts and checking their expiry.
///
/// This is internally used by the [`Processor`] to handle synchronized
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

#[derive(Default)]
pub struct StdSyncHandler {
    timeout: Option<Instant>,
}

impl StdSyncHandler {
    /// Synchronized update expiration time.
    #[inline]
    pub fn sync_timeout(&self) -> Option<Instant> {
        self.timeout
    }
}

impl Timeout for StdSyncHandler {
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
