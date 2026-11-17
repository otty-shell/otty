//! The open New session dialog: the typed folder, the latest answer about
//! it, and whether the folders it may stand for show under the field.

use std::path::{Path, PathBuf};

use crate::profile_dialog::{FolderProblem, full_path};
use crate::types::{FolderQuery, LoadedFolder};

/// The open New session dialog. The profile is not kept here: the one the
/// user picked last is kept by Agents mode while OTTY runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSessionDialog {
    text: String,
    answer: Option<LoadedFolder>,
    are_suggestions_open: bool,
}

impl NewSessionDialog {
    /// The folder as typed.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The dialog with `text` in the folder field and no suggestion shown.
    pub(crate) fn new(text: String) -> Self {
        Self {
            text,
            answer: None,
            are_suggestions_open: false,
        }
    }

    /// What to ask about the typed folder, with `~` standing for `home`;
    /// `None` while it is not a full path.
    pub(crate) fn query(&self, home: &Path) -> Option<FolderQuery> {
        folder_query(&self.text, home)
    }

    /// Whether the typed folder can be used, as far as the answer about it
    /// tells.
    pub(crate) fn folder_state(&self, home: &Path) -> FolderState {
        let folder = match full_path(&self.text, home) {
            Ok(folder) => folder,
            Err(FolderProblem::Empty) => return FolderState::Empty,
            Err(_) => return FolderState::NotFull,
        };

        match self.current_answer(home) {
            None => FolderState::Checking,
            Some(answer) if answer.is_folder => {
                FolderState::Found(folder.components().collect())
            },
            Some(_) => FolderState::Missing,
        }
    }

    /// The folders the typed text may stand for, as a pick puts them in
    /// the field; none while the list is closed.
    pub(crate) fn suggestions(&self, home: &Path) -> Vec<String> {
        if !self.are_suggestions_open {
            return Vec::new();
        }
        let Some(answer) = self.current_answer(home) else {
            return Vec::new();
        };

        answer
            .suggestions
            .iter()
            .map(|folder| format!("{}/", shown_path(folder, home)))
            .collect()
    }

    /// Take the typed text and show its suggestions once they come.
    pub(crate) fn set_text(&mut self, text: String) {
        self.text = text;
        self.are_suggestions_open = true;
    }

    /// Take the answer `loaded` when it is about the text typed now;
    /// `false` when it is about an older text and is dropped.
    pub(crate) fn apply(&mut self, loaded: LoadedFolder, home: &Path) -> bool {
        if self.query(home).as_ref() != Some(&loaded.query) {
            return false;
        }

        self.answer = Some(loaded);
        true
    }

    /// Put the suggestion at `index` in the field and close the list;
    /// `false` when there is no such suggestion.
    pub(crate) fn pick(&mut self, index: usize, home: &Path) -> bool {
        let Some(picked) = self.suggestions(home).into_iter().nth(index) else {
            return false;
        };

        self.text = picked;
        self.are_suggestions_open = false;
        true
    }

    /// Close the list of suggestions until the next typing.
    pub(crate) fn close_suggestions(&mut self) {
        self.are_suggestions_open = false;
    }

