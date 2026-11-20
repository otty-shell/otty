//! Whether a plugin part still runs (spec 2026-11-19-agent-plugins,
//! Decision 8, "Engine limits"): a trap or a missed deadline fails that
//! read only, and three failed reads in a row pause the part until the
//! user presses Try again or OTTY starts again. The plugin's other parts
//! keep their own count.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// How many failed reads in a row pause a part.
pub const PAUSE_AFTER: u32 = 3;

/// The failed reads in a row of one plugin part. Clones share the count,
/// so the Plugins page can tell a part is paused and start it again.
#[derive(Debug, Clone, Default)]
pub struct PartHealth {
    failures_in_a_row: Arc<AtomicU32>,
}

impl PartHealth {
    /// Whether the part is paused: its last [`PAUSE_AFTER`] reads failed.
    ///
    /// ```
    /// use otty_agents::PartHealth;
    ///
    /// assert!(!PartHealth::default().is_paused());
    /// ```
    pub fn is_paused(&self) -> bool {
        self.failures_in_a_row.load(Ordering::Relaxed) >= PAUSE_AFTER
    }

    /// Starts the part again, as **Try again** does: its count of failed
    /// reads starts from zero.
    pub fn try_again(&self) {
        self.failures_in_a_row.store(0, Ordering::Relaxed);
    }

    /// One read of the part failed with a trap or a missed deadline.
    pub(crate) fn failed(&self) {
        self.failures_in_a_row.fetch_add(1, Ordering::Relaxed);
    }

    /// One read of the part answered.
    pub(crate) fn answered(&self) {
        self.failures_in_a_row.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_failed_reads_in_a_row_pause_the_part() {
        let health = PartHealth::default();

        health.failed();
        health.failed();
        let after_two = health.is_paused();
        health.failed();

        assert!(!after_two);
        assert!(health.is_paused());
    }

    #[test]
    fn an_answer_between_failures_starts_the_count_again() {
        let health = PartHealth::default();

        health.failed();
        health.failed();
        health.answered();
        health.failed();
        health.failed();

        assert!(!health.is_paused());
    }

    #[test]
    fn try_again_starts_a_paused_part_and_clones_share_the_count() {
        let health = PartHealth::default();
        let page = health.clone();
        for _ in 0..PAUSE_AFTER {
            health.failed();
        }

        let paused = page.is_paused();
        page.try_again();

        assert!(paused);
        assert!(!health.is_paused());
    }
}
