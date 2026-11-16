use otty_agents::{Profile, ProfileStatus};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::state::{AgentsState, ProfileState};
use super::left_panel::{failure_text, state_word};

/// The details of the selected profile in the main area (spec Decision 6):
/// its agent, name, state, account and plan, config folder and buttons.
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
    /// The plan of the account, when known.
    pub(crate) plan: Option<String>,
    /// The config folder.
    pub(crate) config_dir: String,
    /// The program path the user set, if any.
    pub(crate) program: Option<String>,
    /// Why the sessions could not be read, when the last read failed.
    pub(crate) failure: Option<String>,
    /// Whether Log in shows: the profile is not logged in and its program
    /// was found.
    pub(crate) can_log_in: bool,
    /// Whether Remove shows: only an added profile can be removed.
    pub(crate) can_remove: bool,
}

impl ProfileDetailsViewModel {
    /// Build the details of the shown profile `shown`.
    pub(in super::super) fn from_profile(
        state: &AgentsState,
        shown: &ProfileState,
    ) -> Self {
        let profile = shown.profile();
        let (word, tone) = state_word(shown);
        let agent = state
            .agent(profile.agent())
            .map_or("", |agent| agent.descriptor().name());
        let (account, plan) = match shown.status() {
            ProfileStatus::Connected { account, plan } => {
                (account.clone(), plan.clone())
            },
            _ => (None, None),
        };
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
            can_log_in: *shown.status() == ProfileStatus::NotLoggedIn
                && state.can_start(profile),
            can_remove: !profile.is_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        CLAUDE, claude, claude_work, codex, read, with_profiles,
    };

    fn details(state: &AgentsState) -> ProfileDetailsViewModel {
        let shown = state.selected_profile().expect("a profile is selected");

        ProfileDetailsViewModel::from_profile(state, shown)
    }

    #[test]
    fn a_connected_profile_shows_its_account_plan_and_folder() {
        let account = ProfileStatus::Connected {
            account: Some(String::from("user@example.com")),
            plan: Some(String::from("Max")),
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
                can_log_in: false,
                can_remove: false,
            }
        );
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
}
