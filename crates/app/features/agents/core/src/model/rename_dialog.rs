//! The Rename dialog of an account or a profile (spec 2026-11-19,
//! Decision 7): its title and text say what it names.

use otty_app_i18n::{self as i18n, Key};

use super::super::rename_dialog::RenameTarget;
use super::super::state::AgentsState;

/// The open Rename dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenameDialogViewModel<'a> {
    /// "Rename account" or "Rename profile".
    pub(crate) title: &'static str,
    /// What the name is for and who sees it.
    pub(crate) text: &'static str,
    /// The name typed in the field.
    pub(crate) name: &'a str,
    /// Why the name was not saved.
    pub(crate) error: Option<String>,
    /// Whether Save acts.
    pub(crate) can_save: bool,
}

impl<'a> RenameDialogViewModel<'a> {
    /// The open dialog; `None` while it is closed.
    pub(crate) fn from_state(state: &'a AgentsState) -> Option<Self> {
        let dialog = state.rename()?;

        let (title, text) = match dialog.target() {
            RenameTarget::Account(_) => {
                (Key::AgentsRenameAccountTitle, Key::AgentsRenameAccountText)
            },
            RenameTarget::Profile(_) => {
                (Key::AgentsRenameProfileTitle, Key::AgentsRenameProfileText)
            },
        };

        Some(Self {
            title: i18n::t(title),
            text: i18n::t(text),
            name: dialog.name(),
            error: dialog.error().map(String::from),
            can_save: dialog.can_save(),
        })
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;

    use super::*;
    use crate::event::{AccountAction, AgentsIntent, ProfileAction};
    use crate::reducer::reduce;
    use crate::rename_dialog::RenameIntent;
    use crate::testing::{
        chatgpt_login, codex, connected, open_account_menu, open_profile_menu,
        with_accounts,
    };

    fn with_chatgpt() -> AgentsState {
        with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        )
    }

    #[test]
    fn the_dialog_of_an_account_says_it_names_an_account() {
        let mut state = with_chatgpt();
        open_account_menu(&mut state, chatgpt_login());
        let _ = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::Rename),
        );

        let vm = RenameDialogViewModel::from_state(&state);

        assert_eq!(
            vm,
            Some(RenameDialogViewModel {
                title: i18n::t(Key::AgentsRenameAccountTitle),
                text: i18n::t(Key::AgentsRenameAccountText),
                name: "OpenAI (you@example.com)",
                error: None,
                can_save: true,
            })
        );
    }

    /// A profile needs a name: Save is off while the field is empty.
    #[test]
    fn the_dialog_of_a_profile_saves_no_empty_name() {
        let mut state = with_chatgpt();
        open_profile_menu(&mut state, codex());
        let _ = reduce(
            &mut state,
            AgentsIntent::ProfileMenu(ProfileAction::Rename),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::Rename(RenameIntent::NameChanged(String::new())),
        );

        let vm = RenameDialogViewModel::from_state(&state);

        assert_eq!(
            vm,
            Some(RenameDialogViewModel {
                title: i18n::t(Key::AgentsRenameProfileTitle),
                text: i18n::t(Key::AgentsRenameProfileText),
                name: "",
                error: None,
                can_save: false,
            })
        );
    }
}
