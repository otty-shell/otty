//! Whether a plugin part still runs (spec 2026-11-19-agent-plugins,
//! Decision 8, "Engine limits"): a trap or a missed deadline fails that
//! read only, and three failed reads in a row pause the part until the
//! user presses Try again or OTTY starts again. The plugin's other parts
//! keep their own count. The last read of the part that failed is kept
//! for its card on the Plugins page (issue 19), where plugin authors look.
//! It is the same for a part of any domain (issue 21).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use crate::plugin_failure::PluginFailure;

/// How many failed reads in a row pause a part.
pub const PAUSE_AFTER: u32 = 3;

/// The failed reads in a row of one plugin part and its last failure.
/// Clones share both, so the Plugins page can tell a part is paused,
/// start it again and show what failed last.
#[derive(Debug, Clone, Default)]
pub struct PartHealth {
    failures_in_a_row: Arc<AtomicU32>,
    last_failure: Arc<Mutex<Option<LastFailure>>>,
}

impl PartHealth {
    /// Whether the part is paused: its last [`PAUSE_AFTER`] reads failed.
    ///
    /// ```
    /// use otty_plugin_host::PartHealth;
    ///
    /// assert!(!PartHealth::default().is_paused());
    /// ```
    pub fn is_paused(&self) -> bool {
        self.failures_in_a_row.load(Ordering::Relaxed) >= PAUSE_AFTER
    }

    /// The last call of the part that failed or that the host refused,
    /// with its time; `None` while none did. It stays after later reads
    /// answer.
    ///
    /// ```
    /// use otty_plugin_host::PartHealth;
    ///
    /// assert!(PartHealth::default().last_failure().is_none());
    /// ```
    pub fn last_failure(&self) -> Option<LastFailure> {
        self.last_failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
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

    /// Keeps `failure`, which happened at `at`, as the last failure.
    pub(crate) fn remember_failure(
        &self,
        failure: &PluginFailure,
        at: SystemTime,
    ) {
        let mut last = self
            .last_failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        *last = Some(LastFailure {
            failure: failure.clone(),
            at,
        });
    }
}

/// The last call of a plugin part that failed: the call and the kind of
/// error, never a value, a key or session text, and when it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastFailure {
    failure: PluginFailure,
    at: SystemTime,
}

impl LastFailure {
    /// The failure: the part, the call and the kind, with the report for
    /// the plugin's author.
    pub fn failure(&self) -> &PluginFailure {
        &self.failure
    }

    /// When it failed.
    pub fn at(&self) -> SystemTime {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::plugin_failure::PluginFailureKind;
    use crate::plugin_part::PluginPart;

    fn failure(call: &'static str, kind: PluginFailureKind) -> PluginFailure {
        let part = PluginPart::new(
            "example",
            "Example",
            "0.1.0",
            "https://github.com/otty-shell/otty-plugin-example",
            "example-agent",
        );

        PluginFailure::new(part, call, kind)
    }

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

    /// Issue 19: the card of the part on the Plugins page shows the last
    /// failure, the newest one, with its time; an answer after it keeps it.
    #[test]
    fn the_last_failure_is_kept_with_its_time_and_clones_share_it() {
        let health = PartHealth::default();
        let page = health.clone();
        let first = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        let second = SystemTime::UNIX_EPOCH + Duration::from_secs(20);
        let late = failure("read-limits", PluginFailureKind::TooSlow);

        health.remember_failure(
            &failure("sessions", PluginFailureKind::Stopped),
            first,
        );
        health.remember_failure(&late, second);
        health.answered();

        let last = page.last_failure().expect("a failure");
        assert_eq!(last.failure(), &late);
        assert_eq!(last.at(), second);
    }
}
