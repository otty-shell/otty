//! The walk over the sources of one kind of data (spec Decision 5).
//!
//! The sources are tried best first and the first one that answers wins.
//! Any failure moves on to the next source, and every failure is kept, so
//! the user can see which source answered and why the earlier ones did not.

use crate::error::SourceError;
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::session::{AgentSession, SessionSource};
use crate::source_id::SourceId;

/// The answer of the first source that answered.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer<T> {
    value: T,
    source: SourceId,
    earlier_failures: Vec<SourceFailure>,
}

impl<T> Answer<T> {
    /// What the source answered.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// The id of the source that answered.
    pub fn source(&self) -> SourceId {
        self.source
    }

    /// The failures of the sources tried before this one, in order. Empty
    /// when the best source answered.
    pub fn earlier_failures(&self) -> &[SourceFailure] {
        &self.earlier_failures
    }

    /// The answer without the source and the failures.
    pub fn into_value(self) -> T {
        self.value
    }

    /// The answer `value` of the source `source`, the first one asked.
    pub(crate) fn first(value: T, source: SourceId) -> Self {
        Self {
            value,
            source,
            earlier_failures: Vec::new(),
        }
    }
}

/// A source that gave no answer, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFailure {
    source: SourceId,
    error: SourceError,
}

impl SourceFailure {
    /// The id of the source.
    pub fn source(&self) -> SourceId {
        self.source
    }

    /// Why it gave no answer.
    pub fn error(&self) -> &SourceError {
        &self.error
    }

    /// The failure of the source `source` with `error`, for a read that
    /// ended before the walk asked the source, such as a read whose thread
    /// stopped.
    ///
    /// ```
    /// use otty_agents::{SourceError, SourceFailure, SourceId, Unavailable};
    ///
    /// let stopped = SourceError::NotAvailable(Unavailable::ReadStopped);
    /// let failure = SourceFailure::new(SourceId::Plugin, stopped.clone());
    ///
    /// assert_eq!(failure.source(), SourceId::Plugin);
    /// assert_eq!(failure.error(), &stopped);
    /// ```
    pub fn new(source: SourceId, error: SourceError) -> Self {
        Self { source, error }
    }
}

/// Lists the sessions of `profile` from the first source that answers.
///
/// When no source answered, the error holds the failure of each source in
/// order. It is empty when there were no sources, which means the agent can
/// not list sessions.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{
///     AgentId, AgentSession, Profile, SessionSource, SourceError, SourceId,
///     Unavailable, read_sessions,
/// };
///
/// struct NoFiles;
///
/// impl SessionSource for NoFiles {
///     fn id(&self) -> SourceId {
///         SourceId::Plugin
///     }
///
///     fn sessions(
///         &self,
///         _profile: &Profile,
///     ) -> Result<Vec<AgentSession>, SourceError> {
///         Err(SourceError::NotAvailable(Unavailable::NoSessionFiles))
///     }
/// }
///
/// let profile = Profile::default_for(
///     AgentId::new("coder"),
///     "Coder",
///     PathBuf::from("/home/user/.coder"),
/// );
/// let sources: Vec<Box<dyn SessionSource>> = vec![Box::new(NoFiles)];
///
/// let failures = read_sessions(&sources, &profile).unwrap_err();
///
/// assert_eq!(failures[0].source(), SourceId::Plugin);
/// ```
pub fn read_sessions(
    sources: &[Box<dyn SessionSource>],
    profile: &Profile,
) -> Result<Answer<Vec<AgentSession>>, Vec<SourceFailure>> {
    let attempts = sources
        .iter()
        .map(|source| (source.id(), source.sessions(profile)));

    walk(attempts)
}

/// Reads the limits of `profile` from the first source that answers.
///
/// When no source answered, the error holds the failure of each source in
/// order. It is empty when there were no sources, which means the agent can
/// not read limits.
pub fn read_limits(
    sources: &[Box<dyn LimitSource>],
    profile: &Profile,
) -> Result<Answer<Limits>, Vec<SourceFailure>> {
    let attempts = sources
        .iter()
        .map(|source| (source.id(), source.limits(profile)));

    walk(attempts)
}

