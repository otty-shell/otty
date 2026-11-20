//! What was read of the limits of one profile, by the rules of spec
//! Decision 5: the last good answer stays with its time when a later read
//! fails.

use std::time::SystemTime;

use otty_agents::{Answer, Limits, SourceError, SourceFailure, SourceId};

use super::freshness::{ReadState, Which};
use super::types::LimitsRead;

/// The limits of one profile: the last good answer and when it was read,
/// why the latest read failed, the bookkeeping of the reads, and whether a
/// read on its way may start the agent's program, as is or in a hidden
/// terminal.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileLimits {
    last_good: Option<(Answer<Limits>, SystemTime)>,
    failures: Option<Vec<SourceFailure>>,
    has_sources: bool,
    reads: ReadState,
    may_start_programs: bool,
    waits_for_programs: bool,
}

impl ProfileLimits {
    /// The last answer of a part and when it was read; it stays when a
    /// later read fails.
    pub(crate) fn last_good(&self) -> Option<(&Answer<Limits>, SystemTime)> {
        self.last_good
            .as_ref()
            .map(|(answer, read_at)| (answer, *read_at))
    }

    /// Why each part failed at the latest read, when no part answered.
    pub(crate) fn failures(&self) -> Option<&[SourceFailure]> {
        self.failures.as_deref()
    }

    /// Whether anything reads the limits of the profile. `false` once a
    /// read found that neither a provider part nor the agent part reads
    /// them: the mode then draws no limits.
    pub(crate) fn has_sources(&self) -> bool {
        self.has_sources
    }

    /// Whether a read is on its way and when the latest one ended.
    pub(crate) fn reads(&self) -> &ReadState {
        &self.reads
    }

    /// Whether the login of the account expired (spec 2026-11-19,
    /// Decision 3): no part answered at the latest read, and one of them
    /// said the login is missing or expired. OTTY never renews it; the
    /// agent does when it runs, or asks to log in again.
    pub(crate) fn is_login_expired(&self) -> bool {
        let Some(failures) = &self.failures else {
            return false;
        };

        failures
            .iter()
            .any(|failure| matches!(failure.error(), SourceError::NotLoggedIn))
    }

    /// Whether the limits need a read for `which`: unlike
    /// [`ReadState::is_wanted`], limits that wait for the agent's program
    /// are stale whatever their time.
    pub(crate) fn is_wanted(&self, which: Which) -> bool {
        match which {
            Which::WaitingForPrograms => self.waits_for_programs,
            Which::Stale(_) => {
                self.waits_for_programs || self.reads.is_wanted(which)
            },
            Which::Every => true,
        }
    }

    /// Start a read, which may start the agent's program when
    /// `may_start_programs`; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self, may_start_programs: bool) -> bool {
        if !self.reads.start() {
            return false;
        }

