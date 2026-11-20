//! What the button of a need does (spec "Words"): a need is a task only
//! the user can do, with one button. The Found by OTTY report, the All
//! accounts and All profiles pages and the page of an account list the
//! needs and send the same action.

use otty_agents::Profile;
use otty_app_domain::plugins::{OffPart, PluginsPlace};

use super::event::AgentsOutcome;
use super::state::AgentsState;
use super::{keyring, profile_actions, profile_dialog, session_actions};

/// What the button of a need does: the one thing the user can do about it
/// (spec "Words").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeedAction {
    /// Log the profile in: its agent logs in in a terminal tab.
    LogIn(Profile),
    /// Give the admin key of the organization found in the profile, in the
    /// profile dialog.
    AddAdminKey(Profile),
    /// Open the agent of the profile in a terminal tab, where it renews an
    /// expired login or asks to log in again (spec 2026-11-19, Decision 3).
    OpenAgent(Profile),
    /// Put this report of a changed format of an agent's files on the
    /// clipboard (spec 2026-11-19, Decision 9).
    CopyReport(String),
    /// Ask the user to allow OTTY to read the system keyring, in the
    /// keyring dialog (spec 2026-11-19, Decision 8).
    AllowKeyring,
    /// Open the Plugins page with the URL field of Add plugin… empty: a
    /// key was found for a host no provider plugin serves.
    AddPlugin,
    /// Turn on this part of a plugin, which is off.
    TurnOnPart(OffPart),
    /// Open the update dialog of the plugin with this id, whose update
    /// waits for the user's OK.
    ReviewPluginUpdate(String),
    /// Open this page in the user's browser, such as the repository of the
    /// plugin that saw a changed format.
    OpenPage(String),
}

/// Do what the button of a need asks.
pub(crate) fn act(
    state: &mut AgentsState,
    action: NeedAction,
) -> Vec<AgentsOutcome> {
    match action {
        NeedAction::LogIn(profile) => profile_actions::log_in(state, &profile),
        NeedAction::AddAdminKey(profile) => {
            profile_dialog::open_edit(state, &profile)
        },
        NeedAction::OpenAgent(profile) => {
            let home = state.env().home().to_path_buf();
            session_actions::open_tab(state, &profile, None, &home)
        },
        NeedAction::CopyReport(report) => vec![AgentsOutcome::CopyText(report)],
        NeedAction::AllowKeyring => {
            keyring::open_dialog(state);
            Vec::new()
        },
        NeedAction::AddPlugin => {
            vec![AgentsOutcome::OpenPlugins(PluginsPlace::AddPlugin)]
        },
        NeedAction::TurnOnPart(part) => {
            vec![AgentsOutcome::TurnOnPluginPart(part)]
        },
        NeedAction::ReviewPluginUpdate(plugin) => {
            vec![AgentsOutcome::OpenPlugins(PluginsPlace::Review(plugin))]
        },
        NeedAction::OpenPage(address) => vec![AgentsOutcome::OpenPage(address)],
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{AgentCommand, ProfileStatus, SavedProfiles};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, claude, codex, codex_work, connected, open_tab, organization,
        with_accounts, with_saved_profiles,
    };

    fn report_shown() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    #[test]
    fn log_in_of_a_need_on_a_page_opens_the_login_tab() {
        let mut state = with_saved_profiles(
            &[(codex(), ProfileStatus::NotLoggedIn)],
            report_shown(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::LogIn(codex())),
        );

        assert_eq!(
            outcomes,
            vec![open_tab(
                "Codex",
                "/home/user",
                AgentCommand::new("codex").with_arg("login")
            )]
        );
    }

    #[test]
    fn add_admin_key_of_a_need_on_a_page_opens_the_profile_dialog() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            report_shown(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::AddAdminKey(claude())),
        );

        assert!(outcomes.is_empty());
        assert!(state.dialog().is_some());
    }

    /// The agent opens in the home folder, as a new session of the
    /// profile, so it renews its login or asks for a new one.
    #[test]
    fn open_agent_of_a_need_opens_the_agent_in_a_terminal_tab() {
        let mut state = with_saved_profiles(
            &[(codex(), connected()), (codex_work(), connected())],
            report_shown(),
        );

        let default = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::OpenAgent(codex())),
        );
        let found = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::OpenAgent(codex_work())),
        );

        assert_eq!(
            default,
            vec![open_tab("Codex", "/home/user", AgentCommand::new("codex"))]
        );
        assert_eq!(
            found,
            vec![open_tab(
                "Codex · Work",
                "/home/user",
                AgentCommand::new("codex")
                    .with_env("CODEX_HOME", "/home/user/.codex-work"),
            )]
        );
    }

    /// The report of a changed format goes to the clipboard as it is.
    #[test]
    fn copy_report_of_a_need_puts_the_report_on_the_clipboard() {
        let mut state =
            with_saved_profiles(&[(claude(), connected())], report_shown());
        let report =
            String::from("OTTY could not read the files of Claude Code.");

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::CopyReport(report.clone())),
        );

        assert_eq!(outcomes, vec![AgentsOutcome::CopyText(report)]);
    }

    /// Spec 2026-11-19-agent-plugins, "What the UI shows": Add plugin…
    /// opens the Plugins page with the URL field empty, Review opens the
    /// update dialog, and Turn on turns the part on.
    #[test]
    fn the_buttons_of_the_plugin_needs_go_to_the_plugins_page() {
        let mut state =
            with_saved_profiles(&[(claude(), connected())], report_shown());
        let off = otty_app_domain::plugins::OffPart {
            plugin: String::from("deepseek"),
            plugin_name: String::from("DeepSeek"),
            version: String::from("0.2.0"),
            side: otty_app_domain::plugins::PartSide::Provider,
            part: String::from("deepseek"),
            part_name: String::from("DeepSeek"),
            hosts: vec![String::from("api.deepseek.com")],
        };

        let add =
            reduce(&mut state, AgentsIntent::FixNeed(NeedAction::AddPlugin));
        let review = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::ReviewPluginUpdate(
                String::from("openai"),
            )),
        );
        let turn_on = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::TurnOnPart(off.clone())),
        );

        assert_eq!(add, [AgentsOutcome::OpenPlugins(PluginsPlace::AddPlugin)]);
        assert_eq!(
            review,
            [AgentsOutcome::OpenPlugins(PluginsPlace::Review(
                String::from("openai")
            ))]
        );
        assert_eq!(turn_on, [AgentsOutcome::TurnOnPluginPart(off)]);
    }

    /// The report never opens or closes from a page: closing it saves
    /// `agents.json`, and a page has nothing to save.
    #[test]
    fn a_need_on_a_page_leaves_the_found_report_as_it_is() {
        let mut state = with_saved_profiles(
            &[(codex(), ProfileStatus::NotLoggedIn)],
            SavedProfiles::default(),
        );

        let _ = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::LogIn(codex())),
        );

        assert!(state.is_found_report_open());
    }
}
