//! The Add account dialog (spec 2026-11-23-accounts-apart-from-profiles,
//! Decision 6, with the owner's prototype): the provider of the account,
//! picked from the provider parts that are on, and how to add it. A part
//! whose manifest offers "Log in with <agent>" makes OTTY's own login:
//! when the provider is picked, the part is asked for the agent's login
//! command for a new own login's folder, and Continue makes that empty
//! folder, saves the own login in `agents.json` and runs the command in a
//! terminal tab the feature watches (`login_tabs.rs`). OTTY never handles
//! a password or a login flow itself. "Paste a key" is shown off, it comes
//! later (Decision 10). With no provider part on, the dialog offers Plugins
//! mode.

use otty_agents::{
    AgentCommand, OwnLogin, ProviderEntry, ProviderId, Registry,
};
use otty_app_domain::plugins::PluginsPlace;
use otty_app_i18n::{self as i18n, Key};

use super::event::AgentsOutcome;
use super::failure_text::source_error_text;
use super::profile_actions::{save, unread_file};
use super::state::AgentsState;
use super::types::PreparedOwnLogin;

/// What the Add account dialog was used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddAccountIntent {
    /// The Provider select was pressed: open or close its list.
    ProviderListToggled,
    /// The provider at this place of the list was picked.
    ProviderPicked(usize),
    /// Continue: log in with the agent of the picked provider.
    Continued,
    /// Cancel, the cross or Esc: close the dialog and make nothing.
    Cancelled,
    /// Manage plugins… of a dialog with no provider part on: open Plugins
    /// mode.
    PluginsOpened,
}

/// The open Add account dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddAccountDialog {
    picked: Option<PickedPart>,
    is_list_open: bool,
    ready: Option<ReadyLogin>,
    error: Option<String>,
    is_starting: bool,
}

impl AddAccountDialog {
    /// The place in the registry's providers of the picked provider part;
    /// `None` when none is picked or it is no longer on.
    pub(crate) fn picked_index(&self, registry: &Registry) -> Option<usize> {
        let picked = self.picked.as_ref()?;

        registry
            .providers()
            .iter()
            .position(|entry| picked.is(entry))
    }

    /// The entry of the picked provider part, while it is on.
    pub(crate) fn picked_entry<'a>(
        &self,
        registry: &'a Registry,
    ) -> Option<&'a ProviderEntry> {
        let index = self.picked_index(registry)?;

        registry.providers().get(index)
    }

    /// Whether the list of the Provider select is open.
    pub(crate) fn is_list_open(&self) -> bool {
        self.is_list_open
    }

    /// The own login the picked part gave its command for, ready to make.
    pub(crate) fn ready(&self) -> Option<&ReadyLogin> {
        self.ready.as_ref()
    }

    /// Why the login can not be started, ready to show.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// A dialog with `entry` picked, if any, asked nothing yet.
    fn new(entry: Option<&ProviderEntry>) -> Self {
        Self {
            picked: entry.map(PickedPart::of),
            is_list_open: false,
            ready: None,
            error: None,
            is_starting: false,
        }
    }
}

/// The provider part picked in the dialog: its plugin and its id.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PickedPart {
    plugin: String,
    provider: ProviderId,
}

impl PickedPart {
    /// The part of `entry`.
    fn of(entry: &ProviderEntry) -> Self {
        Self {
            plugin: entry.part().plugin().to_string(),
            provider: entry.id().clone(),
        }
    }

    /// Whether `entry` is this part.
    fn is(&self, entry: &ProviderEntry) -> bool {
        entry.id() == &self.provider && entry.part().plugin() == self.plugin
    }

    /// Whether `login` is an own login of this part.
    fn made(&self, login: &OwnLogin) -> bool {
        login.is_of(&self.plugin, &self.provider)
    }
}

/// A new own login and the login command its part gave for its folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReadyLogin {
    login: OwnLogin,
    command: AgentCommand,
}

impl ReadyLogin {
    /// The agent's login command pointed at the own login's folder.
    pub(crate) fn command(&self) -> &AgentCommand {
        &self.command
    }
}

/// Open the Add account dialog on the first provider part that is on,
/// and ask it for its login when it offers "Log in with".
pub(crate) fn open(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let _ = state.close_menu();
    let first = state.registry().providers().first();

    let dialog = AddAccountDialog::new(first);
    state.open_add_account(dialog);
    prepare(state)
}

