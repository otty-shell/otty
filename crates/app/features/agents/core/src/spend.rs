//! What was counted of the tokens one profile spent, by the rules of spec
//! Decision 5: the last good count stays with its time when a later count
//! fails.

use std::time::SystemTime;

use otty_agents::{SourceError, Spend};

use super::freshness::ReadState;
use super::types::SpendCount;

/// The tokens of one profile: the last good count and when it was made,
/// why the latest count failed, whether the agent can count, and the
/// bookkeeping of the counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileSpend {
    last_good: Option<(Spend, SystemTime)>,
    failure: Option<SourceError>,
    has_counter: bool,
    reads: ReadState,
}

impl ProfileSpend {
    /// The last good count and when it was made; it stays when a later
    /// count fails.
    pub(crate) fn last_good(&self) -> Option<(&Spend, SystemTime)> {
        self.last_good
            .as_ref()
            .map(|(spend, counted_at)| (spend, *counted_at))
    }

    /// Why the latest count failed, when it did.
    pub(crate) fn failure(&self) -> Option<&SourceError> {
        self.failure.as_ref()
    }

    /// Whether the agent can count tokens. `false` once a count found it
    /// can not: the mode then draws no tokens.
    pub(crate) fn has_counter(&self) -> bool {
        self.has_counter
    }

    /// Whether a count is on its way and when the latest one ended.
    pub(crate) fn reads(&self) -> &ReadState {
        &self.reads
    }

    /// Start a count; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self) -> bool {
        self.reads.start()
    }

    /// Take the result of a count that ended at `counted_at`. A count
    /// replaces the last good one; a failure keeps it and says why.
    pub(crate) fn apply(&mut self, spend: SpendCount, counted_at: SystemTime) {
        self.reads.end_at(counted_at);

        match spend {
            SpendCount::Counted(spend) => {
                self.last_good = Some((spend, counted_at));
                self.failure = None;
                self.has_counter = true;
            },
            SpendCount::Failed(error) => {
                self.failure = Some(error);
                self.has_counter = true;
            },
            SpendCount::NoCounter => {
                self.failure = None;
                self.has_counter = false;
            },
        }
    }
}

impl Default for ProfileSpend {
    /// Nothing counted yet, by an agent that may count tokens.
    fn default() -> Self {
        Self {
            last_good: None,
            failure: None,
            has_counter: true,
            reads: ReadState::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::Unavailable;

    use super::*;
    use crate::testing::{at, spend_of};

    #[test]
    fn a_failure_keeps_the_last_good_count_and_the_next_count_clears_it() {
        let mut spend = ProfileSpend::default();
        let failed = SourceError::NotAvailable(Unavailable::NoSessionFiles);

        spend.apply(SpendCount::Counted(spend_of(10, 300)), at(10));
        spend.apply(SpendCount::Failed(failed.clone()), at(20));
        let kept = spend.last_good().map(|(spend, at)| (spend.clone(), at));
        let failure = spend.failure().cloned();
        let failed_at = spend.reads().read_at();
        spend.apply(SpendCount::Counted(spend_of(20, 400)), at(30));

        assert_eq!(kept, Some((spend_of(10, 300), at(10))));
        assert_eq!(failure, Some(failed));
        assert_eq!(failed_at, Some(at(20)), "a failure is a count");
        assert_eq!(spend.failure(), None);
        assert_eq!(spend.last_good(), Some((&spend_of(20, 400), at(30))));
        assert!(spend.has_counter());
    }

    #[test]
    fn an_agent_that_can_not_count_has_no_counter() {
        let mut spend = ProfileSpend::default();

        spend.apply(SpendCount::NoCounter, at(10));

        assert!(!spend.has_counter());
        assert_eq!(spend.last_good(), None);
    }
}
