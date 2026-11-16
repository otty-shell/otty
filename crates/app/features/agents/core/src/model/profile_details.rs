use std::time::SystemTime;

use otty_agents::{Profile, ProfileStatus, SourceFailure};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::state::{AgentsState, ProfileState};
use super::failure_reasons;
use super::limits::LimitsViewModel;
use super::tokens::TokensViewModel;

/// The details of the selected profile in the main area (spec Decision 6):
/// its agent, name, state, account and plan, config folder, limits, tokens
/// and buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileDetailsViewModel {
    /// The profile, sent with the buttons.
    pub(crate) profile: Profile,
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile name.
    pub(crate) name: String,
    /// The name of the agent.
    pub(crate) agent: &'static str,
    /// The state word and its color.
    pub(crate) state: String,
    pub(crate) tone: SessionTone,
    /// What is missing, or what to do, when the profile can not be used.
    pub(crate) hint: Option<&'static str>,
    /// The account the profile is logged in with, when known.
    pub(crate) account: Option<String>,
    /// The plan of the account, when known: from the login, else from the
    /// limits.
    pub(crate) plan: Option<String>,
    /// The config folder.
    pub(crate) config_dir: String,
    /// The program path the user set, if any.
    pub(crate) program: Option<String>,
    /// Why the sessions could not be read, when the last read failed.
    pub(crate) failure: Option<String>,
    /// The limits: each window with its percent and reset, the credits,
    /// the source and why a read failed.
    pub(crate) limits: Option<LimitsViewModel>,
    /// The tokens spent today and in the last 30 days.
    pub(crate) tokens: Option<TokensViewModel>,
    /// Whether Log in shows: the profile is not logged in and its program
    /// was found.
    pub(crate) can_log_in: bool,
    /// Whether Remove shows: only an added profile can be removed.
    pub(crate) can_remove: bool,
}

impl ProfileDetailsViewModel {
    /// Build the details of the shown profile `shown` at `now`, the time
    /// the reset times count down from.
    pub(in super::super) fn from_profile(
        state: &AgentsState,
        shown: &ProfileState,
        now: SystemTime,
    ) -> Self {
        let profile = shown.profile();
        let (word, tone) = state_word(shown);
        let agent = state
            .agent(profile.agent())
            .map_or("", |agent| agent.descriptor().name());
        let (account, plan) = match shown.status() {
            ProfileStatus::Connected { account, plan, .. } => {
                (account.clone(), plan.clone())
            },
            _ => (None, None),
        };
        let plan = plan.or_else(|| limits_plan(shown));
        let hint = match shown.status() {
            ProfileStatus::NotLoggedIn => Some(Key::AgentsNotLoggedInHint),
            ProfileStatus::ProgramNotFound => {
                Some(Key::AgentsProgramNotFoundHint)
            },
            ProfileStatus::ConfigDirNotFound => {
                Some(Key::AgentsConfigDirNotFoundHint)
            },
            _ => None,
        };

        Self {
            profile: profile.clone(),
            icon: state.icon_of(profile.agent()),
            name: profile.name().to_string(),
            agent,
            state: i18n::t(word).to_string(),
            tone,
            hint: hint.map(i18n::t),
            account,
            plan,
            config_dir: profile.config_dir().display().to_string(),
            program: profile
                .program()
                .map(|program| program.display().to_string()),
            failure: shown.failures().map(failure_text),
            limits: LimitsViewModel::from_limits(shown.limits(), now),
            tokens: TokensViewModel::from_spend(shown.spend()),
            can_log_in: *shown.status() == ProfileStatus::NotLoggedIn
                && state.can_start(profile),
            can_remove: !profile.is_default(),
        }
    }
}

/// The plan the last good limits named, such as Codex's "Plus".
fn limits_plan(shown: &ProfileState) -> Option<String> {
    let (answer, _) = shown.limits().last_good()?;

    answer.value().plan().map(String::from)
}

/// The state word of a shown profile and its color: while its sessions
/// are read, when that failed, else its status.
fn state_word(shown: &ProfileState) -> (Key, SessionTone) {
    if shown.is_reading() {
        (Key::AgentsStateReading, SessionTone::Muted)
    } else if shown.failures().is_some() {
        (Key::AgentsStateReadFailed, SessionTone::Failed)
    } else {
        status_word(shown.status())
    }
}

/// The state word of a profile and its color.
fn status_word(status: &ProfileStatus) -> (Key, SessionTone) {
    match status {
        ProfileStatus::Connected { .. } => {
            (Key::AgentsStateConnected, SessionTone::Muted)
        },
        ProfileStatus::NotLoggedIn => {
            (Key::AgentsStateNotLoggedIn, SessionTone::Waiting)
        },
        ProfileStatus::ProgramNotFound => {
            (Key::AgentsStateProgramNotFound, SessionTone::Failed)
        },
        ProfileStatus::ConfigDirNotFound => {
            (Key::AgentsStateConfigDirNotFound, SessionTone::Failed)
        },
    }
}

