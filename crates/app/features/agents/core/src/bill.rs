//! What was read of the real bill of one API key profile (spec Decision
//! 9). The bill is the first source of the money: when its latest read
//! failed, the estimate answers instead (spec Decision 5).

use std::time::SystemTime;

use otty_agents::{Bill, SourceError};

use super::freshness::{self, Which};

/// The bill of one profile: the answer of its latest read and when it
/// ended, and whether a read is on its way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProfileBill {
    latest: Option<(Result<Bill, SourceError>, SystemTime)>,
    is_reading: bool,
}

impl ProfileBill {
    /// The bill, when the latest read gave one.
    pub(crate) fn answer(&self) -> Option<&Bill> {
        self.latest.as_ref()?.0.as_ref().ok()
    }

    /// Why the latest read failed, when it did.
    pub(crate) fn failure(&self) -> Option<&SourceError> {
        self.latest.as_ref()?.0.as_ref().err()
    }

    /// When the latest read ended.
    pub(crate) fn read_at(&self) -> Option<SystemTime> {
        self.latest.as_ref().map(|(_, read_at)| *read_at)
    }

    /// Whether a read is on its way.
    pub(crate) fn is_reading(&self) -> bool {
        self.is_reading
    }

    /// Whether the bill needs a read for `which`.
    pub(crate) fn is_wanted(&self, which: Which) -> bool {
        freshness::is_wanted(self.read_at(), which)
    }

    /// Start a read; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self) -> bool {
        if self.is_reading {
            return false;
        }

        self.is_reading = true;
        true
    }

    /// Take the answer of a read that ended at `read_at`, which replaces
    /// the one before.
    pub(crate) fn apply(
        &mut self,
        bill: Result<Bill, SourceError>,
        read_at: SystemTime,
    ) {
        self.is_reading = false;
        self.latest = Some((bill, read_at));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{at, bill_of};

    #[test]
    fn a_read_starts_once_until_it_ends() {
        let mut bill = ProfileBill::default();

        assert!(bill.start_reading());
        assert!(!bill.start_reading());

        bill.apply(Ok(bill_of(1, 2)), at(10));

        assert!(bill.start_reading());
    }

    #[test]
    fn a_failed_read_leaves_no_bill_and_says_why() {
        let mut bill = ProfileBill::default();

        bill.apply(Ok(bill_of(1, 2)), at(10));
        let answered = bill.answer().cloned();
        bill.apply(Err(SourceError::KeyRefused), at(20));

        assert_eq!(answered, Some(bill_of(1, 2)));
        assert_eq!(bill.answer(), None);
        assert_eq!(bill.failure(), Some(&SourceError::KeyRefused));
        assert_eq!(bill.read_at(), Some(at(20)));
    }

    #[test]
    fn a_bill_never_read_or_read_long_ago_is_stale() {
        let mut bill = ProfileBill::default();
        let now = at(10_000);
        let never = bill.is_wanted(Which::Stale(now));

        bill.apply(Ok(bill_of(1, 2)), at(10));
        let old = bill.is_wanted(Which::Stale(now));
        bill.apply(Err(SourceError::KeyRefused), now);

        assert!(never);
        assert!(old);
        assert!(!bill.is_wanted(Which::Stale(now)));
        assert!(bill.is_wanted(Which::Every));
        assert!(!bill.is_wanted(Which::WaitingForPrograms));
    }

    #[test]
    fn nothing_is_known_before_the_first_read() {
        let bill = ProfileBill::default();

        assert_eq!(bill.answer(), None);
        assert_eq!(bill.failure(), None);
        assert_eq!(bill.read_at(), None);
    }
}
