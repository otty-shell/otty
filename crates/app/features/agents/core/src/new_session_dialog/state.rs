//! The open New session dialog: its folder field with suggestions, and
//! whether the typed folder can be used.

use std::path::{Path, PathBuf};

use crate::folder_suggestions::FolderSuggestions;
use crate::profile_dialog::{FolderProblem, full_path};
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

    /// Take the typed folder, with `~` standing for `home`.
    pub(crate) fn set_text(&mut self, text: String, home: &Path) {
        self.remember_what_was_said(home);
        self.folder.set_text(text, home);
    }

    /// Put the suggestion at `index` in the field; `false` when there is
    /// no such suggestion.
    pub(crate) fn pick(&mut self, index: usize, home: &Path) -> bool {
        self.remember_what_was_said(home);
        self.folder.pick(index, home)
    }

    /// The folder field with its suggestions, to change it.
    pub(crate) fn folder_mut(&mut self) -> &mut FolderSuggestions {
        &mut self.folder
    }

    /// Whether the typed folder can be used, as far as the answer about it
    /// tells.
    pub(crate) fn folder_state(&self, home: &Path) -> FolderState {
        let folder = match full_path(self.folder.text(), home) {
            Ok(folder) => folder,
            Err(FolderProblem::Empty) => return FolderState::Empty,
            Err(_) => return FolderState::NotFull,
        };

        match self.folder.answer(home) {
            None => FolderState::Checking,
            Some(answer) if answer.is_folder => {
                FolderState::Found(folder.components().collect())
            },
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
    use crate::types::{FolderQuery, LoadedFolder};

    fn home() -> &'static Path {
        Path::new("/home/user")
    }

    fn answer(folder: &str, prefix: &str, is_folder: bool) -> LoadedFolder {
        LoadedFolder {
            query: FolderQuery {
                asker: FolderAsker::NewSessionDialog,
                folder: PathBuf::from(folder),
                list_in: PathBuf::from("/work"),
                prefix: String::from(prefix),
            },
            is_folder,
            suggestions: Vec::new(),
        }
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
        dialog.set_text(String::from("/work/gone"), home());
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
        dialog.set_text(String::from("/work/otty"), home());

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

        dialog.set_text(String::from("/work/gone"), home());
        empty.set_text(String::from("/"), home());

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
