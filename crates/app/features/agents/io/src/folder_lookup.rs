//! The look-up of the folder typed in the New session dialog: whether it is
//! a folder, and the folders it may stand for. It reads one folder, on a
//! thread of its own, so a slow disk never holds up the window.

use std::path::{Path, PathBuf};

use iced::Task;
use otty_app_agents::{AgentsIntent, FolderQuery, LoadedFolder};

use crate::load::on_thread;

/// The most folders suggested at once.
const MOST_SUGGESTIONS: usize = 50;

/// Look up the folder of `query` in the background. The answer comes back
/// as [`AgentsIntent::FolderLookedUp`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_app_agents::FolderQuery;
///
/// let look_up = otty_app_agents_io::look_up_folder(FolderQuery {
///     folder: PathBuf::from("/no/such/folder"),
///     list_in: PathBuf::from("/no/such"),
///     prefix: String::from("folder"),
/// });
///
/// assert_eq!(look_up.units(), 1);
/// ```
pub fn look_up_folder(query: FolderQuery) -> Task<AgentsIntent> {
    let asked = query.clone();

    Task::perform(on_thread(move || read_folder(query)), move |read| {
        let loaded = read.unwrap_or_else(|| LoadedFolder {
            query: asked,
            is_folder: false,
            suggestions: Vec::new(),
        });
        AgentsIntent::FolderLookedUp(loaded)
    })
}

/// Whether the folder of `query` is a folder, and the folders of its
/// `list_in` whose name starts with its `prefix`, case aside, sorted by
/// name. A hidden folder is suggested only when the prefix starts with a
/// dot; a folder that can not be read suggests nothing.
fn read_folder(query: FolderQuery) -> LoadedFolder {
    let is_folder = query.folder.is_dir();
    let suggestions = folders_in(&query.list_in, &query.prefix);

    LoadedFolder {
        query,
        is_folder,
        suggestions,
    }
}

/// The folders of `parent` whose name starts with `prefix`, case aside.
fn folders_in(parent: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let prefix = prefix.to_lowercase();
    let shows_hidden = prefix.starts_with('.');

    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            name.starts_with(&prefix)
                && (shows_hidden || !name.starts_with('.'))
        })
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    folders.truncate(MOST_SUGGESTIONS);

    folders
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    /// A folder with the folders `projects`, `promo`, `Prod` and `.private`
    /// and the file `profile.txt`.
    fn home() -> TempDir {
        let home = TempDir::new("agents-folder-lookup");
        for name in ["projects", "promo", "Prod", ".private"] {
            std::fs::create_dir(home.path().join(name))
                .expect("the folder should be made");
        }
        std::fs::write(home.path().join("profile.txt"), "")
            .expect("the file should be written");

        home
    }

    fn query(folder: PathBuf, list_in: PathBuf, prefix: &str) -> FolderQuery {
        FolderQuery {
            folder,
            list_in,
            prefix: String::from(prefix),
        }
    }

    /// The folders whose name starts with the typed part, case aside, are
    /// suggested by name; files and hidden folders are not.
    #[test]
    fn the_folders_that_start_with_the_typed_name_are_suggested() {
        let home = home();
        let path = home.path();

        let loaded =
            read_folder(query(path.join("pro"), path.to_path_buf(), "pro"));

        assert!(!loaded.is_folder);
        assert_eq!(
            loaded.suggestions,
            vec![path.join("Prod"), path.join("projects"), path.join("promo")]
        );
    }

    #[test]
    fn a_folder_that_is_there_is_found() {
        let home = home();
        let path = home.path();

        let loaded = read_folder(query(
            path.join("projects"),
            path.to_path_buf(),
            "projects",
        ));
        let file = read_folder(query(
            path.join("profile.txt"),
            path.to_path_buf(),
            "profile.txt",
        ));

        assert!(loaded.is_folder);
        assert_eq!(loaded.suggestions, vec![path.join("projects")]);
        assert!(!file.is_folder, "a file is no folder");
    }

    /// A hidden folder is suggested only when its dot is typed.
    #[test]
    fn a_hidden_folder_is_suggested_once_its_dot_is_typed() {
        let home = home();
        let path = home.path();

        let loaded =
            read_folder(query(path.join("."), path.to_path_buf(), "."));

        assert_eq!(loaded.suggestions, vec![path.join(".private")]);
    }

    #[test]
    fn a_folder_that_can_not_be_read_suggests_nothing() {
        let home = home();
        let gone = home.path().join("gone");

        let loaded = read_folder(query(gone.join("x"), gone, "x"));

        assert!(!loaded.is_folder);
        assert!(loaded.suggestions.is_empty());
    }

    #[test]
    fn the_look_up_runs_in_the_background() {
        let home = home();
        let path = home.path();

        let task =
            look_up_folder(query(path.join("pro"), path.to_path_buf(), "pro"));

        assert_eq!(task.units(), 1);
    }
}
