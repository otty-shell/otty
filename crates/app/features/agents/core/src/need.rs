//! What the button of a need does (spec "Words"): a need is a task only
//! the user can do, with one button. The Found by OTTY report, the All
//! accounts and All profiles pages and the page of an account list the
//! needs and send the same action. A need of a plugin part does the one
//! thing its part asked (spec 2026-11-19-agent-plugins, issue 20).

use otty_agents::{Account, AgentCommand, KeyAsk};
use otty_app_domain::plugins::{OffPart, PluginsPlace};

use super::event::AgentsOutcome;
use super::state::AgentsState;
use super::{key_dialog, keyring};

/// What the button of a need does: the one thing the user can do about it
/// (spec "Words").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeedAction {
    /// Run the command a plugin part gave in a terminal tab titled
    /// `title`, in the home folder, such as the agent's login, the agent
    /// itself, which renews its login, or the install of its program. The
    /// profiles are read again once the tab closes.
    Run {
        /// The title of the tab: the account or the profile of the need.
        title: String,
        /// The program, arguments and variables the part gave.
        command: AgentCommand,
    },
    /// Open the field of the key a plugin part asks for, given for
    /// `account`, in the key dialog.
    AskKey {
        /// The account the key is given for.
        account: Account,
        /// The name of the plugin that asks.
        plugin: String,
        /// The key the part asks for: its slot, label, hint and page.
        ask: KeyAsk,
    },
    /// Put this report on the clipboard, such as the report of a changed
    /// format of an agent's files (spec 2026-11-19, Decision 9).
    CopyReport(String),
    /// Ask the user to allow OTTY to read the system keyring, in the
    /// keyring dialog (spec 2026-11-19, Decision 8).
    AllowKeyring,
    /// Open Plugins mode with the URL field of Add plugin… empty: a
    /// key was found for a host no provider plugin serves.
    AddPlugin,
    /// Turn on this part of a plugin, which is off.
    TurnOnPart(OffPart),
    /// Open the update dialog of the plugin with this id, whose update
    /// waits for the user's OK.
    ReviewPluginUpdate(String),
    /// Open this page in the user's browser, such as the repository of the
    /// plugin that saw a changed format, or the page a part's need names.
    OpenPage(String),
}

/// Do what the button of a need asks.
pub(crate) fn act(
    state: &mut AgentsState,
    action: NeedAction,
) -> Vec<AgentsOutcome> {
    match action {
        NeedAction::Run { title, command } => vec![AgentsOutcome::OpenFixTab {
            title,
            folder: state.env().home().to_path_buf(),
            command,
        }],
        NeedAction::AskKey {
            account,
            plugin,
            ask,
        } => key_dialog::open(state, account, plugin, ask),
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
    use otty_agents::{AgentCommand, SavedProfiles};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, claude, codex, connected, key_ask, open_fix_tab, organization,
        with_accounts, with_saved_profiles,
    };

    fn report_shown() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    /// Issue 20: the command a part gave runs in a terminal tab in the
    /// home folder, titled after the place of the need, as the part gave
    /// it.
    #[test]
    fn run_of_a_need_runs_the_command_of_the_part_in_a_terminal_tab() {
        let mut state = with_saved_profiles(
            &[(codex(), crate::testing::not_logged_in(&codex()))],
            report_shown(),
        );
        let login = AgentCommand::new("codex")
            .with_arg("login")
            .with_env("CODEX_HOME", "/home/user/.codex-work");

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::Run {
                title: String::from("Codex · Work"),
                command: login.clone(),
            }),
        );

        assert_eq!(
            outcomes,
            vec![open_fix_tab("Codex · Work", "/home/user", login)]
        );
    }

    /// Issue 32: once the tab of a need closes, such as the install of
    /// the agent's program or its login, the profiles are read again, so
    /// a program just installed is found; another tab that closes reads
    /// nothing.
    #[test]
    fn the_profiles_are_read_again_once_the_tab_of_a_need_closes() {
        let mut state = with_saved_profiles(
            &[(codex(), otty_agents::ProfileStatus::ProgramNotFound)],
            report_shown(),
        );
        let _ = reduce(&mut state, AgentsIntent::FixTabOpened(7));

        let other = reduce(&mut state, AgentsIntent::TabClosed(3));
        let fix = reduce(&mut state, AgentsIntent::TabClosed(7));
        let _ = reduce(
            &mut state,
            crate::testing::profiles_loaded(&[(
                codex(),
                otty_agents::ProfileStatus::ProgramNotFound,
            )]),
        );
        let again = reduce(&mut state, AgentsIntent::TabClosed(7));

        assert_eq!(other, []);
        assert_eq!(fix, [AgentsOutcome::LoadProfiles]);
        assert_eq!(again, []);
    }

    /// Issue 20: the key a part asks for opens its dialog, for the
    /// account.
    #[test]
    fn ask_key_of_a_need_opens_the_key_dialog_of_the_account() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            report_shown(),
        );
        let ask = |account| NeedAction::AskKey {
            account,
            plugin: String::from("Anthropic"),
            ask: key_ask(),
        };
        let outcomes =
            reduce(&mut state, AgentsIntent::FixNeed(ask(organization())));

        assert!(outcomes.is_empty());
        assert!(
            state
                .key_dialog()
                .is_some_and(|dialog| dialog.ask().slot() == "admin")
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
    /// opens Plugins mode with the URL field empty, Review opens the
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
            &[(codex(), crate::testing::not_logged_in(&codex()))],
            SavedProfiles::default(),
        );

        let _ = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::Run {
                title: String::from("Codex"),
                command: AgentCommand::new("codex").with_arg("login"),
            }),
        );

        assert!(state.is_found_report_open());
    }
}