    /// The answer about the text typed now, if it came.
    fn current_answer(&self, home: &Path) -> Option<&LoadedFolder> {
        let query = self.query(home)?;

        self.answer.as_ref().filter(|answer| answer.query == query)
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

/// `path` as the folder field shows it: `~` for the home folder `home` and
/// what is in it, else the whole path.
pub(crate) fn shown_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => String::from("~"),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// What to ask about `text`: the folders in it when it ends with a slash
/// (or is `~`), else the folders next to it whose name starts with its last
/// part.
fn folder_query(text: &str, home: &Path) -> Option<FolderQuery> {
    let folder: PathBuf = full_path(text, home).ok()?.components().collect();
    let typed = text.trim();
    let lists_inside = typed.ends_with('/') || typed == "~";

    let name = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let (list_in, prefix) = match (lists_inside, folder.parent(), name) {
        (false, Some(parent), Some(name)) => (parent.to_path_buf(), name),
        _ => (folder.clone(), String::new()),
    };

    Some(FolderQuery {
        folder,
        list_in,
        prefix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/user";

    fn home() -> &'static Path {
        Path::new(HOME)
    }

    fn query(folder: &str, list_in: &str, prefix: &str) -> FolderQuery {
        FolderQuery {
            folder: PathBuf::from(folder),
            list_in: PathBuf::from(list_in),
            prefix: String::from(prefix),
        }
    }

    fn found(query: FolderQuery, suggestions: &[&str]) -> LoadedFolder {
        LoadedFolder {
            query,
            is_folder: true,
            suggestions: suggestions.iter().map(PathBuf::from).collect(),
        }
    }

    /// A typed name is looked for among the folders of its parent.
    #[test]
    fn a_typed_name_asks_for_the_folders_of_its_parent_that_start_with_it() {
        let dialog = NewSessionDialog::new(String::from("~/pro"));

        assert_eq!(
            dialog.query(home()),
            Some(query("/home/user/pro", HOME, "pro"))
        );
    }

    /// A path that ends with a slash lists the folders in it.
    #[test]
    fn a_path_with_a_slash_at_the_end_asks_for_the_folders_in_it() {
        let dialog = NewSessionDialog::new(String::from("/work/otty/"));
        let root = NewSessionDialog::new(String::from("/"));
        let tilde = NewSessionDialog::new(String::from("~"));

        assert_eq!(
            dialog.query(home()),
            Some(query("/work/otty", "/work/otty", ""))
        );
        assert_eq!(root.query(home()), Some(query("/", "/", "")));
        assert_eq!(tilde.query(home()), Some(query(HOME, HOME, "")));
    }

    #[test]
    fn an_empty_or_relative_path_asks_for_nothing_and_says_why() {
        let empty = NewSessionDialog::new(String::from("  "));
        let relative = NewSessionDialog::new(String::from("work/otty"));

        assert_eq!(empty.query(home()), None);
        assert_eq!(relative.query(home()), None);
        assert_eq!(empty.folder_state(home()), FolderState::Empty);
        assert_eq!(relative.folder_state(home()), FolderState::NotFull);
    }

    /// Until the answer comes, the folder is being checked: Start stays
    /// off without an error.
    #[test]
    fn the_folder_is_found_or_missing_only_once_its_answer_came() {
        let mut dialog = NewSessionDialog::new(String::from("/work/otty"));
        let asked = query("/work/otty", "/work", "otty");
        let checking = dialog.folder_state(home());

        let applied = dialog.apply(found(asked.clone(), &[]), home());
        let there = dialog.folder_state(home());
        dialog.set_text(String::from("/work/gone"));
        let gone = query("/work/gone", "/work", "gone");
        let _ = dialog.apply(
            LoadedFolder {
                query: gone,
                is_folder: false,
                suggestions: Vec::new(),
            },
            home(),
        );

        assert_eq!(checking, FolderState::Checking);
        assert!(applied);
        assert_eq!(there, FolderState::Found(PathBuf::from("/work/otty")));
        assert_eq!(dialog.folder_state(home()), FolderState::Missing);
    }

    /// The answer of a text typed before does not count for the text now.
    #[test]
    fn an_answer_for_another_text_is_dropped() {
        let mut dialog = NewSessionDialog::new(String::from("/work/ott"));
        dialog.set_text(String::from("/work/otty"));

        let applied = dialog
            .apply(found(query("/work/ott", "/work", "ott"), &[]), home());

        assert!(!applied);
        assert_eq!(dialog.folder_state(home()), FolderState::Checking);
    }

    /// The suggestions show while the user types, written with `~` for the
    /// home folder and a slash at the end, as a pick puts them in the
    /// field.
    #[test]
    fn typing_shows_the_suggestions_of_the_answer() {
        let mut dialog = NewSessionDialog::new(String::from("~"));
        let opened_with = dialog.suggestions(home());
        dialog.set_text(String::from("~/pro"));
        let _ = dialog.apply(
            found(
                query("/home/user/pro", HOME, "pro"),
                &["/home/user/projects", "/home/user/promo"],
            ),
            home(),
        );

        assert!(opened_with.is_empty(), "the list opens on typing");
        assert_eq!(
            dialog.suggestions(home()),
            vec![String::from("~/projects/"), String::from("~/promo/")]
        );
    }

    #[test]
    fn a_picked_suggestion_fills_the_field_and_closes_the_list() {
        let mut dialog = NewSessionDialog::new(String::from("~"));
        dialog.set_text(String::from("/work/o"));
        let _ = dialog.apply(
            found(
                query("/work/o", "/work", "o"),
                &["/work/otty", "/work/other"],
            ),
            home(),
        );

        let picked = dialog.pick(1, home());

        assert!(picked);
        assert_eq!(dialog.text(), "/work/other/");
        assert!(dialog.suggestions(home()).is_empty());
        assert!(!dialog.pick(5, home()), "no such suggestion");
    }

    #[test]
    fn a_closed_list_shows_no_suggestion_until_the_next_typing() {
        let mut dialog = NewSessionDialog::new(String::from("~"));
        dialog.set_text(String::from("/work/o"));
        let _ = dialog.apply(
            found(query("/work/o", "/work", "o"), &["/work/otty"]),
            home(),
        );

        dialog.close_suggestions();

        assert!(dialog.suggestions(home()).is_empty());
    }

    #[test]
    fn a_folder_in_the_home_folder_is_written_with_a_tilde() {
        assert_eq!(shown_path(Path::new("/home/user/work"), home()), "~/work");
        assert_eq!(shown_path(home(), home()), "~");
        assert_eq!(shown_path(Path::new("/srv/app"), home()), "/srv/app");
    }
}
