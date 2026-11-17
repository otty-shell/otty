//! A folder field with suggestions, as the New session dialog and the
//! profile dialog have it (spec Decisions 15 and 17): the typed path, the
//! latest answer of the look-up about it, and whether the folders it may
//! stand for show under the field. `~` stands for the home folder, a path
//! that ends with `/` lists the folders in it, and a pick puts the folder
//! with a `/` in the field, so the next look-up lists the next level.

use std::path::{Path, PathBuf};

use crate::event::AgentsOutcome;
use crate::profile_dialog::full_path;
use crate::types::{FolderAsker, FolderQuery, LoadedFolder};

/// The typed path of a folder field and the folders it may stand for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FolderSuggestions {
    asker: FolderAsker,
    text: String,
    answer: Option<LoadedFolder>,
    are_open: bool,
}

impl FolderSuggestions {
    /// The path as typed.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The field of the dialog `asker` with `text` in it and no suggestion
    /// shown.
    pub(crate) fn new(asker: FolderAsker, text: String) -> Self {
        Self {
            asker,
            text,
            answer: None,
            are_open: false,
        }
    }

    /// What to ask about the typed path, with `~` standing for `home`;
    /// `None` while it is not a full path.
    pub(crate) fn query(&self, home: &Path) -> Option<FolderQuery> {
        folder_query(self.asker, &self.text, home)
    }

    /// The answer about the text typed now, if it came.
    pub(crate) fn answer(&self, home: &Path) -> Option<&LoadedFolder> {
        let query = self.query(home)?;

        self.answer.as_ref().filter(|answer| answer.query == query)
    }

    /// The folders the typed text may stand for, as a pick puts them in
    /// the field; none while the list is closed.
    pub(crate) fn suggestions(&self, home: &Path) -> Vec<String> {
        if !self.are_open {
            return Vec::new();
        }
        let Some(answer) = self.answer(home) else {
            return Vec::new();
        };

        answer
            .suggestions
            .iter()
            .map(|folder| format!("{}/", shown_path(folder, home)))
            .collect()
    }

    /// Ask about the typed path, unless it is not a full path.
    pub(crate) fn look_up(&self, home: &Path) -> Vec<AgentsOutcome> {
        self.query(home)
            .map(AgentsOutcome::LookUpFolder)
            .into_iter()
            .collect()
    }

    /// Take the typed text and show its suggestions once they come.
    pub(crate) fn set_text(&mut self, text: String) {
        self.text = text;
        self.are_open = true;
    }

    /// Take the answer `loaded` when it is about the text typed now in
    /// this field; `false` when it is about an older text or was asked by
    /// the other dialog, and is dropped.
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
        self.are_open = false;
        true
    }

    /// Close the list of suggestions until the next typing.
    pub(crate) fn close(&mut self) {
        self.are_open = false;
    }
}

