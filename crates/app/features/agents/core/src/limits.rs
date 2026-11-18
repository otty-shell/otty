//! What was read of the limits of one profile, by the rules of spec
//! Decision 5: the last good answer stays with its time when a later read
//! fails.

use std::time::SystemTime;

use otty_agents::{Answer, Limits, SourceFailure, SourceId};

use super::freshness::{self, Which};

/// The limits of one profile: the last good answer and when it was read,
/// why the latest read failed, the sources the latest read walked, when
/// the latest read ended, and whether a read is on its way and may start
/// the agent's program (`codex app-server`, or `claude` in a hidden
/// terminal).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileLimits {
    last_good: Option<(Answer<Limits>, SystemTime)>,
    failures: Option<Vec<SourceFailure>>,
    sources: Vec<SourceId>,
    has_sources: bool,
    is_reading: bool,
    read_at: Option<SystemTime>,
    may_start_programs: bool,
    waits_for_programs: bool,
}

impl ProfileLimits {
    /// The last answer of a source and when it was read; it stays when a
    /// later read fails.
    pub(crate) fn last_good(&self) -> Option<(&Answer<Limits>, SystemTime)> {
        self.last_good
            .as_ref()
            .map(|(answer, read_at)| (answer, *read_at))
    }

    /// Why each source failed at the latest read, when no source answered.
    pub(crate) fn failures(&self) -> Option<&[SourceFailure]> {
        self.failures.as_deref()
    }

    /// The ids of every source of the agent at the latest read, best
    /// first, those after the one that answered too.
    pub(crate) fn sources(&self) -> &[SourceId] {
        &self.sources
    }

    /// Whether the agent has a way to read limits. `false` once a read
    /// found no source at all: the mode then draws no limits.
    pub(crate) fn has_sources(&self) -> bool {
        self.has_sources
    }

    /// Whether a read is on its way.
    pub(crate) fn is_reading(&self) -> bool {
        self.is_reading
    }

    /// Whether the limits need a read for `which`: limits that wait for
    /// the agent's program are stale whatever their time.
    pub(crate) fn is_wanted(&self, which: Which) -> bool {
        match which {
            Which::WaitingForPrograms => self.waits_for_programs,
            Which::Stale(_) => {
                self.waits_for_programs
                    || freshness::is_wanted(self.read_at, which)
            },
            Which::Every => true,
        }
    }

    /// Start a read, which may start the agent's program when
    /// `may_start_programs`; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self, may_start_programs: bool) -> bool {
        if self.is_reading {
            return false;
        }

        self.is_reading = true;
        self.may_start_programs = may_start_programs;
        true
    }

    /// Take the result of a read over `sources` that ended at `read_at`.
    /// An answer replaces the last good one. A failure keeps it and says
    /// why. No failures at all means the agent has no way to read limits.
    /// A read that could not start the agent's program waits for it when
    /// a source failed before the answer, or every source failed: the
    /// program, which did not start, may answer better.
    pub(crate) fn apply(
        &mut self,
        limits: Result<Answer<Limits>, Vec<SourceFailure>>,
        sources: Vec<SourceId>,
        read_at: SystemTime,
    ) {
        let has_a_failure = match &limits {
            Ok(answer) => !answer.earlier_failures().is_empty(),
            Err(failures) => !failures.is_empty(),
        };

        self.is_reading = false;
        self.sources = sources;
        self.read_at = Some(read_at);
        self.waits_for_programs = has_a_failure && !self.may_start_programs;

        match limits {
            Ok(answer) => {
                self.last_good = Some((answer, read_at));
                self.failures = None;
                self.has_sources = true;
            },
            Err(failures) if failures.is_empty() => {
                self.failures = None;
                self.has_sources = false;
            },
            Err(failures) => {
                self.failures = Some(failures);
                self.has_sources = true;
            },
        }
    }
}

