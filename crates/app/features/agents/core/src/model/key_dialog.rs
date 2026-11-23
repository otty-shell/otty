//! The dialog of a key a plugin part asks for (spec
//! 2026-11-19-agent-plugins, issue 20): what the part asks, the provider
//! of the account, where the key is kept, and why it can not be saved yet.

use otty_app_i18n::{self as i18n, Key};

use super::super::key_dialog::can_submit;
use super::super::key_place_field::{KeyPlaceChoice, KeyPlaceProblem};
use super::super::state::{AgentsState, provider_name};
use super::needs::open_page_label;

/// The open dialog of a key a part asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyDialogViewModel<'a> {
    /// The label of the key the part asks for, such as "Admin key".
    pub(crate) title: String,
    /// The hint of the key the part asks for.
    pub(crate) text: String,
    /// The plugin that asks for the key, such as "OpenAI plugin".
    pub(crate) plugin: String,
    /// What the key starts with, as a line.
    pub(crate) prefix: Option<String>,
    /// The label of the button that opens the page where the key is made.
    pub(crate) page: Option<String>,
    /// The name of the provider of the account, such as "Example Cloud".
    pub(crate) provider: String,
    /// Where the key is kept: a file or a variable.
    pub(crate) key_choice: KeyPlaceChoice,
    /// The typed path or variable name.
    pub(crate) key: &'a str,
    /// An example of what to type, named by the provider and the slot.
    pub(crate) key_placeholder: String,
    /// What is wrong with the typed place.
    pub(crate) key_error: Option<&'static str>,
    /// Why the key was not saved.
    pub(crate) error: Option<String>,
    /// Whether Save acts.
    pub(crate) can_submit: bool,
}

impl<'a> KeyDialogViewModel<'a> {
    /// The open dialog; `None` while it is closed.
    pub(crate) fn from_state(state: &'a AgentsState) -> Option<Self> {
        let dialog = state.key_dialog()?;

        let ask = dialog.ask();
        let provider =
            provider_name(state.registry(), dialog.account().provider());
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
            title: ask.label().to_string(),
            text: ask.hint().to_string(),
            plugin: i18n::filled(
                Key::TplAgentsNeedPluginLabel,
                &[("plugin", dialog.plugin())],
            ),
            prefix: ask.prefix().map(|prefix| {
                i18n::filled(Key::TplAgentsKeyStartsWith, &[("prefix", prefix)])
            }),
            page: ask.page().map(open_page_label),
            provider: provider.to_string(),
            key_choice: dialog.key().choice(),
            key: dialog.key().text(),
            key_placeholder: placeholder(
                provider,
                ask.slot(),
                dialog.key().choice(),
            ),
            key_error,
            error: dialog.error().map(String::from),
            can_submit: can_submit(state),
        })
    }
}

/// An example of the place of the key `slot` of the provider named
/// `provider`, such as `~/.config/example-admin.key` or
/// `EXAMPLE_ADMIN_KEY`.
fn placeholder(provider: &str, slot: &str, choice: KeyPlaceChoice) -> String {
    let name = provider.to_ascii_lowercase();
    let kind =
        slot.replace(|letter: char| !letter.is_ascii_alphanumeric(), "-");

    match choice {
        KeyPlaceChoice::File => format!("~/.config/{name}-{kind}.key"),
        KeyPlaceChoice::Variable => format!("{name}_{kind}_KEY")
            .replace(['-', ' '], "_")
            .to_ascii_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::key_dialog::KeyDialogIntent;
    use crate::reducer::reduce;

    /// The state with the key dialog of the admin key of the organization
    /// open.
    fn asking_for_the_admin_key() -> AgentsState {
        let organization = crate::testing::organization();
        let mut state = crate::testing::with_accounts(
            &[(
                crate::testing::claude(),
                crate::testing::api_key(),
                vec![organization.clone()],
            )],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::FixNeed(crate::need::NeedAction::AskKey {
                account: organization,
                plugin: String::from("Anthropic"),
                ask: crate::testing::key_ask(),
            }),
        );

        state
    }

    /// Issue 20: the dialog of a key a part asks for is named by the
    /// part's label, with its hint, the plugin that asks, what the key
    /// starts with and its page, at the account's provider.
    #[test]
    fn the_key_dialog_shows_what_the_part_asks() {
        let state = asking_for_the_admin_key();

        let vm =
            KeyDialogViewModel::from_state(&state).expect("the dialog is open");

        assert_eq!(vm.title, "Admin key");
        assert_eq!(vm.text, "Reads the real bill.");
        assert_eq!(
            vm.plugin,
            i18n::filled(
                Key::TplAgentsNeedPluginLabel,
                &[("plugin", "Anthropic")]
            )
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
        assert_eq!(vm.provider, "Anthropic");
        assert_eq!(vm.key_placeholder, "~/.config/anthropic-admin.key");
        assert!(!vm.can_submit);
    }

    #[test]
    fn a_key_path_that_is_not_full_says_so() {
        let mut state = asking_for_the_admin_key();
        let _ = reduce(
            &mut state,
            AgentsIntent::KeyDialog(KeyDialogIntent::KeyChanged(String::from(
                "org-key",
            ))),
        );

        let vm =
            KeyDialogViewModel::from_state(&state).expect("the dialog is open");

        assert_eq!(vm.key_error, Some(i18n::t(Key::AgentsErrorKeyPathNotFull)));
        assert!(!vm.can_submit);
    }

    #[test]
    fn a_variable_is_named_by_the_provider_and_the_slot() {
        let mut state = asking_for_the_admin_key();
        let _ = reduce(
            &mut state,
            AgentsIntent::KeyDialog(KeyDialogIntent::KeyChoicePicked(
                KeyPlaceChoice::Variable,
            )),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::KeyDialog(KeyDialogIntent::KeyChanged(String::from(
                "1KEY",
            ))),
        );

        let vm =
            KeyDialogViewModel::from_state(&state).expect("the dialog is open");

        assert_eq!(vm.key_placeholder, "ANTHROPIC_ADMIN_KEY");
        assert_eq!(vm.key_error, Some(i18n::t(Key::AgentsErrorVariableName)));
    }
}
