//! What was counted of the tokens one profile spent, by the rules of spec
//! Decision 5: the last good count stays with its time when a later count
//! fails.

use std::time::SystemTime;

use otty_agents::{SourceError, Spend};

use super::freshness::{self, Which};

/// The tokens of one profile: the last good count and when it was made,
/// why the latest count failed, when the latest count ended, and whether a
/// count is on its way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileSpend {
    last_good: Option<(Spend, SystemTime)>,
    failure: Option<SourceError>,
    has_counter: bool,
    is_reading: bool,
    counted_at: Option<SystemTime>,
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

    /// Whether a count is on its way.
    pub(crate) fn is_reading(&self) -> bool {
        self.is_reading
    }

    /// Whether the tokens need a count for `which`.
    pub(crate) fn is_wanted(&self, which: Which) -> bool {
        freshness::is_wanted(self.counted_at, which)
    }

    /// Start a count; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self) -> bool {
        if self.is_reading {
            return false;
        }

        self.is_reading = true;
        true
    }

    /// Take the result of a count that ended at `counted_at`: `None` when
    /// the agent can not count tokens. A count replaces the last good one;
    /// a failure keeps it and says why.
    pub(crate) fn apply(
        &mut self,
        spend: Option<Result<Spend, SourceError>>,
        counted_at: SystemTime,
    ) {
        self.is_reading = false;
        self.counted_at = Some(counted_at);

        match spend {
            Some(Ok(spend)) => {
                self.last_good = Some((spend, counted_at));
                self.failure = None;
                self.has_counter = true;
            },
            Some(Err(error)) => {
                self.failure = Some(error);
                self.has_counter = true;
            },
            None => {
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
            is_reading: false,
            counted_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::Unavailable;

    use super::*;
    use crate::testing::{at, spend_of};

    #[test]
    fn a_count_starts_once_until_it_ends() {
        let mut spend = ProfileSpend::default();

        assert!(spend.start_reading());
        assert!(!spend.start_reading());
        assert!(spend.is_reading());

        spend.apply(Some(Ok(spend_of(10, 300))), at(10));

        assert!(!spend.is_reading());
        assert!(spend.start_reading());
    }

    #[test]
    fn a_failure_keeps_the_last_good_count_and_the_next_count_clears_it() {
        let mut spend = ProfileSpend::default();
        let failed = SourceError::NotAvailable(Unavailable::NoSessionFiles);

        spend.apply(Some(Ok(spend_of(10, 300))), at(10));
        spend.apply(Some(Err(failed.clone())), at(20));
        let kept = spend.last_good().map(|(spend, at)| (spend.clone(), at));
        let failure = spend.failure().cloned();
        spend.apply(Some(Ok(spend_of(20, 400))), at(30));

        assert_eq!(kept, Some((spend_of(10, 300), at(10))));
        assert_eq!(failure, Some(failed));
        assert_eq!(spend.failure(), None);
        assert_eq!(spend.last_good(), Some((&spend_of(20, 400), at(30))));
        assert!(spend.has_counter());
    }

    #[test]
    fn tokens_never_counted_or_counted_long_ago_are_stale() {
        let mut spend = ProfileSpend::default();
        let now = at(10_000);
        let never = spend.is_wanted(Which::Stale(now));

        spend.apply(Some(Ok(spend_of(10, 300))), at(10));
        let old = spend.is_wanted(Which::Stale(now));
        spend.apply(Some(Err(SourceError::NotLoggedIn)), now);

        assert!(never);
        assert!(old);
        assert!(!spend.is_wanted(Which::Stale(now)), "a failure is a count");
        assert!(spend.is_wanted(Which::Every));
        assert!(!spend.is_wanted(Which::WaitingForPrograms));
    }

    #[test]
    fn an_agent_that_can_not_count_has_no_counter() {
        let mut spend = ProfileSpend::default();

        spend.apply(None, at(10));

        assert!(!spend.has_counter());
        assert_eq!(spend.last_good(), None);
    }
}