/// Takes the attempts in order until one answers: the walk every kind of
/// data goes through, such as the prices of a [`PriceSource`](crate::PriceSource).
///
/// `attempts` is lazy: a source is asked only when its attempt is taken, so
/// the sources after the one that answered are never asked.
pub(crate) fn walk<T>(
    attempts: impl Iterator<Item = (SourceId, Result<T, SourceError>)>,
) -> Result<Answer<T>, Vec<SourceFailure>> {
    let mut failures = Vec::new();

    for (source, result) in attempts {
        match result {
            Ok(value) => {
                return Ok(Answer {
                    value,
                    source,
                    earlier_failures: failures,
                });
            },
            Err(error) => failures.push(SourceFailure { source, error }),
        }
    }

    Err(failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::error::Unexpected;
    use crate::limits::{
        Billing, LimitWindow, MockLimitSource, UsageWindows, WindowId,
    };
    use crate::session::MockSessionSource;
    use crate::source_id::SourceName;
    use crate::testing::{
        MockAgent, no_programs, no_terminals, profile, session,
    };

    fn session_source(
        id: SourceId,
        answer: Result<Vec<AgentSession>, SourceError>,
    ) -> MockSessionSource {
        let mut source = MockSessionSource::new();
        source.expect_id().return_const(id);
        source
            .expect_sessions()
            .times(1)
            .return_once(move |_| answer);
        source
    }

    fn limit_source(
        id: SourceId,
        answer: Result<Limits, SourceError>,
    ) -> MockLimitSource {
        let mut source = MockLimitSource::new();
        source.expect_id().return_const(id);
        source.expect_limits().times(1).return_once(move |_| answer);
        source
    }

    fn unused_session_source() -> MockSessionSource {
        let mut source = MockSessionSource::new();
        source.expect_id().return_const(SourceId::Plugin);
        source.expect_sessions().never();
        source
    }

    /// The first of two named sources of limits.
    fn first() -> SourceId {
        SourceId::Named(SourceName::new("first made-up source"))
    }

    /// The second of two named sources of limits.
    fn second() -> SourceId {
        SourceId::Named(SourceName::new("second made-up source"))
    }

    fn limits() -> Limits {
        let window = LimitWindow::new(WindowId::hours(5), 42.0, None);

        Limits::new(
            Billing::UsageWindows(UsageWindows::new(vec![window], None)),
            None,
        )
    }

    #[test]
    fn the_first_source_answers_and_the_rest_are_not_asked() {
        let sessions = vec![session("a1", "/work/otty", 10)];
        let sources: Vec<Box<dyn SessionSource>> = vec![
            Box::new(session_source(SourceId::Plugin, Ok(sessions.clone()))),
            Box::new(unused_session_source()),
        ];

        let answer = read_sessions(&sources, &profile("main"))
            .expect("the first source answers");

        assert_eq!(answer.source(), SourceId::Plugin);
        assert!(answer.earlier_failures().is_empty());
        assert_eq!(answer.into_value(), sessions);
    }

    #[test]
    fn the_second_source_answers_when_the_first_fails() {
        let sources: Vec<Box<dyn LimitSource>> = vec![
            Box::new(limit_source(first(), Err(SourceError::NotLoggedIn))),
            Box::new(limit_source(second(), Ok(limits()))),
        ];

        let answer = read_limits(&sources, &profile("main"))
            .expect("the second source answers");

        assert_eq!(answer.source(), second());
        assert_eq!(answer.value(), &limits());
        assert_eq!(answer.earlier_failures().len(), 1);
        assert_eq!(answer.earlier_failures()[0].source(), first());
        assert_eq!(
            answer.earlier_failures()[0].error(),
            &SourceError::NotLoggedIn
        );
    }

    #[test]
    fn every_failure_is_returned_when_no_source_answers() {
        let network = SourceError::Network(String::from("timed out"));
        let unexpected =
            SourceError::UnexpectedAnswer(Unexpected::NotJsonObject);
        let sources: Vec<Box<dyn LimitSource>> = vec![
            Box::new(limit_source(first(), Err(network.clone()))),
            Box::new(limit_source(second(), Err(unexpected.clone()))),
        ];

        let failures = read_limits(&sources, &profile("main"))
            .expect_err("no source answers");

        let reasons: Vec<_> = failures
            .iter()
            .map(|failure| (failure.source(), failure.error()))
            .collect();
        assert_eq!(reasons, [(first(), &network), (second(), &unexpected)]);
    }

    #[test]
    fn an_agent_with_no_sources_gives_no_answer_and_no_failures() {
        let mut agent = MockAgent::new();
        agent.expect_session_sources().returning(Vec::new);
        agent.expect_limit_sources().returning(|_, _| Vec::new());
        let profile = profile("main");

        let sessions = read_sessions(&agent.session_sources(), &profile);
        let sources = agent.limit_sources(no_programs(), no_terminals());
        let limits = read_limits(&sources, &profile);

        assert_eq!(sessions, Err(Vec::new()));
        assert_eq!(limits, Err(Vec::new()));
    }
}
