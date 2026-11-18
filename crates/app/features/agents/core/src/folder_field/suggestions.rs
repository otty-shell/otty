//! The typed path of a folder field, the latest answer of the look-up
//! about it, and the folders it may stand for under the field.

use std::path::{Path, PathBuf};

use super::event::FolderFieldIntent;
use super::full_path::full_path;
use crate::event::AgentsOutcome;
use crate::types::{FolderAsker, FolderQuery, LoadedFolder};

/// The typed path of a folder field and the folders it may stand for.
///
/// The rows shown are kept apart from the answers (issue 39): while the
/// answer about the text typed now is on its way, the list keeps its rows,
/// narrowed to the ones the new text also stands for when it can be, so it
/// does not close and open again after each letter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FolderSuggestions {
    asker: FolderAsker,
    text: String,
    answer: Option<LoadedFolder>,
    rows: Vec<PathBuf>,
    highlighted: Option<usize>,
    are_open: bool,
}

impl FolderSuggestions {
    /// The path as typed.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The row highlighted from the keyboard, while the list shows.
    pub(crate) fn highlighted(&self) -> Option<usize> {
        self.highlighted.filter(|_| self.are_open)
    }

    /// The field of the dialog `asker` with `text` in it and no suggestion
    /// shown.
    pub(crate) fn new(asker: FolderAsker, text: String) -> Self {
        Self {
            asker,
            text,
            answer: None,
            rows: Vec::new(),
            highlighted: None,
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

        self.rows
            .iter()
            .map(|folder| {
                let shown = shown_path(folder, home);
                format!("{shown}/")
            })
            .collect()
    }

    /// Ask about the typed path, unless it is not a full path.
    pub(crate) fn look_up(&self, home: &Path) -> Vec<AgentsOutcome> {
        self.query(home)
            .map(AgentsOutcome::LookUpFolder)
            .into_iter()
            .collect()
    }

    /// Do what the user did in the field, with `~` standing for `home`,
    /// and ask about the text in it when typing or a pick changed it.
    pub(crate) fn reduce(
        &mut self,
        intent: FolderFieldIntent,
        home: &Path,
    ) -> Vec<AgentsOutcome> {
        match intent {
            FolderFieldIntent::Changed(text) => {
                self.set_text(text, home);
                self.look_up(home)
            },
            FolderFieldIntent::SuggestionPicked(index) => {
                if !self.pick(index, home) {
                    return Vec::new();
                }

                self.look_up(home)
            },
            FolderFieldIntent::SuggestionHighlighted(row) => {
                self.highlight(row);
                Vec::new()
            },
            FolderFieldIntent::SuggestionsClosed => {
                self.close();
                Vec::new()
            },
        }
    }

    /// Take the typed text, with `~` standing for `home`, and open the
    /// list. Until the answer about it comes the list keeps its rows,
    /// narrowed when it can be; a text that asks for nothing closes it.
    /// The highlight is cleared.
    fn set_text(&mut self, text: String, home: &Path) {
        self.text = text;
        self.are_open = true;
        self.highlighted = None;

        self.rows = match self.query(home) {
            Some(query) => self.rows_until_answer(&query),
            None => Vec::new(),
        };
    }

    /// Take the answer `loaded` when it is about the text typed now in
    /// this field: its folders replace the rows, and a highlighted folder
    /// stays highlighted while it is among them. `false` when it is about
    /// an older text or was asked by the other dialog, and is dropped.
    pub(crate) fn apply(&mut self, loaded: LoadedFolder, home: &Path) -> bool {
        if self.query(home).as_ref() != Some(&loaded.query) {
            return false;
        }

        let highlighted_folder =
            self.highlighted.and_then(|row| self.rows.get(row));
        self.highlighted = highlighted_folder.and_then(|folder| {
            loaded.suggestions.iter().position(|found| found == folder)
        });
        self.rows.clone_from(&loaded.suggestions);
        self.answer = Some(loaded);
        true
    }

    /// Highlight the shown row at `row`, or none; a row that is not shown
    /// is not highlighted.
    fn highlight(&mut self, row: Option<usize>) {
        match row {
            Some(row) if !self.are_open || row >= self.rows.len() => {},
            _ => self.highlighted = row,
        }
    }

    /// Put the suggestion at `index` in the field and clear the rows; the
    /// answer about the picked folder lists its folders. `false` when
    /// there is no such suggestion.
    fn pick(&mut self, index: usize, home: &Path) -> bool {
        let Some(picked) = self.suggestions(home).into_iter().nth(index) else {
            return false;
        };

        self.text = picked;
        self.rows.clear();
        self.highlighted = None;
        true
    }

    /// Close the list of suggestions until the next typing.
    pub(crate) fn close(&mut self) {
        self.are_open = false;
        self.highlighted = None;
    }

    /// The rows to show while the answer about `query` is on its way: the
    /// folders of the last answer that `query` also stands for, when it
    /// lists the same folder; else, or when none is left, the rows shown
    /// now.
    fn rows_until_answer(&self, query: &FolderQuery) -> Vec<PathBuf> {
        let narrowed: Vec<PathBuf> = self
            .answer
            .iter()
            .filter(|answer| answer.query.list_in == query.list_in)
            .flat_map(|answer| &answer.suggestions)
            .filter(|folder| {
                folder
                    .file_name()
                    .is_some_and(|name| query.suggests(&name.to_string_lossy()))
            })
            .cloned()
            .collect();

        if narrowed.is_empty() {
            self.rows.clone()
        } else {
            narrowed
        }
    }
}

/// `path` as a folder field shows it: `~` for the home folder `home` and
/// what is in it, else the whole path.
pub(crate) fn shown_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => String::from("~"),
        Ok(rest) => {
            let rest = rest.display();
            format!("~/{rest}")
        },
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
    use crate::testing::{folder_query, loaded_folder};