        self.may_start_programs = may_start_programs;
        true
    }

    /// Take the result of a read that ended at `read_at`. An answer
    /// replaces the last good one. A failure keeps it and says why. A read
    /// that could not start the agent's program waits for it unless the
    /// provider part answered: the agent part, which may read the
    /// program, may answer better with it, or answer at all.
    pub(crate) fn apply(&mut self, limits: LimitsRead, read_at: SystemTime) {
        let is_provider_answer = matches!(
            &limits,
            LimitsRead::Answered(answer)
                if answer.source() == SourceId::ProviderPart
        );
        let agent_part_was_asked = match &limits {
            LimitsRead::Answered(_) => !is_provider_answer,
            LimitsRead::Failed(_) => true,
            LimitsRead::NoSources => false,
        };

        self.reads.end_at(read_at);
        self.waits_for_programs =
            agent_part_was_asked && !self.may_start_programs;

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

impl Default for ProfileLimits {
    /// Nothing read yet, of a profile whose limits something may read.
    fn default() -> Self {
        Self {
            last_good: None,
            failures: None,
            has_sources: true,
            reads: ReadState::default(),
            may_start_programs: true,
            waits_for_programs: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::Unavailable;

    use super::*;
    use crate::testing::{at, five_hour_limits, limits_answer};

    /// The limits the provider part answered with `percent` used.
    fn from_provider(percent: f64) -> LimitsRead {
        limits_answer(vec![(
            SourceId::ProviderPart,
            Ok(five_hour_limits(percent)),
        )])
    }

    #[test]
    fn a_read_starts_once_until_it_ends() {
        let mut limits = ProfileLimits::default();

        assert!(limits.start_reading(true));
        assert!(!limits.start_reading(true));
        assert!(limits.reads().is_reading());

        limits.apply(from_provider(42.0), at(10));

        assert!(!limits.reads().is_reading());
        assert!(limits.start_reading(true));
    }

    #[test]
    fn limits_never_read_or_read_long_ago_are_stale() {
        let mut limits = ProfileLimits::default();
        let now = at(10_000);
        let never = limits.is_wanted(Which::Stale(now));

        let _ = limits.start_reading(true);
        limits.apply(from_provider(42.0), at(10));
        let old = limits.is_wanted(Which::Stale(now));
        let _ = limits.start_reading(true);
        limits.apply(from_provider(42.0), now);

        assert!(never);
        assert!(old);
        assert!(!limits.is_wanted(Which::Stale(now)));
        assert!(limits.is_wanted(Which::Every));
        assert!(!limits.is_wanted(Which::WaitingForPrograms));
    }

    /// A read while Agents mode was away did not start the hidden
    /// terminal; when no part answered, the terminal may answer on
    /// screen, so the limits wait for it although the read is fresh.
    #[test]
    fn a_failed_read_without_the_hidden_terminal_waits_for_it() {
        let mut limits = ProfileLimits::default();
        let now = at(10_000);
        let failed = limits_answer(vec![(
            SourceId::AgentPart,
            Err(SourceError::NotAvailable(Unavailable::ReadStopped)),
        )]);

        let _ = limits.start_reading(false);
        limits.apply(failed.clone(), now);
        let waits = limits.is_wanted(Which::WaitingForPrograms);
        let stale = limits.is_wanted(Which::Stale(now));
        let _ = limits.start_reading(false);
        limits.apply(from_provider(42.0), now);
        let answered_waits = limits.is_wanted(Which::WaitingForPrograms);
        let _ = limits.start_reading(true);
        limits.apply(failed, now);

        assert!(waits);
        assert!(stale);
        assert!(!answered_waits, "the provider part needs no program");
        assert!(!limits.is_wanted(Which::WaitingForPrograms));
        assert!(!limits.is_wanted(Which::Stale(now)));
    }

    /// Codex: the provider part failed and `codex app-server` did not
    /// start while Agents mode was away, so the agent part answered from
    /// its session files. On screen `app-server` may give fresher numbers,
    /// so the limits wait for it.
    #[test]
    fn an_agent_part_answer_without_programs_waits_for_them() {
        let mut limits = ProfileLimits::default();
        let now = at(10_000);
        let fallback = limits_answer(vec![
            (SourceId::ProviderPart, Err(SourceError::NotLoggedIn)),
            (SourceId::AgentPart, Ok(five_hour_limits(42.0))),
        ]);

        let _ = limits.start_reading(false);
        limits.apply(fallback.clone(), now);
        let waits = limits.is_wanted(Which::WaitingForPrograms);
        let _ = limits.start_reading(true);
        limits.apply(fallback, now);

        assert!(waits);
        assert!(
            !limits.is_wanted(Which::WaitingForPrograms),
            "on screen the programs had their chance"
        );
    }

    #[test]
    fn an_answer_is_kept_with_its_time() {
        let mut limits = ProfileLimits::default();
        let answer = from_provider(42.0);

        limits.apply(answer.clone(), at(10));

        let LimitsRead::Answered(expected) = answer else {
            panic!("the part answers");
        };
        assert_eq!(limits.last_good(), Some((&expected, at(10))));
        assert_eq!(limits.failures(), None);
    }

    #[test]
    fn a_failure_keeps_the_last_good_answer_and_the_next_answer_clears_it() {
        let mut limits = ProfileLimits::default();
        let good = from_provider(42.0);
        let failed = limits_answer(vec![(
            SourceId::ProviderPart,
            Err(SourceError::NotLoggedIn),
        )]);

        limits.apply(good.clone(), at(10));
        limits.apply(failed.clone(), at(20));
        let kept = limits
            .last_good()
            .map(|(answer, read_at)| (answer.clone(), read_at));
        let failures = limits.failures().map(<[SourceFailure]>::to_vec);
        limits.apply(from_provider(50.0), at(30));

        let LimitsRead::Answered(good) = good else {
            panic!("the part answers");
        };
        assert_eq!(kept, Some((good, at(10))));
        assert_eq!(failures.as_deref(), Some(failed.failures()));
        assert_eq!(limits.failures(), None);
        assert_eq!(
            limits.last_good().map(|(_, read_at)| read_at),
            Some(at(30))
        );
        assert!(limits.has_sources());
    }

    /// Spec 2026-11-19, Decision 3: a login the provider part could not
    /// use is "login expired"; an answer of the agent part after it, or
    /// another failure, is not.
    #[test]
    fn only_a_read_that_failed_for_the_login_is_a_login_expired() {
        let read = |parts: Vec<(SourceId, Result<Limits, SourceError>)>| {
            let mut limits = ProfileLimits::default();
            limits.apply(limits_answer(parts), at(10));
            limits.is_login_expired()
        };
        let not_logged_in =
            (SourceId::ProviderPart, Err(SourceError::NotLoggedIn));

        assert!(read(vec![not_logged_in.clone()]));
        assert!(!read(vec![
            not_logged_in,
            (SourceId::AgentPart, Ok(five_hour_limits(42.0))),
        ]));
        assert!(!read(vec![(
            SourceId::ProviderPart,
            Err(SourceError::Network(String::from("down"))),
        )]));
        assert!(!ProfileLimits::default().is_login_expired());
    }

    #[test]
    fn no_failures_at_all_means_nothing_reads_the_limits() {
        let mut limits = ProfileLimits::default();

        limits.apply(LimitsRead::NoSources, at(10));

        assert!(!limits.has_sources());
        assert_eq!(limits.failures(), None);
        assert_eq!(limits.last_good(), None);
    }
}
