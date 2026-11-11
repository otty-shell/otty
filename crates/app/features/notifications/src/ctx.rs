use std::time::Instant;

/// What the notifications feature reads from outside while it reduces: the
/// clock, which the binary reads, so that `reduce` stays plain and tests can
/// pick the time.
#[derive(Debug, Clone, Copy)]
pub struct NotificationsCtx {
    /// The time the intent is handled at.
    pub now: Instant,
}
