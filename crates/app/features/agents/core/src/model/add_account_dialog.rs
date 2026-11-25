//! The Add account dialog (spec 2026-11-23-accounts-apart-from-profiles,
//! Decision 6, with the owner's prototype): the Provider select of the
//! provider parts that are on, each with its logo, its name and its
//! plugin, and under How the ways to add the picked one: "Log in with
//! <agent>" when its part offers it, and "Paste a key", off until it comes
//! (Decision 10).

use std::path::Path;

use otty_agents::{AgentCommand, ProviderEntry};
use otty_app_i18n::{self as i18n, Key};

use super::super::add_account_dialog::can_continue;
use super::super::state::AgentsState;
use super::EntryIcon;

/// The open Add account dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddAccountDialogViewModel {
    /// "Add account".
    pub(crate) title: &'static str,
    /// The line over the select: the list comes from the installed
    /// plugins.
    pub(crate) intro: &'static str,
    /// The provider parts that are on, in the order of the registry.
    pub(crate) providers: Vec<ProviderOptionViewModel>,
    /// The place of the picked provider in `providers`.
    pub(crate) picked: Option<usize>,
    /// Whether the list of the select is open.
    pub(crate) is_list_open: bool,
    /// "Log in with <agent>", when the picked part offers it.
    pub(crate) log_in: Option<LogInChoiceViewModel>,
    /// "Paste a key", off.
    pub(crate) paste_key: PasteKeyChoiceViewModel,
    /// The line of a provider no agent logs in to.
    pub(crate) no_agent_login: Option<String>,
    /// The text of a dialog with no provider part on, which offers Plugins
    /// mode in place of the select.
    pub(crate) no_providers: Option<&'static str>,
    /// Why the login can not be started, ready to show.
    pub(crate) error: Option<String>,
    /// Whether Continue acts.
    pub(crate) can_continue: bool,
}

impl AddAccountDialogViewModel {
    /// The open dialog; `None` while it is closed.
    pub(crate) fn from_state(state: &AgentsState) -> Option<Self> {
        let dialog = state.add_account()?;

        let registry = state.registry();
        let providers: Vec<ProviderOptionViewModel> = registry
            .providers()
            .iter()
            .map(ProviderOptionViewModel::of)
            .collect();
        let picked = dialog.picked_entry(registry);
        let provider_name = picked.map_or("", ProviderEntry::name);

        let log_in = picked.and_then(ProviderEntry::log_in).map(|with| {
            LogInChoiceViewModel {
                label: i18n::filled(
                    Key::TplAgentsLogInWith,
                    &[("agent", with.name())],
                ),
                text: dialog.ready().map(|ready| {
                    i18n::filled(
                        Key::TplAgentsLogInWithText,
                        &[("command", &command_text(ready.command()))],
                    )
                }),
            }
        });
        let no_agent_login = picked
            .filter(|entry| entry.log_in().is_none())
            .map(|entry| {
                i18n::filled(
                    Key::TplAgentsNoAgentLogin,
                    &[("provider", entry.name())],
                )
            });
        let no_providers = providers
            .is_empty()
            .then(|| i18n::t(Key::AgentsNoProviderPart));

        Some(Self {
            title: i18n::t(Key::AgentsAddAccountTitle),
            intro: i18n::t(Key::AgentsAddAccountIntro),
            picked: dialog.picked_index(registry),
            providers,
            is_list_open: dialog.is_list_open(),
            log_in,
            paste_key: PasteKeyChoiceViewModel {
                label: i18n::t(Key::AgentsPasteKey),
                text: i18n::filled(
                    Key::TplAgentsPasteKeyText,
                    &[("provider", provider_name)],
                ),
                reason: i18n::t(Key::AgentsComingLater),
            },
            no_agent_login,
            no_providers,
            error: dialog.error().map(String::from),
            can_continue: can_continue(state),
        })
    }
}

/// One provider part of the select: its name, its plugin and its logo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderOptionViewModel {
    /// The name of the provider, such as "OpenAI".
    pub(crate) name: String,
    /// Its plugin, such as "OpenAI plugin".
    pub(crate) plugin: String,
    /// The logo of the provider.
    pub(crate) icon: EntryIcon,
}

impl ProviderOptionViewModel {
    fn of(entry: &ProviderEntry) -> Self {
        Self {
            name: entry.name().to_string(),
            plugin: i18n::filled(
                Key::TplAgentsNeedPluginLabel,
                &[("plugin", entry.part().name())],
            ),
            icon: EntryIcon::of(entry.icon(), entry.name()),
        }
    }
}

/// "Log in with <agent>": the way OTTY's own login is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogInChoiceViewModel {
    /// "Log in with Codex".
    pub(crate) label: String,
    /// The line that names the part's command, once the part gave it.
    pub(crate) text: Option<String>,
}

/// "Paste a key", shown off with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PasteKeyChoiceViewModel {
    /// "Paste a key".
    pub(crate) label: &'static str,
    /// The line under it, which names the provider.
    pub(crate) text: String,
    /// Why it is off: "Coming later".
    pub(crate) reason: &'static str,
}

