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
        }
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
        dialog.folder_mut().set_text(String::from("/work/gone"));
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
        dialog.folder_mut().set_text(String::from("/work/otty"));

        let applied = dialog
            .folder_mut()
            .apply(answer("/work/ott", "ott", true), home());

        assert!(!applied);
        assert_eq!(dialog.folder_state(home()), FolderState::Checking);
    }
}
