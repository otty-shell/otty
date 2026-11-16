//! The walk over the sources of one kind of data (spec Decision 5).
//!
//! The sources are tried best first and the first one that answers wins.
//! Any failure moves on to the next source, and every failure is kept, so
//! the user can see which source answered and why the earlier ones did not.

use crate::error::SourceError;
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::session::{AgentSession, SessionSource};

/// The answer of the first source that answered.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer<T> {
    value: T,
    source: &'static str,
    earlier_failures: Vec<SourceFailure>,
}

impl<T> Answer<T> {
    /// What the source answered.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// The name of the source that answered.
    pub fn source(&self) -> &'static str {
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
}

/// A source that gave no answer, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFailure {
    source: &'static str,
    error: SourceError,
}

impl SourceFailure {
    /// The name of the source.
    pub fn source(&self) -> &'static str {
        self.source
    }

    /// Why it gave no answer.
    pub fn error(&self) -> &SourceError {
        &self.error
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
///     AgentId, AgentSession, Profile, SessionSource, SourceError,
///     read_sessions,
/// };
///
/// struct NoFiles;
///
/// impl SessionSource for NoFiles {
///     fn name(&self) -> &'static str {
///         "files"
///     }
///
///     fn sessions(
///         &self,
///         _profile: &Profile,
///     ) -> Result<Vec<AgentSession>, SourceError> {
///         Err(SourceError::NotAvailable(String::from("no folder")))
///     }
/// }
///
/// let profile = Profile::default_for(
///     AgentId::new("codex"),
///     "Codex",
///     PathBuf::from("/home/user/.codex"),
/// );
/// let sources: Vec<Box<dyn SessionSource>> = vec![Box::new(NoFiles)];
///
/// let failures = read_sessions(&sources, &profile).unwrap_err();
///
/// assert_eq!(failures[0].source(), "files");
/// ```
pub fn read_sessions(
    sources: &[Box<dyn SessionSource>],
    profile: &Profile,
) -> Result<Answer<Vec<AgentSession>>, Vec<SourceFailure>> {
    let attempts = sources
        .iter()
        .map(|source| (source.name(), source.sessions(profile)));

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
        .map(|source| (source.name(), source.limits(profile)));

    walk(attempts)
}

/// Takes the attempts in order until one answers.
///
/// `attempts` is lazy: a source is asked only when its attempt is taken, so
/// the sources after the one that answered are never asked.
fn walk<T>(
    attempts: impl Iterator<Item = (&'static str, Result<T, SourceError>)>,
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
    use crate::billing::{Billing, LimitWindow, UsageWindows};
    use crate::limits::MockLimitSource;
    use crate::session::MockSessionSource;
    use crate::testing::{MockAgent, no_programs, profile, session};

    fn session_source(
        name: &'static str,
        answer: Result<Vec<AgentSession>, SourceError>,
    ) -> MockSessionSource {
        let mut source = MockSessionSource::new();
        source.expect_name().return_const(name);
        source
            .expect_sessions()
            .times(1)
            .return_once(move |_| answer);
        source
    }

    fn limit_source(
        name: &'static str,
        answer: Result<Limits, SourceError>,
    ) -> MockLimitSource {
        let mut source = MockLimitSource::new();
        source.expect_name().return_const(name);
        source.expect_limits().times(1).return_once(move |_| answer);
        source
    }

    fn unused_session_source() -> MockSessionSource {
        let mut source = MockSessionSource::new();
        source.expect_name().return_const("unused");
        source.expect_sessions().never();
        source
    }

    fn limits() -> Limits {
        let window = LimitWindow::new("5 hours", 42.0, None);

        Limits::new(
            Billing::UsageWindows(UsageWindows::new(vec![window], None)),
            None,
        )
    }

    #[test]
    fn the_first_source_answers_and_the_rest_are_not_asked() {
        let sessions = vec![session("a1", "/work/otty", 10)];
        let sources: Vec<Box<dyn SessionSource>> = vec![
            Box::new(session_source("files", Ok(sessions.clone()))),
            Box::new(unused_session_source()),
        ];

        let answer = read_sessions(&sources, &profile("main"))
            .expect("the first source answers");

        assert_eq!(answer.source(), "files");
        assert!(answer.earlier_failures().is_empty());
        assert_eq!(answer.into_value(), sessions);
    }

    #[test]
    fn the_second_source_answers_when_the_first_fails() {
        let sources: Vec<Box<dyn LimitSource>> = vec![
            Box::new(limit_source("server", Err(SourceError::NotLoggedIn))),
            Box::new(limit_source("program", Ok(limits()))),
        ];

        let answer = read_limits(&sources, &profile("main"))
            .expect("the second source answers");

        assert_eq!(answer.source(), "program");
        assert_eq!(answer.value(), &limits());
        assert_eq!(answer.earlier_failures().len(), 1);
        assert_eq!(answer.earlier_failures()[0].source(), "server");
        assert_eq!(
            answer.earlier_failures()[0].error(),
            &SourceError::NotLoggedIn
        );
    }

    #[test]
    fn every_failure_is_returned_when_no_source_answers() {
        let network = SourceError::Network(String::from("timed out"));
        let unexpected = SourceError::UnexpectedAnswer(String::from("no json"));
        let sources: Vec<Box<dyn LimitSource>> = vec![
            Box::new(limit_source("server", Err(network.clone()))),
            Box::new(limit_source("program", Err(unexpected.clone()))),
        ];

        let failures = read_limits(&sources, &profile("main"))
            .expect_err("no source answers");

        let reasons: Vec<_> = failures
            .iter()
            .map(|failure| (failure.source(), failure.error()))
            .collect();
        assert_eq!(reasons, [("server", &network), ("program", &unexpected)]);
    }

    #[test]
    fn an_agent_with_no_sources_gives_no_answer_and_no_failures() {
        let mut agent = MockAgent::new();
        agent.expect_session_sources().returning(Vec::new);
        agent.expect_limit_sources().returning(|_| Vec::new());
        let profile = profile("main");

        let sessions = read_sessions(&agent.session_sources(), &profile);
        let limits = read_limits(&agent.limit_sources(no_programs()), &profile);

        assert_eq!(sessions, Err(Vec::new()));
        assert_eq!(limits, Err(Vec::new()));
    }
}
