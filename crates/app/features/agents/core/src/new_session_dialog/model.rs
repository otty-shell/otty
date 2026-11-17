//! View model of the open New session dialog: the folder with its
//! suggestions and what is wrong with it, the profiles that can start, and
//! whether Start can be pressed.

use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};

use super::reducer::new_session_profile;
use super::state::FolderState;
use crate::state::AgentsState;

/// The open New session dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSessionDialogViewModel<'a> {
    /// The folder as typed.
    pub(crate) folder: &'a str,
    /// Why the folder can not be used; `None` while it is checked or found.
    pub(crate) folder_error: Option<&'static str>,
    /// The folders the typed text may stand for, shown under the field.
    pub(crate) suggestions: Vec<String>,
    /// The profiles that can start a session, in their order.
    pub(crate) profiles: Vec<Profile>,
    /// The profile the session starts with.
    pub(crate) profile: Option<Profile>,
    /// The line under the profiles when none can start.
    pub(crate) profile_note: Option<&'static str>,
    /// Whether Start can be pressed: the folder is there and a profile can
    /// start.
    pub(crate) can_start: bool,
}

impl<'a> NewSessionDialogViewModel<'a> {
    /// Build the open dialog; `None` while none is open.
    pub(crate) fn from_state(state: &'a AgentsState) -> Option<Self> {
        let dialog = state.new_session()?;
        let home = state.env().home();

        let folder_state = dialog.folder_state(home);
        let folder_error = match folder_state {
            FolderState::Empty => Some(Key::AgentsErrorFolderEmpty),
            FolderState::NotFull => Some(Key::AgentsErrorFolderNotFullPath),
            FolderState::Missing => Some(Key::AgentsErrorFolderNotFound),
            FolderState::Checking | FolderState::Found(_) => None,
        };
        let profiles: Vec<Profile> =
            state.startable_profiles().into_iter().cloned().collect();
        let profile = new_session_profile(state).cloned();
        let profile_note = (state.is_loaded() && profiles.is_empty())
            .then(|| i18n::t(Key::AgentsNoProfileToStart));
        let can_start =
            matches!(folder_state, FolderState::Found(_)) && profile.is_some();

        Some(Self {
            folder: dialog.text(),
            folder_error: folder_error.map(i18n::t),
            suggestions: dialog.folder().suggestions(home),
            profiles,
            profile,
            profile_note,
            can_start,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::ProfileStatus;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::new_session_dialog::NewSessionDialogIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        claude, claude_work, codex, connected, with_profiles,
    };
    use crate::types::{FolderAsker, FolderQuery, LoadedFolder};

    /// Agents mode with the dialog open on `text`, and the answer that
    /// `/work/otty` is a folder when `text` is that folder.
    fn dialog_on(state: &mut AgentsState, text: &str) {
        let _ = reduce(
            state,
            AgentsIntent::OpenNewSessionDialog(Some(PathBuf::from("/work"))),
        );
        let _ = reduce(
            state,
            AgentsIntent::NewSessionDialog(
                NewSessionDialogIntent::FolderChanged(String::from(text)),
            ),
        );
        let _ = reduce(
            state,
            AgentsIntent::FolderLookedUp(LoadedFolder {
                query: FolderQuery {
                    asker: FolderAsker::NewSessionDialog,
                    folder: PathBuf::from("/work/otty"),
                    list_in: PathBuf::from("/work"),
                    prefix: String::from("otty"),
                },
                is_folder: true,
                suggestions: vec![PathBuf::from("/work/otty")],
            }),
        );
    }

    fn vm(state: &AgentsState) -> NewSessionDialogViewModel<'_> {
        NewSessionDialogViewModel::from_state(state)
            .expect("the dialog is open")
    }

    #[test]
    fn no_dialog_is_drawn_while_none_is_open() {
        let state = with_profiles(&[(claude(), connected())]);

        assert!(NewSessionDialogViewModel::from_state(&state).is_none());
    }

    #[test]
    fn a_found_folder_and_a_profile_that_can_start_let_start_be_pressed() {
        let mut state = with_profiles(&[
            (claude(), ProfileStatus::ProgramNotFound),
            (claude_work(), connected()),
            (codex(), connected()),
        ]);
        dialog_on(&mut state, "/work/otty");

        let vm = vm(&state);

        assert!(vm.can_start);
        assert_eq!(vm.folder, "/work/otty");
        assert_eq!(vm.folder_error, None);
        assert_eq!(vm.profiles, vec![claude_work(), codex()]);
        assert_eq!(vm.profile, Some(claude_work()));
        assert_eq!(vm.profile_note, None);
        assert_eq!(vm.suggestions, vec![String::from("/work/otty/")]);
    }

    /// The field says why Start is off.
    #[test]
    fn the_field_says_why_the_folder_can_not_be_used() {
        let cases = [
            ("", Some(Key::AgentsErrorFolderEmpty)),
            ("work/otty", Some(Key::AgentsErrorFolderNotFullPath)),
            ("/work/gone", None),
        ];

        for (text, error) in cases {
            let mut state = with_profiles(&[(claude(), connected())]);
            dialog_on(&mut state, text);

            let vm = vm(&state);

            assert_eq!(vm.folder_error, error.map(i18n::t), "{text:?}");
            assert!(!vm.can_start, "{text:?}");
        }
    }

    #[test]
    fn a_folder_that_is_not_there_says_so() {
        let mut state = with_profiles(&[(claude(), connected())]);
        dialog_on(&mut state, "/work/gone");
        let _ = reduce(
            &mut state,
            AgentsIntent::FolderLookedUp(LoadedFolder {
                query: FolderQuery {
                    asker: FolderAsker::NewSessionDialog,
                    folder: PathBuf::from("/work/gone"),
                    list_in: PathBuf::from("/work"),
                    prefix: String::from("gone"),
                },
                is_folder: false,
                suggestions: Vec::new(),
            }),
        );

        let vm = vm(&state);

        assert_eq!(
            vm.folder_error,
            Some(i18n::t(Key::AgentsErrorFolderNotFound))
        );
        assert!(!vm.can_start);
    }

    #[test]
    fn with_no_profile_that_can_start_the_select_says_why() {
        let mut state =
            with_profiles(&[(claude(), ProfileStatus::ProgramNotFound)]);
        dialog_on(&mut state, "/work/otty");

        let vm = vm(&state);

        assert!(vm.profiles.is_empty());
        assert_eq!(vm.profile, None);
        assert_eq!(vm.profile_note, Some(i18n::t(Key::AgentsNoProfileToStart)));
        assert!(!vm.can_start);
    }
}
