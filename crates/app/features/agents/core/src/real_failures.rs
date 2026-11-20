//! The real failures of Agents mode and telling each of them once (spec
//! Decision 10).
//!
//! A real failure is one that is not an expected state: `agents.json` that
//! can not be read, a config or session folder that is there but can not
//! be read, every limit source failing with at least one answer OTTY does
//! not expect (the agent probably changed), and a price table that is
//! broken or not what OTTY expects. Not logged in, not available and a
//! plan without numbers are expected and never told.
//!
//! Each answer of a read is checked: a problem that is there and was not
//! told yet is told, as [`AgentsOutcome::Failed`]; a problem that is gone is
//! forgotten, so it is told again when it comes back. A failed save of
//! `agents.json` is not here: it answers the user's own press every time.

use otty_agents::{Profile, SourceError, SourceFailure};
use otty_app_i18n::{self as i18n, Key};

use super::event::AgentsOutcome;
use super::failure_text::failure_reasons;
use super::types::{
    LimitsRead, LoadedLimits, LoadedPrices, LoadedSessions, SessionsRead,
};

/// The problems the user was told of and that are still there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ToldProblems {
    told: Vec<Problem>,
}

impl ToldProblems {
    /// Take the checks of one answer and return the notifications of the
    /// problems that are there and were not told yet. A problem that is
    /// gone is forgotten.
    pub(crate) fn tell(&mut self, checks: Vec<Check>) -> Vec<AgentsOutcome> {
        let mut outcomes = Vec::new();

        for Check { problem, text } in checks {
            let was_told = self.told.contains(&problem);
            match text {
                Some(text) if !was_told => {
                    self.told.push(problem);
                    outcomes.push(AgentsOutcome::Failed(text));
                },
                Some(_) => {},
                None => self.told.retain(|told| *told != problem),
            }
        }

        outcomes
    }
}

/// One kind of real failure checked in an answer, with the text that tells
/// it when it is there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Check {
    problem: Problem,
    text: Option<String>,
}

/// A real failure, by what failed and for which profile.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Problem {
    /// `agents.json` could not be read.
    ProfilesFileUnreadable,
    /// A config or session folder of the profile is there but could not
    /// be read.
    FolderUnreadable(Profile),
    /// Every limit source of the profile failed, one with an answer OTTY
    /// does not expect.
    LimitsChanged(Profile),
    /// The LiteLLM price table or its kept copy is broken or not what OTTY
    /// expects.
    PricesUnreadable,
}

/// The check of a read of the profiles that could not read `agents.json`
/// for the reason `problem`, ready to show; `None` when it could.
pub(crate) fn profiles(problem: Option<&str>) -> Check {
    Check {
        problem: Problem::ProfilesFileUnreadable,
        text: problem.map(str::to_string),
    }
}

/// The check of a read of the profiles that stopped before it answered:
/// the user is told the profiles could not be read, because the read
/// stopped.
pub(crate) fn profiles_read_stopped() -> Check {
    let text = i18n::with_error(
        Key::TplAgentsProfilesReadFailed,
        i18n::t(Key::AgentsReadStopped),
    );

    profiles(Some(&text))
}

/// The check of a read of the sessions: a folder that is there but could
/// not be read.
pub(crate) fn sessions(loaded: &LoadedSessions) -> Check {
    let failures = match &loaded.sessions {
        SessionsRead::Failed(failures) => failures.as_slice(),
        _ => &[],
    };

    let unreadable = only(failures, |error| {
        matches!(error, SourceError::Unreadable { .. })
    });
    let text = (!unreadable.is_empty()).then(|| {
        problem_text(
            Key::TplAgentsFolderUnreadable,
            &loaded.profile,
            &unreadable,
        )
    });

    Check {
        problem: Problem::FolderUnreadable(loaded.profile.clone()),
        text,
    }
}

/// The check of a read of the limits: every source failing with one
/// unexpected answer.
pub(crate) fn limits(loaded: &LoadedLimits) -> Check {
    let failures = loaded.limits.failures();
    let has_answer = matches!(loaded.limits, LimitsRead::Answered(_));

    let is_unexpected =
        |error: &SourceError| matches!(error, SourceError::UnexpectedAnswer(_));
    let has_unexpected = failures.iter().any(|f| is_unexpected(f.error()));
    let changed = (!has_answer && has_unexpected).then(|| {
        problem_text(Key::TplAgentsLimitsChanged, &loaded.profile, failures)
    });

    Check {
        problem: Problem::LimitsChanged(loaded.profile.clone()),
        text: changed,
    }
}