/// Reduce what the dialog was used for. Nothing happens while it is
/// closed.
pub(crate) fn reduce(
    state: &mut AgentsState,
    intent: AddAccountIntent,
) -> Vec<AgentsOutcome> {
    match intent {
        AddAccountIntent::ProviderListToggled => {
            if let Some(dialog) = state.add_account_mut() {
                dialog.is_list_open = !dialog.is_list_open;
            }
            Vec::new()
        },
        AddAccountIntent::ProviderPicked(index) => pick(state, index),
        AddAccountIntent::Continued => start(state),
        AddAccountIntent::Cancelled => {
            let _ = state.take_add_account();
            Vec::new()
        },
        AddAccountIntent::PluginsOpened => {
            let _ = state.take_add_account();
            vec![AgentsOutcome::OpenPlugins(PluginsPlace::List)]
        },
    }
}

/// Take the login command the part gave for a new own login: it is kept
/// while that part is still picked, and its failure shows in the dialog.
pub(crate) fn prepared(
    state: &mut AgentsState,
    answer: PreparedOwnLogin,
) -> Vec<AgentsOutcome> {
    let Some(dialog) = state.add_account_mut() else {
        return Vec::new();
    };
    let is_picked = dialog
        .picked
        .as_ref()
        .is_some_and(|picked| picked.made(&answer.login));
    if !is_picked || dialog.is_starting {
        return Vec::new();
    }

    match answer.command {
        Ok(command) => {
            dialog.ready = Some(ReadyLogin {
                login: answer.login,
                command,
            });
            dialog.error = None;
        },
        Err(error) => dialog.error = Some(source_error_text(&error)),
    }
    Vec::new()
}

/// Once the folder of the own login is made: close the dialog, save the
/// own login in `agents.json` and run its login command in a tab the
/// feature watches. A folder that could not be made stays in the dialog
/// with why.
pub(crate) fn folder_made(
    state: &mut AgentsState,
    login: OwnLogin,
    made: Result<(), String>,
) -> Vec<AgentsOutcome> {
    let Some(dialog) = state.add_account_mut() else {
        return Vec::new();
    };
    let is_this_login = dialog
        .ready
        .as_ref()
        .is_some_and(|ready| ready.login == login);
    if !dialog.is_starting || !is_this_login {
        return Vec::new();
    }

    if let Err(error) = made {
        dialog.error = Some(error);
        dialog.is_starting = false;
        return Vec::new();
    }

    let Some(ReadyLogin { login, command }) =
        state.take_add_account().and_then(|dialog| dialog.ready)
    else {
        return Vec::new();
    };
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };
    let title = log_in_label(state.registry(), &login);
    let folder = state.env().home().to_path_buf();

    let changed = saved.clone().with_own_login(login.clone());
    state.profiles_mut().apply_saved(changed.clone());
    vec![
        save(changed),
        AgentsOutcome::OpenLoginTab {
            login,
            title,
            folder,
            command,
        },
    ]
}

/// Whether Continue acts: the picked part gave its login command, no
/// folder is being made, and `agents.json` was read, so the own login can
/// be saved.
pub(crate) fn can_continue(state: &AgentsState) -> bool {
    let Some(dialog) = state.add_account() else {
        return false;
    };

    dialog.ready.is_some()
        && !dialog.is_starting
        && state.profiles().saved().is_some()
}

/// "Log in with <agent>" of the part that made `login`, such as "Log in
/// with Codex": the label of its choice and the title of its tab.
pub(crate) fn log_in_label(registry: &Registry, login: &OwnLogin) -> String {
    let name = registry
        .providers()
        .iter()
        .filter(|entry| login.is_of(entry.part().plugin(), entry.id()))
        .find_map(|entry| entry.log_in())
        .map_or_else(
            || login.agent().as_str().to_string(),
            |with| with.name().to_string(),
        );

    i18n::filled(Key::TplAgentsLogInWith, &[("agent", &name)])
}

/// Pick the provider at `index` of the registry's providers, close the
/// list and ask the new part for its login.
fn pick(state: &mut AgentsState, index: usize) -> Vec<AgentsOutcome> {
    let entry = state.registry().providers().get(index).cloned();
    let Some(dialog) = state.add_account_mut() else {
        return Vec::new();
    };
    if dialog.is_starting {
        return Vec::new();
    }

    *dialog = AddAccountDialog::new(entry.as_ref());
    prepare(state)
}

/// The ask of the picked part for a new own login, when it offers "Log in
/// with"; none for a part that does not.
fn prepare(state: &AgentsState) -> Vec<AgentsOutcome> {
    let entry = state
        .add_account()
        .and_then(|dialog| dialog.picked_entry(state.registry()));
    let Some((entry, log_in)) =
        entry.and_then(|entry| Some((entry, entry.log_in()?)))
    else {
        return Vec::new();
    };

    vec![AgentsOutcome::PrepareOwnLogin {
        plugin: entry.part().plugin().to_string(),
        provider: entry.id().clone(),
        agent: log_in.agent().clone(),
    }]
}

