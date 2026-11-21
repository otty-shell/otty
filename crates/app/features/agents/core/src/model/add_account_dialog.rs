//! The Add account dialog, the Edit key dialog of an account the user
//! added (spec 2026-11-19, Decision 7) and the dialog of a key a plugin
//! part asks for (spec 2026-11-19-agent-plugins, issue 20): the providers,
//! where the key is kept, and why it can not be saved yet.

use otty_agents::{KeyRef, ProviderRef, Registry};
use otty_app_i18n::{self as i18n, Key};

use super::super::add_account_dialog::can_submit;
use super::super::key_place_field::{KeyPlaceChoice, KeyPlaceProblem};
use super::super::state::{AgentsState, provider_name};
use super::needs::open_page_label;

/// A provider an account can be added for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderOption {
    /// The provider.
    pub(crate) provider: ProviderRef,
    /// Its name, such as "Example Cloud".
    pub(crate) name: String,
}

/// The open Add account, Edit key or asked key dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddAccountDialogViewModel<'a> {
    /// "Add account", "Edit key", or the label of the key a part asks
    /// for, such as "Admin key".
    pub(crate) title: String,
    /// What the dialog is for and what OTTY saves, or the hint of the key
    /// a part asks for.
    pub(crate) text: String,
    /// The plugin that asks for the key, such as "OpenAI plugin"; `None`
    /// for Add account and Edit key.
    pub(crate) plugin: Option<String>,
    /// What the key a part asks for starts with, as a line.
    pub(crate) prefix: Option<String>,
    /// The label of the button that opens the page where the key is made.
    pub(crate) page: Option<String>,
    /// The providers to pick from; empty while the provider stays.
    pub(crate) providers: Vec<ProviderOption>,
    /// The provider picked.
    pub(crate) provider: ProviderOption,
    /// Whether the list of the providers is open.
    pub(crate) is_provider_list_open: bool,
    /// Where the key is kept: a file or a variable.
    pub(crate) key_choice: KeyPlaceChoice,
    /// The typed path or variable name.
    pub(crate) key: &'a str,
    /// An example of what to type, for the picked provider.
    pub(crate) key_placeholder: String,
    /// What is wrong with the typed place.
    pub(crate) key_error: Option<&'static str>,
    /// Why the account was not saved.
    pub(crate) error: Option<String>,
    /// "Add" or "Save".
    pub(crate) submit_label: &'static str,
    /// Whether Add or Save acts.
    pub(crate) can_submit: bool,
}

impl<'a> AddAccountDialogViewModel<'a> {
    /// The open dialog; `None` while it is closed.
    pub(crate) fn from_state(state: &'a AgentsState) -> Option<Self> {
        let dialog = state.add_account()?;

        let asked = dialog.asked();
        let is_fixed = dialog.editing().is_some() || asked.is_some();
        let (title, text, submit_label) = match (asked, dialog.editing()) {
            (Some(asked), _) => (
                asked.ask().label().to_string(),
                asked.ask().hint().to_string(),
                Key::ButtonSave,
            ),
            (None, Some(_)) => (
                i18n::t(Key::AgentsEditKeyTitle).to_string(),
                i18n::t(Key::AgentsAddAccountText).to_string(),
                Key::ButtonSave,
            ),
            (None, None) => (
                i18n::t(Key::AgentsAddAccountTitle).to_string(),
                i18n::t(Key::AgentsAddAccountText).to_string(),
                Key::ButtonAdd,
            ),
        };
        let providers = if is_fixed {
            Vec::new()
        } else {
            state
                .registry()
                .addable_providers()
                .iter()
                .map(|entry| {
                    option_of(
                        state.registry(),
                        &ProviderRef::Id(entry.id().clone()),
                    )
                })
                .collect()
        };
        let key_error = match dialog.key().key_place(state.env().home()) {
            Err(KeyPlaceProblem::KeyPathNotFull) => {
                Some(i18n::t(Key::AgentsErrorKeyPathNotFull))
            },
            Err(KeyPlaceProblem::VariableName) => {
                Some(i18n::t(Key::AgentsErrorVariableName))
            },
            Ok(_) => None,
        };
        let slot = asked.and_then(|asked| asked.ask().slot());

        Some(Self {
            title,
            text,
            plugin: asked.map(|asked| {
                i18n::filled(
                    Key::TplAgentsNeedPluginLabel,
                    &[("plugin", asked.plugin())],
                )
            }),
            prefix: asked.and_then(|asked| asked.ask().prefix()).map(
                |prefix| {
                    i18n::filled(
                        Key::TplAgentsKeyStartsWith,
                        &[("prefix", prefix)],
                    )
                },
            ),
            page: asked
                .and_then(|asked| asked.ask().page())
                .map(open_page_label),
            providers,
            provider: option_of(state.registry(), dialog.provider()),
            is_provider_list_open: dialog.is_provider_list_open(),
            key_choice: dialog.key().choice(),
            key: dialog.key().text(),
            key_placeholder: placeholder(
                state.registry(),
                dialog.provider(),
                slot,
                dialog.key().choice(),
            ),
            key_error,
            error: dialog.error().map(String::from),
            submit_label: i18n::t(submit_label),
            can_submit: can_submit(state),
        })
    }
}

