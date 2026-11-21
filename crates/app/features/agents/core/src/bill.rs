//! What was read of the real bill of one API key profile (spec Decision
//! 9). The bill is the first source of the money: when its latest read
//! failed, the estimate answers instead (spec Decision 5).

use std::time::SystemTime;

use otty_agents::{Bill, SourceError, Unavailable};

use super::freshness::ReadState;

/// The bill of one profile: the answer of its latest read and the
/// bookkeeping of its reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProfileBill {
    latest: Option<Result<Bill, SourceError>>,
    reads: ReadState,
}

impl ProfileBill {
    /// The bill, when the latest read gave one.
    pub(crate) fn answer(&self) -> Option<&Bill> {
        self.latest.as_ref()?.as_ref().ok()
    }

    /// Why the latest read failed, when it did.
    pub(crate) fn failure(&self) -> Option<&SourceError> {
        self.latest.as_ref()?.as_ref().err()
    }

    /// Whether a read is on its way and when the latest one ended.
    pub(crate) fn reads(&self) -> &ReadState {
        &self.reads
    }

    /// Start a read; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self) -> bool {
        self.reads.start()
    }

    /// Take the answer of a read that ended at `read_at`, which replaces
    /// the one before. A login with no key at its place now has no bill
    /// and no failure (issue 13 of spec 2026-11-19-agent-plugins).
    pub(crate) fn apply(
        &mut self,
        bill: Result<Bill, SourceError>,
        read_at: SystemTime,
    ) {
        self.reads.end_at(read_at);

        self.latest = match bill {
            Err(SourceError::NotAvailable(Unavailable::NoLoginKey)) => None,
            bill => Some(bill),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{at, bill_of};

    #[test]
    fn a_failed_read_leaves_no_bill_and_says_why() {
        let mut bill = ProfileBill::default();

        bill.apply(Ok(bill_of(1, 2)), at(10));
        let answered = bill.answer().cloned();
        bill.apply(Err(crate::testing::refused_key_need()), at(20));

        assert_eq!(answered, Some(bill_of(1, 2)));
        assert_eq!(bill.answer(), None);
        assert_eq!(bill.failure(), Some(&crate::testing::refused_key_need()));
        assert_eq!(bill.reads().read_at(), Some(at(20)));
    }

    /// Issue 13 of spec 2026-11-19-agent-plugins: a login whose key is not
    /// at its place now reads no bill and has no failure to show; the
    /// estimate answers.
    #[test]
    fn a_login_without_its_key_has_no_bill_and_no_failure() {
        let mut bill = ProfileBill::default();
        let no_key = Err(SourceError::NotAvailable(
            otty_agents::Unavailable::NoLoginKey,
        ));

        bill.apply(Ok(bill_of(1, 2)), at(10));
        bill.apply(no_key, at(20));

        assert_eq!(bill.answer(), None);
        assert_eq!(bill.failure(), None);
        assert_eq!(bill.reads().read_at(), Some(at(20)));
    }

    #[test]
    fn nothing_is_known_before_the_first_read() {
        let bill = ProfileBill::default();

        assert_eq!(bill.answer(), None);
        assert_eq!(bill.failure(), None);
        assert_eq!(bill.reads().read_at(), None);
    }
}
