//! What was read of the limits of one profile, by the rules of spec
//! Decision 5: the last good answer stays with its time when a later read
//! fails.

use std::time::SystemTime;

use otty_agents::{Answer, Limits, SourceFailure};

/// The limits of one profile: the last good answer and when it was read,
/// why the latest read failed, the sources the latest read walked, and
/// whether a read is on its way.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileLimits {
    last_good: Option<(Answer<Limits>, SystemTime)>,
    failures: Option<Vec<SourceFailure>>,
    sources: Vec<&'static str>,
    has_sources: bool,
    is_reading: bool,
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

    /// The names of every source of the agent at the latest read, best
    /// first, those after the one that answered too.
    pub(crate) fn sources(&self) -> &[&'static str] {
        &self.sources
    }

    /// Whether the agent has a way to read limits. `false` once a read
    /// found no source at all: the mode then draws no limits.
    pub(crate) fn has_sources(&self) -> bool {
        self.has_sources
    }

    /// Start a read; `false` when one is on its way.
    pub(crate) fn start_reading(&mut self) -> bool {
        if self.is_reading {
            return false;
        }

        self.is_reading = true;
        true
    }

    /// Take the result of a read over `sources` that ended at `read_at`.
    /// An answer replaces the last good one. A failure keeps it and says
    /// why. No failures at all means the agent has no way to read limits.
    pub(crate) fn apply(
        &mut self,
        limits: Result<Answer<Limits>, Vec<SourceFailure>>,
        sources: Vec<&'static str>,
        read_at: SystemTime,
    ) {
        self.is_reading = false;
        self.sources = sources;

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

#[cfg(test)]
impl ProfileLimits {
    /// Whether a read is on its way.
    pub(crate) fn is_reading(&self) -> bool {
        self.is_reading
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
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SourceError;

    use super::*;
    use crate::testing::{at, five_hour_limits, limits_answer};

    #[test]
    fn a_read_starts_once_until_it_ends() {
        let mut limits = ProfileLimits::default();

        assert!(limits.start_reading());
        assert!(!limits.start_reading());
        assert!(limits.is_reading());

        limits.apply(
            limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]),
            vec!["server"],
            at(10),
        );

        assert!(!limits.is_reading());
        assert!(limits.start_reading());
    }

    #[test]
    fn an_answer_is_kept_with_its_time() {
        let mut limits = ProfileLimits::default();
        let answer =
            limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]);

        limits.apply(answer.clone(), vec!["server"], at(10));

        let expected = answer.expect("the source answers");
        assert_eq!(limits.last_good(), Some((&expected, at(10))));
        assert_eq!(limits.failures(), None);
    }

    #[test]
    fn a_failure_keeps_the_last_good_answer_and_the_next_answer_clears_it() {
        let mut limits = ProfileLimits::default();
        let good = limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]);
        let failed =
            limits_answer(vec![("server", Err(SourceError::NotLoggedIn))]);
        let better =
            limits_answer(vec![("server", Ok(five_hour_limits(50.0)))]);

        limits.apply(good.clone(), vec!["server"], at(10));
        limits.apply(failed.clone(), vec!["server"], at(20));
        let kept = limits
            .last_good()
            .map(|(answer, read_at)| (answer.clone(), read_at));
        let failures = limits.failures().map(<[SourceFailure]>::to_vec);
        limits.apply(better.clone(), vec!["server"], at(30));

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
            limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]);

        limits.apply(answer, vec!["server", "program", "file"], at(10));

        assert_eq!(limits.sources(), ["server", "program", "file"]);
    }
}
