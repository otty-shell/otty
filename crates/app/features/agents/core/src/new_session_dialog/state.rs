//! The open New session dialog: its folder field with suggestions, and
//! whether the typed folder can be used.

use std::path::{Path, PathBuf};

use crate::event::AgentsOutcome;
use crate::folder_field::{
    FolderFieldIntent, FolderSuggestions, PathProblem, full_folder,
};
use crate::types::FolderAsker;

/// The open New session dialog. The profile is not kept here: the one the
/// user picked last is kept by Agents mode while OTTY runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSessionDialog {
    folder: FolderSuggestions,
    said_before: FolderState,
}

impl NewSessionDialog {
    /// The folder as typed.
    pub(crate) fn text(&self) -> &str {
        self.folder.text()
    }

    /// The folder field with its suggestions.
    pub(crate) fn folder(&self) -> &FolderSuggestions {
        &self.folder
    }

    /// The dialog with `text` in the folder field and no suggestion shown.
    pub(crate) fn new(text: String) -> Self {
        Self {
            folder: FolderSuggestions::new(FolderAsker::NewSessionDialog, text),
            said_before: FolderState::Checking,
        }
    }

    /// Do what the user did in the folder field, with `~` standing for
    /// `home`, and ask about the text in it when it changed. What the
    /// field says now is kept, for while the answer about a new text is on
    /// its way.
    pub(crate) fn change_folder(
        &mut self,
        intent: FolderFieldIntent,
        home: &Path,
    ) -> Vec<AgentsOutcome> {
        self.remember_what_was_said(home);
        self.folder.reduce(intent, home)
    }

    /// The folder field with its suggestions, to change it.
    pub(crate) fn folder_mut(&mut self) -> &mut FolderSuggestions {
        &mut self.folder
    }

    /// Whether the typed folder can be used, as far as the answer about it
    /// tells.
    pub(crate) fn folder_state(&self, home: &Path) -> FolderState {
        let folder = match full_folder(self.folder.text(), home) {
            Ok(folder) => folder,
            Err(PathProblem::Empty) => return FolderState::Empty,
            Err(PathProblem::NotFull) => return FolderState::NotFull,
        };

        match self.folder.answer(home) {
            None => FolderState::Checking,
            Some(answer) if answer.is_folder => FolderState::Found(folder),
            Some(_) => FolderState::Missing,
        }
    }

    /// What the field says about the typed folder: whether it can be used
    /// or, while the answer about it is on its way, what the field said
    /// before. So the line under the field does not blink after each
    /// letter (issue 39); Start still waits for the answer.
    pub(crate) fn shown_folder_state(&self, home: &Path) -> FolderState {
        match self.folder_state(home) {
            FolderState::Checking => self.said_before.clone(),
            state => state,
        }
    }

    /// Keep what the field says now, before the text changes.
    fn remember_what_was_said(&mut self, home: &Path) {
        self.said_before = self.shown_folder_state(home);
    }
}

/// Whether the typed folder of the New session dialog can be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FolderState {
    /// No path was typed.
    Empty,
    /// The path is not a full path, such as `work/otty`.
    NotFull,
    /// The answer about the folder has not come yet.
    Checking,
    /// There is no such folder on this computer.
    Missing,
    /// The folder is there; its path without a slash at the end.
    Found(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{folder_query, loaded_folder};
    use crate::types::LoadedFolder;

    fn home() -> &'static Path {
        Path::new("/home/user")
    }

    fn answer(folder: &str, prefix: &str, is_folder: bool) -> LoadedFolder {
        let query = folder_query(
            FolderAsker::NewSessionDialog,
            folder,
            "/work",
            prefix,
        );

        loaded_folder(query, is_folder, &[])
    }

    /// `text` typed in the folder field of `dialog`.
    fn type_folder(dialog: &mut NewSessionDialog, text: &str) {
        let _ = dialog.change_folder(
            FolderFieldIntent::Changed(String::from(text)),
            home(),
        );
    }

    #[test]
    fn an_empty_or_relative_path_says_why_it_can_not_be_used() {
        let empty = NewSessionDialog::new(String::from("  "));
        let relative = NewSessionDialog::new(String::from("work/otty"));

        assert_eq!(empty.folder_state(home()), FolderState::Empty);
        assert_eq!(relative.folder_state(home()), FolderState::NotFull);
    }

    /// Until the answer comes, the folder is being checked: Start stays
    /// off without an error.
    #[test]
    fn the_folder_is_found_or_missing_only_once_its_answer_came() {
        let mut dialog = NewSessionDialog::new(String::from("/work/otty"));
        let checking = dialog.folder_state(home());

        let applied = dialog
            .folder_mut()
            .apply(answer("/work/otty", "otty", true), home());
        let there = dialog.folder_state(home());
        type_folder(&mut dialog, "/work/gone");
        let _ = dialog
            .folder_mut()
            .apply(answer("/work/gone", "gone", false), home());

        assert_eq!(checking, FolderState::Checking);
        assert!(applied);
        assert_eq!(there, FolderState::Found(PathBuf::from("/work/otty")));
        assert_eq!(dialog.folder_state(home()), FolderState::Missing);
    }

    /// The answer of a text typed before does not count for the text now.
    #[test]
    fn an_answer_for_another_text_leaves_the_folder_checked() {
        let mut dialog = NewSessionDialog::new(String::from("/work/ott"));
        type_folder(&mut dialog, "/work/otty");

        let applied = dialog
            .folder_mut()
            .apply(answer("/work/ott", "ott", true), home());

        assert!(!applied);
        assert_eq!(dialog.folder_state(home()), FolderState::Checking);
    }

    /// Issue 39: while the answer about the text typed now is on its way,
    /// the field says what it said before, so the line under it does not
    /// blink; Start stays off until the answer.
    #[test]
    fn while_the_answer_is_on_its_way_the_field_says_what_it_said() {
        let mut dialog = NewSessionDialog::new(String::from("/work/gon"));
        let _ = dialog
            .folder_mut()
            .apply(answer("/work/gon", "gon", false), home());
        let mut empty = NewSessionDialog::new(String::new());

        type_folder(&mut dialog, "/work/gone");
        type_folder(&mut empty, "/");

        assert_eq!(dialog.folder_state(home()), FolderState::Checking);
        assert_eq!(dialog.shown_folder_state(home()), FolderState::Missing);
        assert_eq!(empty.shown_folder_state(home()), FolderState::Empty);
        let _ = dialog
            .folder_mut()
            .apply(answer("/work/gone", "gone", true), home());
        assert_eq!(
            dialog.shown_folder_state(home()),
            FolderState::Found(PathBuf::from("/work/gone"))
        );
    }
}
