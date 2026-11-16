//! The real failures of Agents mode and telling each of them once (spec
//! Decision 10).
//!
//! A real failure is one that is not an expected state: `agents.json` that
//! can not be read, a config or session folder that is there but can not
//! be read, every limit source failing with at least one answer OTTY does
//! not expect (the agent probably changed), and a program started for the
//! limits that stopped unexpectedly. Not logged in, not available and a
//! plan without numbers are expected and never told.
//!
//! Each answer of a read is checked: a problem that is there and was not
//! told yet is told, as [`AgentsOutcome::Failed`]; a problem that is gone is
//! forgotten, so it is told again when it comes back. A failed save of
//! `agents.json` is not here: it answers the user's own press every time.

use otty_agents::{Profile, SourceError, SourceFailure};
use otty_app_i18n::{self as i18n, Key};

use super::event::AgentsOutcome;
use super::model::failure_reasons;
use super::types::{LoadedLimits, LoadedSessions};

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
    /// A program started to read the limits of the profile stopped
    /// unexpectedly.
    ProgramStopped(Profile),
}

/// The check of a read of the profiles that could not read `agents.json`
/// for the reason `problem`, ready to show; `None` when it could.
pub(crate) fn profiles(problem: Option<&str>) -> Check {
    Check {
        problem: Problem::ProfilesFileUnreadable,
        text: problem.map(str::to_string),
    }
}