/// `path` as a folder field shows it: `~` for the home folder `home` and
/// what is in it, else the whole path.
pub(crate) fn shown_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => String::from("~"),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// What the field of `asker` asks about `text`: the folders in it when it
/// ends with a slash (or is `~`), else the folders next to it whose name
/// starts with its last part.
fn folder_query(
    asker: FolderAsker,
    text: &str,
    home: &Path,
) -> Option<FolderQuery> {
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
        asker,
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
        query_of(FolderAsker::NewSessionDialog, folder, list_in, prefix)
    }

    fn query_of(
        asker: FolderAsker,
        folder: &str,
        list_in: &str,
        prefix: &str,
    ) -> FolderQuery {
        FolderQuery {
            asker,
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

    fn field(text: &str) -> FolderSuggestions {
        FolderSuggestions::new(
            FolderAsker::NewSessionDialog,
            String::from(text),
        )
    }

    /// A typed name is looked for among the folders of its parent.
    #[test]
    fn a_typed_name_asks_for_the_folders_of_its_parent_that_start_with_it() {
        let folder = field("~/pro");

        assert_eq!(
            folder.query(home()),
            Some(query("/home/user/pro", HOME, "pro"))
        );
    }

    /// A path that ends with a slash lists the folders in it.
    #[test]
    fn a_path_with_a_slash_at_the_end_asks_for_the_folders_in_it() {
        let folder = field("/work/otty/");
        let root = field("/");
        let tilde = field("~");

        assert_eq!(
            folder.query(home()),
            Some(query("/work/otty", "/work/otty", ""))
        );
        assert_eq!(root.query(home()), Some(query("/", "/", "")));
        assert_eq!(tilde.query(home()), Some(query(HOME, HOME, "")));
    }

    #[test]
    fn an_empty_or_relative_path_asks_for_nothing() {
        assert_eq!(field("  ").query(home()), None);
        assert_eq!(field("work/otty").query(home()), None);
        assert!(field("work/otty").look_up(home()).is_empty());
    }

    /// Issue 38: the answer goes back to the dialog that asked, so the
    /// query names it. Config folders are hidden: `~/.c` asks for them.
    #[test]
    fn the_query_names_the_dialog_that_asked() {
        let folder =
            FolderSuggestions::new(FolderAsker::ProfileDialog, "~/.c".into());

        assert_eq!(
            folder.look_up(home()),
            vec![AgentsOutcome::LookUpFolder(query_of(
                FolderAsker::ProfileDialog,
                "/home/user/.c",
                HOME,
                ".c",
            ))]
        );
    }

    /// The answer of a text typed before does not count for the text now.
    #[test]
    fn an_answer_for_another_text_is_dropped() {
        let mut folder = field("/work/ott");
        folder.set_text(String::from("/work/otty"));

        let applied = folder.apply(
            found(query("/work/ott", "/work", "ott"), &["/work/otty"]),
            home(),
        );

        assert!(!applied);
        assert_eq!(folder.answer(home()), None);
        assert!(folder.suggestions(home()).is_empty());
    }

    /// Issue 38: an answer the other dialog asked for is dropped, even for
    /// the same text.
    #[test]
    fn an_answer_for_the_other_dialog_is_dropped() {
        let mut folder =
            FolderSuggestions::new(FolderAsker::ProfileDialog, String::new());
        folder.set_text(String::from("~/.c"));

        let applied = folder.apply(
            found(
                query("/home/user/.c", HOME, ".c"),
                &["/home/user/.claude-work"],
            ),
            home(),
        );

        assert!(!applied);
        assert!(folder.suggestions(home()).is_empty());
    }

    /// The suggestions show while the user types, written with `~` for the
    /// home folder and a slash at the end, as a pick puts them in the
    /// field.
    #[test]
    fn typing_shows_the_suggestions_of_the_answer() {
        let mut folder = field("~");
        let opened_with = folder.suggestions(home());
        folder.set_text(String::from("~/pro"));
        let applied = folder.apply(
            found(
                query("/home/user/pro", HOME, "pro"),
                &["/home/user/projects", "/home/user/promo"],
            ),
            home(),
        );

        assert!(opened_with.is_empty(), "the list opens on typing");
        assert!(applied);
        assert_eq!(
            folder.suggestions(home()),
            vec![String::from("~/projects/"), String::from("~/promo/")]
        );
        assert!(folder.answer(home()).is_some_and(|answer| answer.is_folder));
    }

    #[test]
    fn a_picked_suggestion_fills_the_field_and_closes_the_list() {
        let mut folder = field("~");
        folder.set_text(String::from("/work/o"));
        let _ = folder.apply(
            found(
                query("/work/o", "/work", "o"),
                &["/work/otty", "/work/other"],
            ),
            home(),
        );

        let picked = folder.pick(1, home());

        assert!(picked);
        assert_eq!(folder.text(), "/work/other/");
        assert!(folder.suggestions(home()).is_empty());
        assert!(!folder.pick(5, home()), "no such suggestion");
    }

    #[test]
    fn a_closed_list_shows_no_suggestion_until_the_next_typing() {
        let mut folder = field("~");
        folder.set_text(String::from("/work/o"));
        let _ = folder.apply(
            found(query("/work/o", "/work", "o"), &["/work/otty"]),
            home(),
        );

        folder.close();

        assert!(folder.suggestions(home()).is_empty());
    }

    #[test]
    fn a_folder_in_the_home_folder_is_written_with_a_tilde() {
        assert_eq!(shown_path(Path::new("/home/user/work"), home()), "~/work");
        assert_eq!(shown_path(home(), home()), "~");
        assert_eq!(shown_path(Path::new("/srv/app"), home()), "/srv/app");
    }
}
