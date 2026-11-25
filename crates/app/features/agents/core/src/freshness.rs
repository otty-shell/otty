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
    /// failed: the agent's program, as is or in a hidden terminal, which
    /// did not start then, may answer now.
    WaitingForPrograms,
}

/// The bookkeeping of one kind of read of one profile, or of the prices:
/// whether a read is on its way, and when the latest one ended. One read
/// runs at a time, and what it read stays fresh for [`READ_PERIOD`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ReadState {
    is_reading: bool,
    read_at: Option<SystemTime>,
}

impl ReadState {
    /// Whether a read is on its way.
    pub(crate) fn is_reading(&self) -> bool {
        self.is_reading
    }

    /// When the latest read ended; `None` before the first one.
    pub(crate) fn read_at(&self) -> Option<SystemTime> {
        self.read_at
    }

    /// Whether a new read is wanted for `which`.
    pub(crate) fn is_wanted(&self, which: Which) -> bool {
        is_wanted(self.read_at, which)
    }

    /// Start a read; `false` when one is on its way.
    pub(crate) fn start(&mut self) -> bool {
        if self.is_reading {
            return false;
        }

        self.is_reading = true;
        true
    }

    /// The read on its way ended at `read_at`.
    pub(crate) fn end_at(&mut self, read_at: SystemTime) {
        self.is_reading = false;
        self.read_at = Some(read_at);
    }

    /// The read on its way ended, with no time to keep: the sessions,
    /// which are read again with every read of the profiles.
    pub(crate) fn end(&mut self) {
        self.is_reading = false;
    }
}

/// Whether something read at `read_at` is still fresh at `now`. A time
/// after `now`, from a clock that was set back, counts as fresh.
fn is_fresh(read_at: SystemTime, now: SystemTime) -> bool {
    match now.duration_since(read_at) {
        Ok(age) => age < READ_PERIOD,
        Err(_) => true,
    }
}

/// Whether a kind whose latest read ended at `read_at` (`None`: it was
/// never read) is wanted by `which`. Waiting for the agent's program is
/// not about time: only the limits know it, so it wants nothing here.
fn is_wanted(read_at: Option<SystemTime>, which: Which) -> bool {
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
    fn a_read_starts_once_until_it_ends() {
        let mut reads = ReadState::default();

        assert!(reads.start());
        assert!(!reads.start());
        assert!(reads.is_reading());

        reads.end_at(at(10));

        assert!(!reads.is_reading());
        assert_eq!(reads.read_at(), Some(at(10)));
        assert!(reads.start());
        reads.end();
        assert!(!reads.is_reading());
        assert_eq!(reads.read_at(), Some(at(10)), "an end with no time");
    }

    #[test]
    fn a_read_never_made_or_made_long_ago_is_wanted() {
        let mut reads = ReadState::default();
        let now = at(10_000);
        let never = reads.is_wanted(Which::Stale(now));

        reads.end_at(at(10));
        let old = reads.is_wanted(Which::Stale(now));
        reads.end_at(now);

        assert!(never);
        assert!(old);
        assert!(!reads.is_wanted(Which::Stale(now)));
        assert!(reads.is_wanted(Which::Every));
        assert!(!reads.is_wanted(Which::WaitingForPrograms));
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
