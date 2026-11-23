//! The limits of one subscription, as the reads of its numbers left them
//! (spec Decision 5): the last good answer stays with its time when a
//! later read fails. They belong to the account, never to a profile (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 1).

use std::time::SystemTime;

use otty_agents::{Answer, Limits, PartNeed, SourceError, SourceFailure};

use super::types::LimitsRead;

/// The limits of one subscription: the last good answer and when it was
/// read, why the latest read failed, whether anything reads them, and
/// when the latest read ended.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AccountLimits {
    last_good: Option<(Answer<Limits>, SystemTime)>,
    failures: Option<Vec<SourceFailure>>,
    has_sources: bool,
    read_at: Option<SystemTime>,
}

impl AccountLimits {
    /// The last answer of a part and when it was read; it stays when a
    /// later read fails.
    pub(crate) fn last_good(&self) -> Option<(&Answer<Limits>, SystemTime)> {
        self.last_good
            .as_ref()
            .map(|(answer, read_at)| (answer, *read_at))
    }

    /// Why the part failed at the latest read, when it gave no limits.
    pub(crate) fn failures(&self) -> Option<&[SourceFailure]> {
        self.failures.as_deref()
    }

    /// Whether anything reads the limits. `false` once a read found that
    /// no provider part reads them: the mode then draws no limits.
    pub(crate) fn has_sources(&self) -> bool {
        self.has_sources
    }

    /// When the latest read ended; `None` before the first one.
    pub(crate) fn read_at(&self) -> Option<SystemTime> {
        self.read_at
    }

    /// What the provider part asks the user to do when it gave no limits
    /// at the latest read, such as log in again (spec
    /// 2026-11-19-agent-plugins, issue 20).
    pub(crate) fn needs(&self) -> Vec<&PartNeed> {
        let Some(failures) = &self.failures else {
            return Vec::new();
        };

        failures.iter().filter_map(need_of).collect()
    }

    /// Take the result of a read that ended at `read_at`. An answer
    /// replaces the last good one. A failure keeps it and says why.
    pub(crate) fn apply(&mut self, limits: LimitsRead, read_at: SystemTime) {
        self.read_at = Some(read_at);

        match limits {
            LimitsRead::Answered(answer) => {
                self.last_good = Some((answer, read_at));
                self.failures = None;
                self.has_sources = true;
            },
            LimitsRead::Failed(failures) => {
                self.failures = Some(failures);
                self.has_sources = true;
            },
            LimitsRead::NoSources => {
                self.failures = None;
                self.has_sources = false;
            },
        }
    }
}

impl Default for AccountLimits {
    /// Nothing read yet, of a subscription whose limits something may
    /// read.
    fn default() -> Self {
        Self {
            last_good: None,
            failures: None,
            has_sources: true,
            read_at: None,
        }
    }
}

/// The need of the part that gave `failure`, when its read waits for the
/// user.
fn need_of(failure: &SourceFailure) -> Option<&PartNeed> {
    match failure.error() {
        SourceError::NeedsUser(need) => Some(need.as_ref()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SourceId;

    use super::*;
    use crate::testing::{at, five_hour_limits, limits_numbers};

    /// The limits the provider part answered with `percent` used.
    fn from_provider(percent: f64) -> LimitsRead {
        let otty_agents::AccountNumbers::Limits(answer) =
            limits_numbers(five_hour_limits(percent))
        else {
            panic!("the part answers with limits");
        };

        LimitsRead::Answered(answer)
    }

    /// A read the provider part failed with `error`.
    fn failed(error: SourceError) -> LimitsRead {
        LimitsRead::Failed(vec![SourceFailure::new(
            SourceId::ProviderPart,
            error,
        )])
    }

    #[test]
    fn an_answer_is_kept_with_its_time() {
        let mut limits = AccountLimits::default();
        let answer = from_provider(42.0);

        limits.apply(answer.clone(), at(10));

        let LimitsRead::Answered(expected) = answer else {
            panic!("the part answers");
        };
        assert_eq!(limits.last_good(), Some((&expected, at(10))));
        assert_eq!(limits.failures(), None);
        assert_eq!(limits.read_at(), Some(at(10)));
    }

    #[test]
    fn a_failure_keeps_the_last_good_answer_and_the_next_answer_clears_it() {
        let mut limits = AccountLimits::default();
        let good = from_provider(42.0);
        let expired = failed(crate::testing::login_expired());

        limits.apply(good.clone(), at(10));
        limits.apply(expired.clone(), at(20));
        let kept = limits
            .last_good()
            .map(|(answer, read_at)| (answer.clone(), read_at));
        let failures = limits.failures().map(<[SourceFailure]>::to_vec);
        let needs = limits.needs().len();
        limits.apply(from_provider(50.0), at(30));

        let LimitsRead::Answered(good) = good else {
            panic!("the part answers");
        };
        let LimitsRead::Failed(expired) = expired else {
            panic!("the part fails");
        };
        assert_eq!(kept, Some((good, at(10))));
        assert_eq!(failures, Some(expired));
        assert_eq!(needs, 1, "the expired login asks the user");
        assert_eq!(limits.failures(), None);
        assert_eq!(
            limits.last_good().map(|(_, read_at)| read_at),
            Some(at(30))
        );
        assert!(limits.has_sources());
    }

    /// Issue 20: the needs of the part show only while it gave no limits.
    #[test]
    fn only_a_part_that_waits_for_the_user_has_a_need() {
        let mut limits = AccountLimits::default();

        limits
            .apply(failed(SourceError::Network(String::from("down"))), at(10));

        assert!(limits.needs().is_empty());
        assert!(AccountLimits::default().needs().is_empty());
    }

    #[test]
    fn no_sources_means_nothing_reads_the_limits() {
        let mut limits = AccountLimits::default();

        limits.apply(LimitsRead::NoSources, at(10));

        assert!(!limits.has_sources());
        assert_eq!(limits.failures(), None);
        assert_eq!(limits.last_good(), None);
    }
}