impl Default for ProfileLimits {
    /// Nothing read yet, from an agent that may have sources.
    fn default() -> Self {
        Self {
            last_good: None,
            failures: None,
            sources: Vec::new(),
            has_sources: true,
            is_reading: false,
            read_at: None,
            may_start_programs: true,
            waits_for_programs: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SourceId::{
        ChatGptUsageApi, CodexAppServer, LatestCodexSession,
    };
    use otty_agents::{SourceError, Unavailable};

    use super::*;
    use crate::testing::{at, five_hour_limits, limits_answer};

    #[test]
    fn a_read_starts_once_until_it_ends() {
        let mut limits = ProfileLimits::default();

        assert!(limits.start_reading(true));
        assert!(!limits.start_reading(true));
        assert!(limits.is_reading());

        limits.apply(
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(42.0)))]),
            vec![ChatGptUsageApi],
            at(10),
        );

        assert!(!limits.is_reading());
        assert!(limits.start_reading(true));
    }

    #[test]
    fn limits_never_read_or_read_long_ago_are_stale() {
        let mut limits = ProfileLimits::default();
        let now = at(10_000);
        let never = limits.is_wanted(Which::Stale(now));
        let answer =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(42.0)))]);

        let _ = limits.start_reading(true);
        limits.apply(answer.clone(), vec![ChatGptUsageApi], at(10));
        let old = limits.is_wanted(Which::Stale(now));
        let _ = limits.start_reading(true);
        limits.apply(answer, vec![ChatGptUsageApi], now);

        assert!(never);
        assert!(old);
        assert!(!limits.is_wanted(Which::Stale(now)));
        assert!(limits.is_wanted(Which::Every));
        assert!(!limits.is_wanted(Which::WaitingForPrograms));
    }

    /// A read while Agents mode was away did not start the hidden
    /// terminal; when no source answered, the terminal may answer on
    /// screen, so the limits wait for it although the read is fresh.
    #[test]
    fn a_failed_read_without_the_hidden_terminal_waits_for_it() {
        let mut limits = ProfileLimits::default();
        let now = at(10_000);
        let failed = limits_answer(vec![(
            ChatGptUsageApi,
            Err(SourceError::NotAvailable(Unavailable::ReadStopped)),
        )]);
        let answered =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(42.0)))]);

        let _ = limits.start_reading(false);
        limits.apply(failed.clone(), vec![ChatGptUsageApi], now);
        let waits = limits.is_wanted(Which::WaitingForPrograms);
        let stale = limits.is_wanted(Which::Stale(now));
        let _ = limits.start_reading(false);
        limits.apply(answered, vec![ChatGptUsageApi], now);
        let answered_waits = limits.is_wanted(Which::WaitingForPrograms);
        let _ = limits.start_reading(true);
        limits.apply(failed, vec![ChatGptUsageApi], now);

        assert!(waits);
        assert!(stale);
        assert!(!answered_waits, "an answer needs no terminal");
        assert!(!limits.is_wanted(Which::WaitingForPrograms));
        assert!(!limits.is_wanted(Which::Stale(now)));
    }

    /// Codex: the usage request failed and `codex app-server` did not start
    /// while Agents mode was away, so the latest session answered. On
    /// screen `app-server` may give fresher numbers, so the limits wait for
    /// it; an answer of the first source needs no program.
    #[test]
    fn an_answer_after_a_failed_source_without_programs_waits_for_them() {
        let mut limits = ProfileLimits::default();
        let now = at(10_000);
        let sources = vec![ChatGptUsageApi, CodexAppServer, LatestCodexSession];
        let fallback = limits_answer(vec![
            (ChatGptUsageApi, Err(SourceError::NotLoggedIn)),
            (
                CodexAppServer,
                Err(SourceError::NotAvailable(Unavailable::ReadStopped)),
            ),
            (LatestCodexSession, Ok(five_hour_limits(42.0))),
        ]);
        let first =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(42.0)))]);

        let _ = limits.start_reading(false);
        limits.apply(fallback.clone(), sources.clone(), now);
        let waits = limits.is_wanted(Which::WaitingForPrograms);
        let _ = limits.start_reading(false);
        limits.apply(first, sources.clone(), now);
        let first_waits = limits.is_wanted(Which::WaitingForPrograms);
        let _ = limits.start_reading(true);
        limits.apply(fallback, sources, now);

        assert!(waits);
        assert!(!first_waits, "the first source needs no program");
        assert!(
            !limits.is_wanted(Which::WaitingForPrograms),
            "on screen the programs had their chance"
        );
    }

    #[test]
    fn an_answer_is_kept_with_its_time() {
        let mut limits = ProfileLimits::default();
        let answer =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(42.0)))]);

        limits.apply(answer.clone(), vec![ChatGptUsageApi], at(10));

        let expected = answer.expect("the source answers");
        assert_eq!(limits.last_good(), Some((&expected, at(10))));
        assert_eq!(limits.failures(), None);
    }

    #[test]
    fn a_failure_keeps_the_last_good_answer_and_the_next_answer_clears_it() {
        let mut limits = ProfileLimits::default();
        let good =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(42.0)))]);
        let failed = limits_answer(vec![(
            ChatGptUsageApi,
            Err(SourceError::NotLoggedIn),
        )]);
        let better =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(50.0)))]);

        limits.apply(good.clone(), vec![ChatGptUsageApi], at(10));
        limits.apply(failed.clone(), vec![ChatGptUsageApi], at(20));
        let kept = limits
            .last_good()
            .map(|(answer, read_at)| (answer.clone(), read_at));
        let failures = limits.failures().map(<[SourceFailure]>::to_vec);
        limits.apply(better.clone(), vec![ChatGptUsageApi], at(30));

        assert_eq!(kept, Some((good.expect("the source answers"), at(10))));
        assert_eq!(failures, failed.err());
        assert_eq!(limits.failures(), None);
        assert_eq!(
            limits.last_good().map(|(_, read_at)| read_at),
            Some(at(30))
        );
        assert!(limits.has_sources());
    }

    #[test]
    fn no_failures_at_all_means_the_agent_has_no_sources() {
        let mut limits = ProfileLimits::default();

        limits.apply(Err(Vec::new()), Vec::new(), at(10));

        assert!(!limits.has_sources());
        assert_eq!(limits.failures(), None);
        assert_eq!(limits.last_good(), None);
    }

    #[test]
    fn each_read_keeps_the_names_of_every_source_it_walked() {
        let mut limits = ProfileLimits::default();
        let answer =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(42.0)))]);

        limits.apply(
            answer,
            vec![ChatGptUsageApi, CodexAppServer, LatestCodexSession],
            at(10),
        );

        assert_eq!(
            limits.sources(),
            [ChatGptUsageApi, CodexAppServer, LatestCodexSession]
        );
    }
}