/// The text of the place of a key the user added, as an account is shown
/// by it: the path of its file, or `$NAME` of its variable.
pub(crate) fn key_place_text(key: &KeyRef) -> String {
    match key {
        KeyRef::File(path) => path.display().to_string(),
        KeyRef::EnvVar(name) => format!("${name}"),
    }
}

/// The option of `provider`.
fn option_of(registry: &Registry, provider: &ProviderRef) -> ProviderOption {
    ProviderOption {
        provider: provider.clone(),
        name: provider_name(registry, provider).to_string(),
    }
}

/// An example of the place of a key of `provider`, such as
/// `~/.config/example-api-key` or `EXAMPLE_API_KEY`; for a key a part
/// asks for under `slot`, the slot names it, such as `EXAMPLE_ORG_KEY`.
fn placeholder(
    registry: &Registry,
    provider: &ProviderRef,
    slot: Option<&str>,
    choice: KeyPlaceChoice,
) -> String {
    let name = provider_name(registry, provider).to_ascii_lowercase();
    let kind = slot
        .unwrap_or("api")
        .replace(|letter: char| !letter.is_ascii_alphanumeric(), "-");

    match choice {
        KeyPlaceChoice::File => format!("~/.config/{name}-{kind}.key"),
        KeyPlaceChoice::Variable => format!("{name}_{kind}_KEY")
            .replace(['-', ' '], "_")
            .to_ascii_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{Account, SavedProfiles};

    use super::*;
    use crate::add_account_dialog::AddAccountIntent;
    use crate::event::{AccountAction, AgentsIntent};
    use crate::reducer::reduce;
    use crate::testing::{
        chatgpt_login, codex, connected, open_account_menu, with_accounts,
    };

    fn state() -> AgentsState {
        with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        )
    }

    #[test]
    fn the_add_dialog_lists_the_providers_the_parts_offer() {
        let mut state = state();
        let _ = reduce(&mut state, AgentsIntent::OpenAddAccount);

        let vm = AddAccountDialogViewModel::from_state(&state)
            .expect("the dialog is open");

        let listed: Vec<_> = vm
            .providers
            .iter()
            .map(|option| option.name.as_str())
            .collect();
        assert_eq!(listed, ["Anthropic", "OpenAI", "DeepSeek", "OpenRouter"]);
        assert_eq!(vm.plugin, None);
        assert_eq!(vm.title, i18n::t(Key::AgentsAddAccountTitle));
        assert_eq!(vm.key_placeholder, "~/.config/anthropic-api.key");
        assert!(!vm.can_submit);
    }

    #[test]
    fn a_key_path_that_is_not_full_says_so() {
        let mut state = state();
        let _ = reduce(&mut state, AgentsIntent::OpenAddAccount);
        let _ = reduce(
            &mut state,
            AgentsIntent::AddAccount(AddAccountIntent::KeyChanged(
                String::from("deepseek-key"),
            )),
        );

        let vm = AddAccountDialogViewModel::from_state(&state)
            .expect("the dialog is open");

        assert_eq!(vm.key_error, Some(i18n::t(Key::AgentsErrorKeyPathNotFull)));
        assert!(!vm.can_submit);
    }

    #[test]
    fn the_edit_key_dialog_keeps_its_provider_and_shows_its_place() {
        let key = KeyRef::EnvVar(String::from("OPENAI_ORG_KEY"));
        let added = Account::added(ProviderRef::of_id("openai"), key.clone());
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default()
                .with_account_added(ProviderRef::of_id("openai"), key),
        );
        open_account_menu(&mut state, added);
        let _ = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::EditKey),
        );

        let vm = AddAccountDialogViewModel::from_state(&state)
            .expect("the dialog is open");

        assert!(vm.providers.is_empty());
        assert_eq!(vm.provider.name, "OpenAI");
        assert_eq!(vm.key_choice, KeyPlaceChoice::Variable);
        assert_eq!(vm.key, "OPENAI_ORG_KEY");
        assert_eq!(vm.key_placeholder, "OPENAI_API_KEY");
        assert_eq!(vm.submit_label, i18n::t(Key::ButtonSave));
        assert!(vm.can_submit);
    }

    /// Issue 20: the dialog of a key a part asks for is named by the
    /// part's label, with its hint, the plugin that asks, what the key
    /// starts with and its page, at the account's provider.
    #[test]
    fn the_asked_key_dialog_shows_what_the_part_asks() {
        let organization = crate::testing::organization();
        let mut state = with_accounts(
            &[(
                crate::testing::claude(),
                crate::testing::api_key(),
                vec![organization.clone()],
            )],
            SavedProfiles::default(),
        );
        let need =
            otty_agents::fixtures::key_need("anthropic", "anthropic", "admin");
        let otty_agents::PartNeedAction::AskKey(ask) = need.action().clone()
        else {
            panic!("a key to ask for");
        };
        let _ = reduce(
            &mut state,
            AgentsIntent::FixNeed(crate::need::NeedAction::AskKey {
                account: organization,
                plugin: String::from("Anthropic"),
                ask,
            }),
        );

        let vm = AddAccountDialogViewModel::from_state(&state)
            .expect("the dialog is open");

        assert_eq!(vm.title, "Admin key");
        assert_eq!(vm.text, "Reads the real bill.");
        assert_eq!(
            vm.plugin,
            Some(i18n::filled(
                Key::TplAgentsNeedPluginLabel,
                &[("plugin", "Anthropic")]
            ))
        );
        assert_eq!(
            vm.prefix,
            Some(i18n::filled(
                Key::TplAgentsKeyStartsWith,
                &[("prefix", "sk-made-up-")]
            ))
        );
        assert_eq!(
            vm.page,
            Some(i18n::filled(
                Key::TplAgentsNeedOpenPage,
                &[("host", "example.com")]
            ))
        );
        assert!(vm.providers.is_empty());
        assert_eq!(vm.provider.name, "Anthropic");
        assert_eq!(vm.key_placeholder, "~/.config/anthropic-admin.key");
    }

    #[test]
    fn a_place_is_shown_by_its_path_or_its_variable() {
        let file = KeyRef::File(PathBuf::from("/home/user/.config/key"));
        let variable = KeyRef::EnvVar(String::from("DEEPSEEK_API_KEY"));

        assert_eq!(key_place_text(&file), "/home/user/.config/key");
        assert_eq!(key_place_text(&variable), "$DEEPSEEK_API_KEY");
    }
}