/// The check of a read of the sessions: a folder that is there but could
/// not be read.
pub(crate) fn sessions(loaded: &LoadedSessions) -> Check {
    let failures = match &loaded.sessions {
        Ok(_) => &[][..],
        Err(failures) => failures.as_slice(),
    };

    let unreadable = only(failures, |error| {
        matches!(error, SourceError::Unreadable(_))
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

/// The checks of a read of the limits: every source failing with one
/// unexpected answer, and a program that stopped, also when a later
/// source answered.
pub(crate) fn limits(loaded: &LoadedLimits) -> [Check; 2] {
    let (failures, has_answer) = match &loaded.limits {
        Ok(answer) => (answer.earlier_failures(), true),
        Err(failures) => (failures.as_slice(), false),
    };

    let is_unexpected =
        |error: &SourceError| matches!(error, SourceError::UnexpectedAnswer(_));
    let has_unexpected = failures.iter().any(|f| is_unexpected(f.error()));
    let changed = (!has_answer && has_unexpected).then(|| {
        problem_text(Key::TplAgentsLimitsChanged, &loaded.profile, failures)
    });

    let stopped = only(failures, |error| {
        matches!(error, SourceError::ProgramStopped(_))
    });
    let stopped_text = (!stopped.is_empty()).then(|| {
        problem_text(Key::TplAgentsProgramStopped, &loaded.profile, &stopped)
    });

    [
        Check {
            problem: Problem::LimitsChanged(loaded.profile.clone()),
            text: changed,
        },
        Check {
            problem: Problem::ProgramStopped(loaded.profile.clone()),
            text: stopped_text,
        },
    ]
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

    i18n::agents_template(key, &[("name", profile.name()), ("error", &reasons)])
}

#[cfg(test)]
mod tests {
    use otty_agents::SourceId::{
        AnthropicUsageApi, ChatGptUsageApi, ClaudeUsageCommand, CodexAppServer,
        LatestCodexSession, SessionFiles,
    };
    use otty_agents::{ProfileStatus, SourceError, SourceId};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        agents, at, claude, codex, connected, env, five_hour_limits,
        limits_answer, limits_loaded, profiles_loaded_with_problem,
        sessions_loaded, with_profiles,
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

        limits_answer(sources).expect_err("no source answers")
    }

    fn sessions_failed(errors: Vec<(SourceId, SourceError)>) -> AgentsIntent {
        AgentsIntent::SessionsLoaded(LoadedSessions {
            profile: claude(),
            sessions: Err(failures(errors)),
            missing_folders: Vec::new(),
        })
    }

    fn unreadable() -> SourceError {
        SourceError::Unreadable(String::from(
            "/home/user/.claude/projects: Permission denied",
        ))
    }

    fn unexpected() -> SourceError {
        SourceError::UnexpectedAnswer(String::from("no window"))
    }

    fn stopped() -> SourceError {
        SourceError::ProgramStopped(String::from(
            "codex app-server ended before it answered",
        ))
    }

    fn not_available() -> SourceError {
        SourceError::NotAvailable(String::from("no program"))
    }

    #[test]
    fn a_profiles_file_that_can_not_be_read_is_told_once_until_it_is_read() {
        let mut state = AgentsState::new(agents(), env());
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
        let expected = i18n::agents_template(
            Key::TplAgentsFolderUnreadable,
            &[
                ("name", "Claude Code"),
                (
                    "error",
                    "Session files: can not read /home/user/.claude/projects: \
                     Permission denied",
                ),
            ],
        );

        let first = told(reduce(
            &mut state,
            sessions_failed(vec![(SessionFiles, unreadable())]),
        ));
        let again = told(reduce(
            &mut state,
            sessions_failed(vec![(SessionFiles, unreadable())]),
        ));
        let mended =
            told(reduce(&mut state, sessions_loaded(&claude(), Vec::new())));
        let broken_again = told(reduce(
            &mut state,
            sessions_failed(vec![(SessionFiles, unreadable())]),
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
                (ChatGptUsageApi, Err(unexpected())),
                (CodexAppServer, Err(not_available())),
                (LatestCodexSession, Err(not_available())),
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
        assert!(first[0].contains("unexpected answer: no window"));
        assert!(again.is_empty());
    }

    #[test]
    fn a_program_that_stopped_is_told_also_when_a_later_source_answered() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let answered_after_a_stop = || {
            limits_answer(vec![
                (ChatGptUsageApi, Err(SourceError::NotLoggedIn)),
                (CodexAppServer, Err(stopped())),
                (LatestCodexSession, Ok(five_hour_limits(10.0))),
            ])
        };
        let expected = i18n::agents_template(
            Key::TplAgentsProgramStopped,
            &[
                ("name", "Codex"),
                (
                    "error",
                    "Codex app-server: stopped unexpectedly: codex \
                     app-server ended before it answered",
                ),
            ],
        );

        let first = told(reduce(
            &mut state,
            limits_loaded(&codex(), answered_after_a_stop(), at(10)),
        ));
        let again = told(reduce(
            &mut state,
            limits_loaded(&codex(), answered_after_a_stop(), at(20)),
        ));
        let first_answers =
            limits_answer(vec![(ChatGptUsageApi, Ok(five_hour_limits(10.0)))]);
        let mended = told(reduce(
            &mut state,
            limits_loaded(&codex(), first_answers, at(30)),
        ));
        let stopped_again = told(reduce(
            &mut state,
            limits_loaded(&codex(), answered_after_a_stop(), at(40)),
        ));

        assert_eq!(first, std::slice::from_ref(&expected));
        assert!(again.is_empty());
        assert!(mended.is_empty());
        assert_eq!(stopped_again, [expected]);
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
                    (AnthropicUsageApi, Err(SourceError::NotLoggedIn)),
                    (ClaudeUsageCommand, Err(not_available())),
                ]),
                at(10),
            ),
        ));
        let answered_later = told(reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![
                    (AnthropicUsageApi, Err(unexpected())),
                    (ClaudeUsageCommand, Ok(five_hour_limits(10.0))),
                ]),
                at(20),
            ),
        ));
        let no_folder = told(reduce(
            &mut state,
            sessions_failed(vec![(SessionFiles, not_available())]),
        ));

        assert!(logged_out.is_empty());
        assert!(answered_later.is_empty());
        assert!(no_folder.is_empty());
    }
}
