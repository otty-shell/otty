//! The Add account dialog and the Edit key dialog of an account the user
//! added (spec 2026-11-19, Decision 7): the providers with what OTTY reads
//! of each, where the key is kept, and why it can not be saved yet.

use otty_agents::{AddAccountKind, AdminKeyRef, ProviderRef, Registry};
use otty_app_i18n::{self as i18n, Key};

use super::super::add_account_dialog::can_submit;
use super::super::key_place_field::KeyPlaceProblem;
use super::super::profile_dialog::AdminKeyChoice;
use super::super::state::{AgentsState, provider_name};

/// A provider an account can be added for, with what OTTY reads of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderOption {
    /// The provider.
    pub(crate) provider: ProviderRef,
    /// Its name, such as "DeepSeek".
    pub(crate) name: String,
    /// What OTTY reads of it, such as "Money left".
    pub(crate) note: &'static str,
}

/// The open Add account or Edit key dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddAccountDialogViewModel<'a> {
    /// "Add account" or "Edit key".
    pub(crate) title: &'static str,
    /// What the dialog is for and what OTTY saves.
    pub(crate) text: &'static str,
    /// The providers to pick from; empty while a key is edited, whose
    /// provider stays.
    pub(crate) providers: Vec<ProviderOption>,
    /// The provider picked.
    pub(crate) provider: ProviderOption,
    /// Whether the list of the providers is open.
    pub(crate) is_provider_list_open: bool,
    /// Where the key is kept: a file or a variable.
    pub(crate) key_choice: AdminKeyChoice,
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

        let is_editing = dialog.editing().is_some();
        let (title, submit_label) = if is_editing {
            (Key::AgentsEditKeyTitle, Key::ButtonSave)
        } else {
            (Key::AgentsAddAccountTitle, Key::ButtonAdd)
        };
        let providers = if is_editing {
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

        Some(Self {
            title: i18n::t(title),
            text: i18n::t(Key::AgentsAddAccountText),
            providers,
            provider: option_of(state.registry(), dialog.provider()),
            is_provider_list_open: dialog.is_provider_list_open(),
            key_choice: dialog.key().choice(),
            key: dialog.key().text(),
            key_placeholder: placeholder(
                state.registry(),
                dialog.provider(),
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
pub(crate) fn key_place_text(key: &AdminKeyRef) -> String {
    match key {
        AdminKeyRef::File(path) => path.display().to_string(),
        AdminKeyRef::EnvVar(name) => format!("${name}"),
    }
}

/// The option of `provider`, with what OTTY reads of an account added
/// there, as its registry entry tells: the money left of a key, or the
/// bill of an organization read with an admin key.
fn option_of(registry: &Registry, provider: &ProviderRef) -> ProviderOption {
    let note = match add_account_kind(registry, provider) {
        Some(AddAccountKind::AdminKey) => Key::AgentsAddedBill,
        _ => Key::AgentsAddedLeft,
    };

    ProviderOption {
        provider: provider.clone(),
        name: provider_name(registry, provider).to_string(),
        note: i18n::t(note),
    }
}

/// What Add account offers for `provider` in `registry`; `None` for a
/// provider no entry serves.
fn add_account_kind(
    registry: &Registry,
    provider: &ProviderRef,
) -> Option<AddAccountKind> {
    registry.provider_of(provider)?.add_account()
}

/// An example of the place of a key of `provider`, such as
/// `~/.config/deepseek-key` or `DEEPSEEK_API_KEY`, or the admin key of an
/// organization, such as `OPENAI_ADMIN_KEY`.
fn placeholder(
    registry: &Registry,
    provider: &ProviderRef,
    choice: AdminKeyChoice,
) -> String {
    let name = provider_name(registry, provider).to_ascii_lowercase();
    let kind = match add_account_kind(registry, provider) {
        Some(AddAccountKind::AdminKey) => "admin",
        _ => "api",
    };

    match choice {
        AdminKeyChoice::File => format!("~/.config/{name}-{kind}-key"),
        AdminKeyChoice::Variable => {
            format!("{name}_{kind}_KEY").to_ascii_uppercase()
        },
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
    fn the_add_dialog_lists_the_providers_with_what_otty_reads() {
        let mut state = state();
        let _ = reduce(&mut state, AgentsIntent::OpenAddAccount);

        let vm = AddAccountDialogViewModel::from_state(&state)
            .expect("the dialog is open");

        let listed: Vec<_> = vm
            .providers
            .iter()
            .map(|option| (option.name.as_str(), option.note))
            .collect();
        assert_eq!(
            listed,
            [
                ("DeepSeek", i18n::t(Key::AgentsAddedLeft)),
                ("OpenRouter", i18n::t(Key::AgentsAddedLeft)),
                ("Moonshot", i18n::t(Key::AgentsAddedLeft)),
                ("Anthropic", i18n::t(Key::AgentsAddedBill)),
                ("OpenAI", i18n::t(Key::AgentsAddedBill)),
            ]
        );
        assert_eq!(vm.title, i18n::t(Key::AgentsAddAccountTitle));
        assert_eq!(vm.key_placeholder, "~/.config/deepseek-api-key");
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
        let key = AdminKeyRef::EnvVar(String::from("OPENAI_ADMIN_KEY"));
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
        assert_eq!(vm.key_choice, AdminKeyChoice::Variable);
        assert_eq!(vm.key, "OPENAI_ADMIN_KEY");
        assert_eq!(vm.key_placeholder, "OPENAI_ADMIN_KEY");
        assert_eq!(vm.submit_label, i18n::t(Key::ButtonSave));
        assert!(vm.can_submit);
    }

    #[test]
    fn a_place_is_shown_by_its_path_or_its_variable() {
        let file = AdminKeyRef::File(PathBuf::from("/home/user/.config/key"));
        let variable = AdminKeyRef::EnvVar(String::from("DEEPSEEK_API_KEY"));

        assert_eq!(key_place_text(&file), "/home/user/.config/key");
        assert_eq!(key_place_text(&variable), "$DEEPSEEK_API_KEY");
    }
}