/// The check of a read of the list prices: a price file that is broken
/// or not what the code expects, also when a later source answered (spec
/// Decision 16). A network failure is expected and never told.
pub(crate) fn prices(loaded: &LoadedPrices) -> Check {
    let failures = match &loaded.prices {
        Ok(answer) => answer.earlier_failures(),
        Err(failures) => failures.as_slice(),
    };

    let unexpected = only(failures, |error| {
        matches!(error, SourceError::UnexpectedAnswer(_))
    });
    let text = (!unexpected.is_empty()).then(|| {
        i18n::with_error(
            Key::TplAgentsPricesUnreadable,
            &failure_reasons(&unexpected),
        )
    });

    Check {
        problem: Problem::PricesUnreadable,
        text,
    }
}

/// The failures among `failures` whose error is one `is_kind` picks.
fn only(
    failures: &[SourceFailure],
    is_kind: impl Fn(&SourceError) -> bool,
) -> Vec<SourceFailure> {
    failures
        .iter()
        .filter(|failure| is_kind(failure.error()))
        .cloned()
        .collect()
}

/// The text of the template `key` for `profile`, with the reasons of
/// `failures`.
fn problem_text(
    key: Key,
    profile: &Profile,
    failures: &[SourceFailure],
) -> String {
    let reasons = failure_reasons(failures);

    i18n::filled(key, &[("name", profile.name()), ("error", &reasons)])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        ProfileStatus, SourceError, SourceId, Unavailable, Unexpected,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        agents, api_key, at, claude, codex, connected, env, five_hour_limits,
        limits_answer, limits_loaded, prices_answer, prices_loaded,
        profiles_loaded_with_problem, sessions_loaded, with_profiles,
    };
    use crate::types::LoadedSessions;

    /// The texts of the notifications among `outcomes`.
    fn told(outcomes: Vec<AgentsOutcome>) -> Vec<String> {
        outcomes
            .into_iter()
            .filter_map(|outcome| match outcome {
                AgentsOutcome::Failed(text) => Some(text),
                _ => None,
            })
            .collect()
    }

    /// The failures of one read in which each of `errors` failed its
    /// source.
    fn failures(errors: Vec<(SourceId, SourceError)>) -> Vec<SourceFailure> {
        let sources = errors
            .into_iter()
            .map(|(source, error)| (source, Err(error)))
            .collect();

        limits_answer(sources).failures().to_vec()
    }

    fn sessions_failed(errors: Vec<(SourceId, SourceError)>) -> AgentsIntent {
        AgentsIntent::SessionsLoaded(LoadedSessions {
            profile: claude(),
            sessions: SessionsRead::Failed(failures(errors)),
            missing_folders: Vec::new(),
        })
    }

    fn unreadable() -> SourceError {
        SourceError::Unreadable {
            path: PathBuf::from("/home/user/.claude/projects"),
            detail: String::from("Permission denied"),
        }
    }

    fn unexpected() -> SourceError {
        SourceError::UnexpectedAnswer(Unexpected::NotJsonObject)
    }

    fn not_available() -> SourceError {
        SourceError::NotAvailable(Unavailable::ProgramNotFound("claude"))
    }

    #[test]
    fn a_profiles_file_that_can_not_be_read_is_told_once_until_it_is_read() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let profiles = [(claude(), connected())];
        let mut read = |problem: Option<&str>| {
            let _ = reduce(&mut state, AgentsIntent::Refresh);
            told(reduce(
                &mut state,
                profiles_loaded_with_problem(&profiles, problem),
            ))
        };

        let first = read(Some("Could not read agents.json"));
        let again = read(Some("Could not read agents.json"));
        let mended = read(None);
        let broken_again = read(Some("Could not read agents.json"));

        assert_eq!(first, ["Could not read agents.json"]);
        assert!(again.is_empty(), "the same problem is told once");
        assert!(mended.is_empty());
        assert_eq!(broken_again, ["Could not read agents.json"]);
    }

    #[test]
    fn a_folder_that_can_not_be_read_is_told_once_each_time_it_breaks() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let expected = i18n::filled(
            Key::TplAgentsFolderUnreadable,
            &[
                ("name", "Claude Code"),
                (
                    "error",
                    "Plugin: can not read /home/user/.claude/projects: \
                     Permission denied",
                ),
            ],
        );

        let first = told(reduce(
            &mut state,
            sessions_failed(vec![(SourceId::Plugin, unreadable())]),
        ));
        let again = told(reduce(
            &mut state,
            sessions_failed(vec![(SourceId::Plugin, unreadable())]),
        ));
        let mended =
            told(reduce(&mut state, sessions_loaded(&claude(), Vec::new())));
        let broken_again = told(reduce(
            &mut state,
            sessions_failed(vec![(SourceId::Plugin, unreadable())]),
        ));

        assert_eq!(first, std::slice::from_ref(&expected));
        assert!(again.is_empty());
        assert!(mended.is_empty());
        assert_eq!(broken_again, [expected]);
    }

    #[test]
    fn every_limit_source_failing_with_an_unexpected_answer_is_told_once() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let changed = || {
            limits_answer(vec![
                (
                    crate::testing::named_source("ChatGPT usage API"),
                    Err(unexpected()),
                ),
                (
                    crate::testing::named_source("codex app-server"),
                    Err(not_available()),
                ),
                (
                    crate::testing::named_source("Latest Codex session"),
                    Err(not_available()),
                ),
            ])
        };

        let first = told(reduce(
            &mut state,
            limits_loaded(&codex(), changed(), at(10)),
        ));
        let again = told(reduce(
            &mut state,
            limits_loaded(&codex(), changed(), at(20)),
        ));

        assert_eq!(first.len(), 1);
        assert!(first[0].contains("Codex"), "{first:?}");
        assert!(
            first[0]
                .contains("unexpected answer: the answer is not a JSON object")
        );
        assert!(again.is_empty());
    }

    /// Not logged in, not available, a later source that answered after an
    /// unexpected answer and a folder that is not there are expected.
    #[test]
    fn an_expected_state_is_never_told() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), ProfileStatus::NotLoggedIn),
        ]);

        let logged_out = told(reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![
                    (
                        crate::testing::named_source("Anthropic usage API"),
                        Err(SourceError::NotLoggedIn),
                    ),
                    (
                        crate::testing::named_source("Claude Code /usage"),
                        Err(not_available()),
                    ),
                ]),
                at(10),
            ),
        ));
        let answered_later = told(reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![
                    (
                        crate::testing::named_source("Anthropic usage API"),
                        Err(unexpected()),
                    ),
                    (
                        crate::testing::named_source("Claude Code /usage"),
                        Ok(five_hour_limits(10.0)),
                    ),
                ]),
                at(20),
            ),
        ));
        let no_folder = told(reduce(
            &mut state,
            sessions_failed(vec![(SourceId::Plugin, not_available())]),
        ));

        assert!(logged_out.is_empty());
        assert!(answered_later.is_empty());
        assert!(no_folder.is_empty());
    }

    #[test]
    fn a_broken_price_file_is_told_once_until_it_reads() {
        let mut state = with_profiles(&[(claude(), api_key())]);
        let broken = SourceError::UnexpectedAnswer(Unexpected::NotJsonObject);
        let mut read = |failures: &[(SourceId, SourceError)]| {
            told(reduce(
                &mut state,
                prices_loaded(prices_answer(failures, None), at(10)),
            ))
        };

        let first = read(&[
            (SourceId::LiteLlmPrices, broken.clone()),
            (SourceId::KeptPrices, not_available()),
        ]);
        let again = read(&[(SourceId::LiteLlmPrices, broken.clone())]);
        let down = read(&[(
            SourceId::LiteLlmPrices,
            SourceError::Network(String::from("down")),
        )]);
        let broken_again = read(&[(SourceId::KeptPrices, broken.clone())]);

        let text = |source: Key| {
            let name = i18n::t(source);
            i18n::with_error(
                Key::TplAgentsPricesUnreadable,
                &format!("{name}: {broken}"),
            )
        };
        assert_eq!(first, [text(Key::AgentsSourceLitellmPrices)]);
        assert!(again.is_empty(), "the same problem is told once");
        assert!(down.is_empty(), "a network failure is expected");
        assert_eq!(broken_again, [text(Key::AgentsSourceKeptPrices)]);
    }
}