/// `command` as the dialog names it: the name of its program and its
/// arguments, such as `codex login`, with no variable.
fn command_text(command: &AgentCommand) -> String {
    let program = Path::new(command.program()).file_name().map_or_else(
        || command.program().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );

    std::iter::once(program)
        .chain(command.args().iter().cloned())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use otty_agents::{
        AgentCommand, OwnLogin, ProviderId, Registry, SavedProfiles,
    };
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::add_account_dialog::AddAccountIntent;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        claude, claude_id, connected, provider_icon,
        with_registry_and_profiles, with_saved_profiles,
    };
    use crate::types::PreparedOwnLogin;

    /// Agents mode with the Add account dialog open.
    fn with_dialog() -> AgentsState {
        let mut state = with_saved_profiles(
            &[(claude(), connected())],
            SavedProfiles::default(),
        );
        let _ = reduce(&mut state, AgentsIntent::OpenAddAccount);

        state
    }

    fn vm(state: &AgentsState) -> AddAccountDialogViewModel {
        AddAccountDialogViewModel::from_state(state)
            .expect("the dialog is open")
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6, with the
    /// owner's prototype: the Provider select lists every provider part
    /// that is on, each with its logo, its name and its plugin.
    #[test]
    fn the_select_lists_every_provider_part_with_its_logo_and_plugin() {
        let state = with_dialog();

        let vm = vm(&state);

        let names: Vec<&str> = vm
            .providers
            .iter()
            .map(|option| option.name.as_str())
            .collect();
        let plugins: Vec<&str> = vm
            .providers
            .iter()
            .map(|option| option.plugin.as_str())
            .collect();
        let plugin = |name: &str| {
            i18n::filled(Key::TplAgentsNeedPluginLabel, &[("plugin", name)])
        };
        assert_eq!(names, ["Anthropic", "OpenAI", "Tealcloud", "Hopway"]);
        assert_eq!(
            plugins,
            [
                plugin("Anthropic"),
                plugin("OpenAI"),
                plugin("Tealcloud"),
                plugin("Hopway")
            ]
        );
        assert_eq!(vm.providers[0].icon, provider_icon("anthropic"));
        assert_eq!(vm.picked, Some(0));
        assert_eq!(vm.title, i18n::t(Key::AgentsAddAccountTitle));
    }

    /// Decision 6: "Log in with <agent>" is offered for a part that says
    /// so, with the line that names the part's command once it gave it.
    #[test]
    fn log_in_with_names_the_agent_and_the_parts_command() {
        let mut state = with_dialog();
        let waiting = vm(&state);
        let login = OwnLogin::new(
            "6f0c1e2a",
            "anthropic",
            ProviderId::new("anthropic"),
            claude_id(),
        );
        let command = AgentCommand::new("claude")
            .with_arg("auth")
            .with_arg("login")
            .with_arg("--claudeai")
            .with_env("CLAUDE_CONFIG_DIR", "/home/user/x");

        let _ = reduce(
            &mut state,
            AgentsIntent::OwnLoginPrepared(PreparedOwnLogin {
                login,
                command: Ok(command),
            }),
        );
        let ready = vm(&state);

        let label =
            i18n::filled(Key::TplAgentsLogInWith, &[("agent", "Claude Code")]);
        let line = i18n::filled(
            Key::TplAgentsLogInWithText,
            &[("command", "claude auth login --claudeai")],
        );
        let log_in = ready.log_in.expect("Log in with is offered");
        assert_eq!(log_in.label, label);
        assert_eq!(log_in.text, Some(line));
        assert_eq!(waiting.log_in.and_then(|log_in| log_in.text), None);
        assert!(!waiting.can_continue);
        assert!(ready.can_continue);
        assert_eq!(ready.no_agent_login, None);
    }

    /// Decision 10 and the owner's prototype: Paste a key is shown off,
    /// saying "Coming later", for every provider.
    #[test]
    fn paste_a_key_is_shown_off_with_coming_later() {
        let state = with_dialog();

        let vm = vm(&state);

        assert_eq!(vm.paste_key.label, i18n::t(Key::AgentsPasteKey));
        assert_eq!(vm.paste_key.reason, i18n::t(Key::AgentsComingLater));
        assert_eq!(
            vm.paste_key.text,
            i18n::filled(
                Key::TplAgentsPasteKeyText,
                &[("provider", "Anthropic")]
            )
        );
    }

    /// A provider with no agent login says it, and can not continue.
    #[test]
    fn a_provider_with_no_agent_login_says_so() {
        let mut state = with_dialog();
        let _ = reduce(
            &mut state,
            AgentsIntent::AddAccount(AddAccountIntent::ProviderPicked(2)),
        );

        let vm = vm(&state);

        assert_eq!(vm.log_in, None);
        assert_eq!(
            vm.no_agent_login,
            Some(i18n::filled(
                Key::TplAgentsNoAgentLogin,
                &[("provider", "Tealcloud")]
            ))
        );
        assert!(!vm.can_continue);
    }

    /// With no provider part on, the dialog says so and offers Plugins
    /// mode in place of the select.
    #[test]
    fn with_no_provider_part_the_dialog_says_so() {
        let registry = Registry::new(
            crate::testing::registry().agents().to_vec(),
            Vec::new(),
        );
        let mut state = with_registry_and_profiles(
            Arc::new(registry),
            &[(claude(), connected())],
        );
        let _ = reduce(&mut state, AgentsIntent::OpenAddAccount);

        let vm = vm(&state);

        assert!(vm.providers.is_empty());
        assert_eq!(vm.no_providers, Some(i18n::t(Key::AgentsNoProviderPart)));
        assert!(!vm.can_continue);
    }
}