    const HOME: &str = "/home/user";

    fn home() -> &'static Path {
        Path::new(HOME)
    }

    fn query(folder: &str, list_in: &str, prefix: &str) -> FolderQuery {
        folder_query(FolderAsker::NewSessionDialog, folder, list_in, prefix)
    }

    fn found(query: FolderQuery, suggestions: &[&str]) -> LoadedFolder {
        loaded_folder(query, true, suggestions)
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
            vec![AgentsOutcome::LookUpFolder(folder_query(
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
        folder.set_text(String::from("/work/otty"), home());

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
        folder.set_text(String::from("~/.c"), home());

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
        folder.set_text(String::from("~/pro"), home());
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
        folder.set_text(String::from("/work/o"), home());
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
        folder.set_text(String::from("/work/o"), home());
        let _ = folder.apply(
            found(query("/work/o", "/work", "o"), &["/work/otty"]),
            home(),
        );

        folder.close();

        assert!(folder.suggestions(home()).is_empty());
    }

    /// The field after `~/p` was typed and its answer came: three folders.
    fn after_p() -> FolderSuggestions {
        let mut folder = field("~");
        folder.set_text(String::from("~/p"), home());
        let _ = folder.apply(
            found(
                query("/home/user/p", HOME, "p"),
                &[
                    "/home/user/Programs",
                    "/home/user/projects",
                    "/home/user/public",
                ],
            ),
            home(),
        );

        folder
    }

    /// Issue 39: while the answer about the new text is on its way, the
    /// list shows the rows of the last answer that the new text also
    /// stands for, case aside, and does not close.
    #[test]
    fn while_the_answer_is_on_its_way_the_rows_are_narrowed() {
        let mut folder = after_p();

        folder.set_text(String::from("~/pro"), home());

        assert_eq!(
            folder.suggestions(home()),
            vec![String::from("~/Programs/"), String::from("~/projects/")]
        );
    }

    /// Issue 39: the answer about the new text replaces the rows.
    #[test]
    fn the_answer_about_the_new_text_replaces_the_rows() {
        let mut folder = after_p();
        folder.set_text(String::from("~/pro"), home());

        let _ = folder.apply(
            found(
                query("/home/user/pro", HOME, "pro"),
                &[
                    "/home/user/Programs",
                    "/home/user/projects",
                    "/home/user/promo",
                ],
            ),
            home(),
        );

        assert_eq!(
            folder.suggestions(home()),
            vec![
                String::from("~/Programs/"),
                String::from("~/projects/"),
                String::from("~/promo/"),
            ]
        );
    }

    /// Issue 39: the list closes when the answer has no folders.
    #[test]
    fn an_answer_with_no_folders_closes_the_list() {
        let mut folder = after_p();
        folder.set_text(String::from("~/px"), home());

        let _ = folder
            .apply(found(query("/home/user/px", HOME, "px"), &[]), home());

        assert!(folder.suggestions(home()).is_empty());
    }

    /// Issue 39: rows the new text stands for none of, or rows of another
    /// folder, stay until the answer: the list does not blink.
    #[test]
    fn rows_that_can_not_be_narrowed_stay_until_the_answer() {
        let mut none_left = after_p();
        let mut inside = after_p();

        none_left.set_text(String::from("~/px"), home());
        inside.set_text(String::from("~/projects/"), home());

        let rows = vec![
            String::from("~/Programs/"),
            String::from("~/projects/"),
            String::from("~/public/"),
        ];
        assert_eq!(none_left.suggestions(home()), rows);
        assert_eq!(inside.suggestions(home()), rows);
    }

    /// Issue 39: a text that asks for nothing closes the list.
    #[test]
    fn a_text_that_asks_for_nothing_closes_the_list() {
        let mut empty = after_p();
        let mut relative = after_p();

        empty.set_text(String::new(), home());
        relative.set_text(String::from("work"), home());

        assert!(empty.suggestions(home()).is_empty());
        assert!(relative.suggestions(home()).is_empty());
    }

    /// Issue 39: a shown row can be highlighted; a row that is not there
    /// can not.
    #[test]
    fn a_shown_row_is_highlighted() {
        let mut folder = after_p();

        folder.highlight(Some(2));
        let highlighted = folder.highlighted();
        folder.highlight(Some(3));

        assert_eq!(highlighted, Some(2));
        assert_eq!(folder.highlighted(), Some(2), "there is no fourth row");
    }

    /// Issue 39: typing, closing and picking clear the highlight.
    #[test]
    fn typing_closing_and_picking_clear_the_highlight() {
        let mut typed = after_p();
        let mut closed = after_p();
        let mut picked = after_p();
        for folder in [&mut typed, &mut closed, &mut picked] {
            folder.highlight(Some(1));
        }

        typed.set_text(String::from("~/pr"), home());
        closed.close();
        let _ = picked.pick(1, home());

        assert_eq!(typed.highlighted(), None);
        assert_eq!(closed.highlighted(), None);
        assert_eq!(picked.highlighted(), None);
    }

    /// Issue 39: an answer that comes while a row is highlighted keeps the
    /// highlight on that folder, or drops it when the folder is gone.
    #[test]
    fn an_answer_keeps_the_highlight_on_its_folder() {
        let mut kept = after_p();
        kept.set_text(String::from("~/pro"), home());
        kept.highlight(Some(1));
        let mut gone = kept.clone();
        let pro = query("/home/user/pro", HOME, "pro");

        let _ = kept.apply(
            found(pro.clone(), &["/home/user/projects", "/home/user/promo"]),
            home(),
        );
        let _ = gone.apply(found(pro, &["/home/user/promo"]), home());

        assert_eq!(kept.highlighted(), Some(0));
        assert_eq!(gone.highlighted(), None);
    }

    /// Issue 39: a pick closes the rows; the answer about the picked folder
    /// lists its folders, the next level.
    #[test]
    fn the_answer_about_a_picked_folder_lists_the_next_level() {
        let mut folder = after_p();
        let _ = folder.pick(1, home());
        let right_after = folder.suggestions(home());

        let _ = folder.apply(
            found(
                query("/home/user/projects", "/home/user/projects", ""),
                &["/home/user/projects/otty"],
            ),
            home(),
        );

        assert!(right_after.is_empty());
        assert_eq!(
            folder.suggestions(home()),
            vec![String::from("~/projects/otty/")]
        );
    }

    #[test]
    fn a_folder_in_the_home_folder_is_written_with_a_tilde() {
        assert_eq!(shown_path(Path::new("/home/user/work"), home()), "~/work");
        assert_eq!(shown_path(home(), home()), "~");
        assert_eq!(shown_path(Path::new("/srv/app"), home()), "/srv/app");
    }

    /// Typing asks about the new text, unless it is not a full path; a
    /// pick puts the folder in the field and asks about its folders.
    #[test]
    fn typing_and_a_pick_ask_about_the_new_text() {
        let mut folder = after_p();

        let relative =
            folder.reduce(FolderFieldIntent::Changed("pro".into()), home());
        let typed =
            folder.reduce(FolderFieldIntent::Changed("~/pro".into()), home());
        let picked =
            folder.reduce(FolderFieldIntent::SuggestionPicked(1), home());
        let missing =
            folder.reduce(FolderFieldIntent::SuggestionPicked(5), home());

        assert!(relative.is_empty());
        assert_eq!(
            typed,
            vec![AgentsOutcome::LookUpFolder(query(
                "/home/user/pro",
                HOME,
                "pro"
            ))]
        );
        assert_eq!(
            picked,
            vec![AgentsOutcome::LookUpFolder(query(
                "/home/user/projects",
                "/home/user/projects",
                ""
            ))]
        );
        assert!(missing.is_empty(), "no such suggestion");
        assert_eq!(folder.text(), "~/projects/");
    }

    /// Moving the highlight and closing the list ask about nothing.
    #[test]
    fn a_highlight_and_closing_the_list_ask_nothing() {
        let mut folder = after_p();

        let highlighted = folder
            .reduce(FolderFieldIntent::SuggestionHighlighted(Some(1)), home());
        let row = folder.highlighted();
        let closed =
            folder.reduce(FolderFieldIntent::SuggestionsClosed, home());

        assert!(highlighted.is_empty());
        assert_eq!(row, Some(1));
        assert!(closed.is_empty());
        assert!(folder.suggestions(home()).is_empty());
    }
}
