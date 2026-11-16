//! What was read of the real bill of one API key profile (spec Decision
//! 9). The bill is the first source of the money: when its latest read
//! failed, the estimate answers instead (spec Decision 5).

use otty_agents::{Bill, SourceError};

/// The bill of one profile: the answer of its latest read, and whether a
/// read is on its way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProfileBill {
    latest: Option<Result<Bill, SourceError>>,
    is_reading: bool,
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

    /// Start a read; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self) -> bool {
        if self.is_reading {
            return false;
        }

        self.is_reading = true;
        true
    }

    /// Take the answer of a read, which replaces the one before.
    pub(crate) fn apply(&mut self, bill: Result<Bill, SourceError>) {
        self.is_reading = false;
        self.latest = Some(bill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::bill_of;

    #[test]
    fn a_read_starts_once_until_it_ends() {
        let mut bill = ProfileBill::default();

        assert!(bill.start_reading());
        assert!(!bill.start_reading());

        bill.apply(Ok(bill_of(1, 2)));

        assert!(bill.start_reading());
    }

    #[test]
    fn a_failed_read_leaves_no_bill_and_says_why() {
        let mut bill = ProfileBill::default();

        bill.apply(Ok(bill_of(1, 2)));
        let answered = bill.answer().cloned();
        bill.apply(Err(SourceError::KeyRefused));

        assert_eq!(answered, Some(bill_of(1, 2)));
        assert_eq!(bill.answer(), None);
        assert_eq!(bill.failure(), Some(&SourceError::KeyRefused));
    }

    #[test]
    fn nothing_is_known_before_the_first_read() {
        let bill = ProfileBill::default();

        assert_eq!(bill.answer(), None);
        assert_eq!(bill.failure(), None);
    }
}