/// Continue: make the folder of the ready own login first.
fn start(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !can_continue(state) {
        return Vec::new();
    }
    let Some(dialog) = state.add_account_mut() else {
        return Vec::new();
    };
    let Some(ready) = &dialog.ready else {
        return Vec::new();
    };

    let login = ready.login.clone();
    dialog.is_starting = true;
    dialog.error = None;
    vec![AgentsOutcome::MakeOwnLoginFolder(login)]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use otty_agents::{
        AgentCommand, OwnLogin, ProviderId, Registry, SavedProfiles,
        SourceError, Unavailable,
    };
    use otty_app_domain::plugins::PluginsPlace;
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::{AgentsIntent, AgentsOutcome};
    use crate::reducer::reduce as reduce_all;
    use crate::testing::{
        claude, claude_id, codex_id, connected, with_registry_and_profiles,
        with_saved_profiles,
    };
    use crate::types::PreparedOwnLogin;

    fn send(
        state: &mut AgentsState,
        intent: AddAccountIntent,
    ) -> Vec<AgentsOutcome> {
        reduce_all(state, AgentsIntent::AddAccount(intent))
    }

    /// Agents mode with Claude Code read and `agents.json` read.
    fn agents_mode() -> AgentsState {
        with_saved_profiles(
            &[(claude(), connected())],
            SavedProfiles::default(),
        )
    }

    /// The own login OTTY would make with the Anthropic part.
    fn anthropic_login() -> OwnLogin {
        OwnLogin::new(
            "6f0c1e2a",
            "anthropic",
            ProviderId::new("anthropic"),
            claude_id(),
        )
    }

    /// The own login OTTY would make with the OpenAI part.
    fn openai_login() -> OwnLogin {
        OwnLogin::new(
            "7a1d2b3c",
            "openai",
            ProviderId::new("openai"),
            codex_id(),
        )
    }

    /// `claude auth login --claudeai` in the folder of [`anthropic_login`].
    fn claude_login_command() -> AgentCommand {
        AgentCommand::new("claude")
            .with_arg("auth")
            .with_arg("login")
            .with_arg("--claudeai")
            .with_env(
                "CLAUDE_CONFIG_DIR",
                "/home/user/.local/share/otty/accounts/6f0c1e2a",
            )
    }

    fn prepared(login: OwnLogin, command: AgentCommand) -> AgentsIntent {
        AgentsIntent::OwnLoginPrepared(PreparedOwnLogin {
            login,
            command: Ok(command),
        })
    }

    /// Agents mode with the Add account dialog open and the Anthropic
    /// login ready.
    fn ready_to_log_in() -> AgentsState {
        let mut state = agents_mode();
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);
        let _ = reduce_all(
            &mut state,
            prepared(anthropic_login(), claude_login_command()),
        );

        state
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6: the
    /// dialog opens on the first provider part that is on and asks it at
    /// once for the login of a new own login.
    #[test]
    fn opening_the_dialog_asks_the_first_part_for_its_login() {
        let mut state = agents_mode();

        let opened = reduce_all(&mut state, AgentsIntent::OpenAddAccount);

        let dialog = state.add_account().expect("the dialog is open");
        assert_eq!(dialog.picked_index(state.registry()), Some(0));
        assert_eq!(
            opened,
            [AgentsOutcome::PrepareOwnLogin {
                plugin: String::from("anthropic"),
                provider: ProviderId::new("anthropic"),
                agent: claude_id(),
            }]
        );
    }

    /// Decision 6: a provider part that offers no "Log in with" is asked
    /// for nothing and can not continue.
    #[test]
    fn a_part_with_no_agent_login_is_asked_nothing_and_can_not_continue() {
        let mut state = agents_mode();
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);

        let picked = send(&mut state, AddAccountIntent::ProviderPicked(2));
        let continued = send(&mut state, AddAccountIntent::Continued);

        assert!(picked.is_empty(), "{picked:?}");
        assert!(continued.is_empty(), "{continued:?}");
        assert!(!can_continue(&state));
    }

    /// Picking another part that offers it asks that part, and an answer
    /// for a part no longer picked is dropped.
    #[test]
    fn picking_another_part_asks_it_and_drops_an_old_answer() {
        let mut state = agents_mode();
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);
        let _ = send(&mut state, AddAccountIntent::ProviderListToggled);

        let picked = send(&mut state, AddAccountIntent::ProviderPicked(1));
        let _ = reduce_all(
            &mut state,
            prepared(anthropic_login(), claude_login_command()),
        );

        let dialog = state.add_account().expect("the dialog is open");
        assert!(!dialog.is_list_open());
        assert_eq!(dialog.ready(), None);
        assert_eq!(
            picked,
            [AgentsOutcome::PrepareOwnLogin {
                plugin: String::from("openai"),
                provider: ProviderId::new("openai"),
                agent: codex_id(),
            }]
        );
        assert!(!can_continue(&state));
    }

    /// Decision 6: Continue makes the folder first; nothing is saved or
    /// opened before it is there.
    #[test]
    fn continue_makes_the_folder_of_the_own_login() {
        let mut state = ready_to_log_in();

        let continued = send(&mut state, AddAccountIntent::Continued);
        let again = send(&mut state, AddAccountIntent::Continued);

        assert_eq!(
            continued,
            [AgentsOutcome::MakeOwnLoginFolder(anthropic_login())]
        );
        assert!(again.is_empty(), "one folder at a time");
    }

    /// Decision 6: once the folder is there, the own login is saved in
    /// `agents.json` and its login command runs in a tab the feature
    /// watches, with the folder variable of the part.
    #[test]
    fn a_made_folder_saves_the_own_login_and_opens_its_login_tab() {
        let mut state = ready_to_log_in();
        let _ = send(&mut state, AddAccountIntent::Continued);

        let made = reduce_all(
            &mut state,
            AgentsIntent::OwnLoginFolderMade {
                login: anthropic_login(),
                made: Ok(()),
            },
        );
        let _ = reduce_all(
            &mut state,
            AgentsIntent::LoginTabOpened {
                tab_id: 7,
                login: anthropic_login(),
            },
        );

        let saved = SavedProfiles::default().with_own_login(anthropic_login());
        let title =
            i18n::filled(Key::TplAgentsLogInWith, &[("agent", "Claude Code")]);
        assert_eq!(
            made,
            [
                AgentsOutcome::SaveProfiles {
                    saved: saved.clone(),
                    create_dir: None,
                },
                AgentsOutcome::OpenLoginTab {
                    login: anthropic_login(),
                    title,
                    folder: PathBuf::from("/home/user"),
                    command: claude_login_command(),
                },
            ]
        );
        assert_eq!(state.add_account(), None);
        assert_eq!(state.profiles().saved(), Some(&saved));
        assert!(state.is_fix_tab(7));
    }

    /// A folder that could not be made stays in the dialog with why, and
    /// Continue may be pressed again.
    #[test]
    fn a_folder_that_could_not_be_made_stays_in_the_dialog() {
        let mut state = ready_to_log_in();
        let _ = send(&mut state, AddAccountIntent::Continued);

        let made = reduce_all(
            &mut state,
            AgentsIntent::OwnLoginFolderMade {
                login: anthropic_login(),
                made: Err(String::from("disk full")),
            },
        );

        let dialog = state.add_account().expect("the dialog stays");
        assert!(made.is_empty(), "{made:?}");
        assert_eq!(dialog.error(), Some("disk full"));
        assert!(can_continue(&state));
    }

    /// A part that could not give its command says why, and Continue
    /// stays off.
    #[test]
    fn a_part_that_gives_no_command_says_why() {
        let mut state = agents_mode();
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);

        let _ = reduce_all(
            &mut state,
            AgentsIntent::OwnLoginPrepared(PreparedOwnLogin {
                login: anthropic_login(),
                command: Err(SourceError::NotAvailable(Unavailable::NoLogIn)),
            }),
        );

        let dialog = state.add_account().expect("the dialog is open");
        let why =
            source_error_text(&SourceError::NotAvailable(Unavailable::NoLogIn));
        assert_eq!(dialog.error(), Some(why.as_str()));
        assert!(!can_continue(&state));
    }

    /// With no provider part on, the dialog offers Plugins mode.
    #[test]
    fn with_no_provider_part_the_dialog_offers_plugins_mode() {
        let registry = Registry::new(
            crate::testing::registry().agents().to_vec(),
            Vec::new(),
        );
        let mut state = with_registry_and_profiles(
            Arc::new(registry),
            &[(claude(), connected())],
        );

        let opened = reduce_all(&mut state, AgentsIntent::OpenAddAccount);
        let plugins = send(&mut state, AddAccountIntent::PluginsOpened);

        assert!(opened.is_empty(), "{opened:?}");
        assert_eq!(plugins, [AgentsOutcome::OpenPlugins(PluginsPlace::List)]);
        assert_eq!(state.add_account(), None);
    }

    #[test]
    fn cancel_closes_the_dialog_and_saves_nothing() {
        let mut state = ready_to_log_in();

        let cancelled = send(&mut state, AddAccountIntent::Cancelled);

        assert!(cancelled.is_empty());
        assert_eq!(state.add_account(), None);
        assert_eq!(state.profiles().saved(), Some(&SavedProfiles::default()));
    }

    /// The answer of another dialog's part, such as one opened before, is
    /// not taken.
    #[test]
    fn the_answer_of_a_part_not_picked_is_not_taken() {
        let mut state = agents_mode();
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);

        let _ = reduce_all(
            &mut state,
            prepared(openai_login(), AgentCommand::new("codex")),
        );

        assert_eq!(state.add_account().and_then(AddAccountDialog::ready), None);
    }
}