/// The note of a failed read: each way of reading the sessions and why it
/// failed, such as "files: not available: no projects folder".
fn failure_text(failures: &[SourceFailure]) -> String {
    i18n::with_error(Key::TplAgentsSessionsFailed, &failure_reasons(failures))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{BillingKind, Limits};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        CLAUDE, at, claude, claude_work, codex, five_hour_limits,
        limits_answer, limits_loaded, now, read, spend_loaded, spend_of,
        with_profiles,
    };

    fn details(state: &AgentsState) -> ProfileDetailsViewModel {
        let shown = state.selected_profile().expect("a profile is selected");

        ProfileDetailsViewModel::from_profile(state, shown, now())
    }

    #[test]
    fn a_connected_profile_shows_its_account_plan_and_folder() {
        let account = ProfileStatus::Connected {
            account: Some(String::from("user@example.com")),
            plan: Some(String::from("Max")),
            billing: Some(BillingKind::UsageWindows),
        };
        let mut state = with_profiles(&[(claude(), account)]);
        read(&mut state, &claude(), Vec::new());
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));

        let vm = details(&state);

        assert_eq!(
            vm,
            ProfileDetailsViewModel {
                profile: claude(),
                icon: "claude-code",
                name: String::from("Claude Code"),
                agent: "Claude Code",
                state: i18n::t(Key::AgentsStateConnected).to_string(),
                tone: SessionTone::Muted,
                hint: None,
                account: Some(String::from("user@example.com")),
                plan: Some(String::from("Max")),
                config_dir: String::from("/home/user/.claude"),
                program: None,
                failure: None,
                limits: None,
                tokens: None,
                can_log_in: false,
                can_remove: false,
            }
        );
    }

    #[test]
    fn the_details_show_the_limits_and_the_plan_they_name() {
        let plain = ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: None,
        };
        let limits = Limits::new(
            five_hour_limits(42.0).billing().clone(),
            Some(String::from("Plus")),
        );
        let mut state = with_profiles(&[(codex(), plain)]);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![("usage request", Ok(limits))]),
                now(),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = details(&state);

        assert_eq!(vm.plan.as_deref(), Some("Plus"));
        let bars = vm.limits.as_ref().map(|limits| limits.bars.len());
        assert_eq!(bars, Some(1));
    }

    #[test]
    fn the_details_show_the_tokens_spent() {
        let mut state = with_profiles(&[(codex(), connected_plain())]);
        let _ = reduce(
            &mut state,
            spend_loaded(&codex(), Some(Ok(spend_of(10, 300))), at(10)),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = details(&state);

        let tokens = vm.tokens.expect("the tokens show");
        assert_eq!(tokens.today, Some(i18n::agents_tokens("10", "0", "0")));
        assert_eq!(
            tokens.last_30_days,
            Some(i18n::agents_tokens("310", "0", "0"))
        );
    }

    fn connected_plain() -> ProfileStatus {
        ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: None,
        }
    }

    #[test]
    fn an_added_profile_that_is_not_logged_in_offers_log_in_and_remove() {
        let work = claude_work().with_program(PathBuf::from("/opt/claude"));
        let mut state =
            with_profiles(&[(work.clone(), ProfileStatus::NotLoggedIn)]);
        read(&mut state, &work, Vec::new());
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(work));

        let vm = details(&state);

        assert!(vm.can_log_in);
        assert!(vm.can_remove);
        assert_eq!(vm.hint, Some(i18n::t(Key::AgentsNotLoggedInHint)));
        assert_eq!(vm.program.as_deref(), Some("/opt/claude"));
    }

    #[test]
    fn a_profile_says_what_is_missing() {
        let mut state = with_profiles(&[
            (claude(), ProfileStatus::ProgramNotFound),
            (codex(), ProfileStatus::ConfigDirNotFound),
        ]);
        read(&mut state, &claude(), Vec::new());

        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
        let program = details(&state);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let folder = details(&state);

        assert_eq!(program.hint, Some(i18n::t(Key::AgentsProgramNotFoundHint)));
        assert!(!program.can_log_in);
        assert_eq!(
            folder.hint,
            Some(i18n::t(Key::AgentsConfigDirNotFoundHint))
        );
        assert_eq!(folder.profile.agent(), crate::testing::CODEX);
        assert_ne!(folder.profile.agent(), CLAUDE);
    }

    #[test]
    fn the_details_say_the_state_of_the_profile_and_why_a_read_failed() {
        let cases = [
            (
                ProfileStatus::NotLoggedIn,
                Key::AgentsStateNotLoggedIn,
                SessionTone::Waiting,
            ),
            (
                ProfileStatus::ProgramNotFound,
                Key::AgentsStateProgramNotFound,
                SessionTone::Failed,
            ),
            (
                ProfileStatus::ConfigDirNotFound,
                Key::AgentsStateConfigDirNotFound,
                SessionTone::Failed,
            ),
        ];
        for (status, word, tone) in cases {
            let mut state = with_profiles(&[(claude(), status)]);
            read(&mut state, &claude(), Vec::new());
            let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));

            let vm = details(&state);

            assert_eq!((vm.state.as_str(), vm.tone), (i18n::t(word), tone));
        }

        let mut state = with_profiles(&[(claude(), connected_plain())]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
        let reading = details(&state);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(crate::types::LoadedSessions {
                profile: claude(),
                sessions: Err(vec![crate::testing::no_files()]),
                missing_folders: Vec::new(),
            }),
        );
        let failed = details(&state);

        assert_eq!(reading.state, i18n::t(Key::AgentsStateReading));
        assert_eq!(failed.state, i18n::t(Key::AgentsStateReadFailed));
        assert_eq!(failed.tone, SessionTone::Failed);
        assert_eq!(
            failed.failure.as_deref(),
            Some(
                i18n::with_error(
                    Key::TplAgentsSessionsFailed,
                    "files: not available: no projects folder"
                )
                .as_str()
            )
        );
    }
}
