//! How long what Agents mode read stays fresh: the period of its timer
//! (spec Decision 10). While the mode is on screen the timer reads the
//! limits again after this period; when the mode opens, only what is older
//! than it is read again.

use std::time::{Duration, SystemTime};

/// How often the limits are read again while Agents mode is on screen, and
/// how long anything the mode read stays fresh.
pub(crate) const READ_PERIOD: Duration = Duration::from_secs(5 * 60);

/// Which profiles get a read of one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    /// Every profile that can have one.
    Every,
    /// The profiles whose latest read of the kind is older than
    /// [`READ_PERIOD`] at this time or that never had one, and the limits
    /// that wait for the agent's program.
    Stale(SystemTime),
    /// Only the limits read while Agents mode was away that a source
    /// failed: the agent's program (`codex app-server`, or `claude` in a
    /// hidden terminal), which did not start then, may answer now.
    WaitingForPrograms,
}

/// Whether something read at `read_at` is still fresh at `now`. A time
/// after `now`, from a clock that was set back, counts as fresh.
pub(crate) fn is_fresh(read_at: SystemTime, now: SystemTime) -> bool {
    match now.duration_since(read_at) {
        Ok(age) => age < READ_PERIOD,
        Err(_) => true,
    }
}

/// Whether a kind whose latest read ended at `read_at` (`None`: it was
/// never read) is wanted by `which`. Waiting for the agent's program is
/// not about time: only the limits know it, so it wants nothing here.
pub(crate) fn is_wanted(read_at: Option<SystemTime>, which: Which) -> bool {
    match which {
        Which::Every => true,
        Which::Stale(now) => {
            !read_at.is_some_and(|read_at| is_fresh(read_at, now))
        },
        Which::WaitingForPrograms => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::at;

    #[test]
    fn a_read_is_fresh_for_the_period_of_the_timer() {
        let read_at = at(1_000);

        assert!(is_fresh(read_at, read_at));
        assert!(is_fresh(read_at, read_at + READ_PERIOD / 2));
        assert!(!is_fresh(read_at, read_at + READ_PERIOD));
        assert!(is_fresh(read_at, at(10)), "a clock set back");
    }

    #[test]
    fn a_stale_read_or_none_is_wanted_and_a_fresh_one_is_not() {
        let now = at(10_000);
        let stale = Which::Stale(now);

        assert!(is_wanted(None, stale));
        assert!(is_wanted(Some(at(10)), stale));
        assert!(!is_wanted(Some(now), stale));
        assert!(is_wanted(Some(now), Which::Every));
        assert!(!is_wanted(None, Which::WaitingForPrograms));
    }
}
