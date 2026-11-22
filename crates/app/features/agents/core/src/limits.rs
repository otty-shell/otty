//! What was read of the limits of one profile, by the rules of spec
//! Decision 5: the last good answer stays with its time when a later read
//! fails.

use std::time::SystemTime;

use otty_agents::{
    Answer, Limits, PartNeed, RefusedLogin, SourceError, SourceFailure,
    SourceId,
};

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

    /// What the parts ask the user to do when no part answered at the
    /// latest read, such as log in again (spec 2026-11-19-agent-plugins,
    /// issue 20). The agent part's needs come first: it knows the
    /// profile's agent and can run it, while a provider part only knows
    /// the login.
    pub(crate) fn needs(&self) -> Vec<&PartNeed> {
        let Some(failures) = &self.failures else {
            return Vec::new();
        };

        let of_agent = failures
            .iter()
            .filter(|failure| failure.source() == SourceId::AgentPart);
        let of_others = failures
            .iter()
            .filter(|failure| failure.source() != SourceId::AgentPart);

        of_agent.chain(of_others).filter_map(need_of).collect()
    }

    /// The logins a provider part refused at the latest read, when no
    /// part answered and the agent part that keeps the login said nothing
    /// about it.
    pub(crate) fn refused_logins(&self) -> Vec<&RefusedLogin> {
        let Some(failures) = &self.failures else {
            return Vec::new();
        };

        failures
            .iter()
            .filter_map(|failure| match failure.error() {
                SourceError::LoginRefused(refused) => Some(refused.as_ref()),
                _ => None,
            })
            .collect()
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
    /// that could not start the agent's program and gave no limits waits
    /// for it: the provider part may read them by asking the agent, such
    /// as with `claude /usage`, once the program may start (issue 29).
    pub(crate) fn apply(&mut self, limits: LimitsRead, read_at: SystemTime) {
        let is_answer = matches!(&limits, LimitsRead::Answered(_));

        self.reads.end_at(read_at);
        self.waits_for_programs = !is_answer && !self.may_start_programs;

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
            SourceId::ProviderPart,
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

    /// Issue 29: Claude Code keeps no token in its files, so the provider
    /// part reads the limits with `claude /usage`, which does not start
    /// while Agents mode is away: the part answers nothing then, and the
    /// limits wait for the program on screen.
    #[test]
    fn nothing_read_without_programs_waits_for_them() {
        let mut limits = ProfileLimits::default();
        let now = at(10_000);

        let _ = limits.start_reading(false);
        limits.apply(LimitsRead::NoSources, now);
        let waits = limits.is_wanted(Which::WaitingForPrograms);
        let _ = limits.start_reading(true);
        limits.apply(LimitsRead::NoSources, now);

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
            Err(crate::testing::login_expired()),
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

    /// Issue 20: the needs of the parts show only while no part answered;
    /// the agent part's need comes first, as it can run the agent.
    #[test]
    fn the_needs_of_a_read_no_part_answered_come_agent_part_first() {
        let needs = |parts: Vec<(SourceId, Result<Limits, SourceError>)>| {
            let mut limits = ProfileLimits::default();
            limits.apply(limits_answer(parts), at(10));
            limits
                .needs()
                .into_iter()
                .map(|need| need.id().to_string())
                .collect::<Vec<_>>()
        };
        let provider = (
            SourceId::ProviderPart,
            Err(crate::testing::refused_key_need()),
        );
        let agent = (SourceId::AgentPart, Err(crate::testing::login_expired()));

        assert_eq!(
            needs(vec![provider.clone(), agent]),
            ["login-expired", "new-key"]
        );
        assert!(
            needs(vec![
                provider,
                (SourceId::AgentPart, Ok(five_hour_limits(42.0))),
            ])
            .is_empty()
        );
        assert!(
            needs(vec![(
                SourceId::ProviderPart,
                Err(SourceError::Network(String::from("down"))),
            )])
            .is_empty()
        );
        assert!(ProfileLimits::default().needs().is_empty());
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
