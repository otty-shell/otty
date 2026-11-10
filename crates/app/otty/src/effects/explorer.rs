//! The folder reads of the Files section: the followed folder, a folder the
//! user pressed open and a watched folder that changed on disk. Each read
//! runs in the background, and its answer comes back as an explorer intent.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary reads folders"
)]

use std::path::{Path, PathBuf};

use iced::Task;
use otty_app_explorer::{ExplorerIntent, FileNode, TreePath};
use thiserror::Error;

use crate::message::AppEvent;

/// Errors emitted by explorer directory operations.
#[derive(Debug, Error)]
enum ExplorerError {
    #[error("Explorer I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Read the folder the explorer follows now in the background. Its failure
/// is only logged by the explorer.
pub(crate) fn load_root(root: PathBuf) -> Task<AppEvent> {
    load_directory_async(root.clone(), move |nodes| {
        ExplorerIntent::RootLoaded {
            root: root.clone(),
            nodes,
        }
    })
}

/// Read the folder the user pressed open in the background. Unlike the
/// loads that follow the terminal or the disk, its failure comes back as
/// its own intent, so the explorer can tell the user.
pub(crate) fn load_pressed_folder(
    path: TreePath,
    directory: PathBuf,
) -> Task<AppEvent> {
    Task::perform(
        async move { read_dir_nodes(&directory).map_err(|err| format!("{err}")) },
        move |result| AppEvent::Explorer(folder_loaded(path, result)),
    )
}

/// Read again, in the background, a watched folder that changed on disk.
/// Its failure is only logged by the explorer.
pub(crate) fn reload_folder(directory: PathBuf) -> Task<AppEvent> {
    load_directory_async(directory.clone(), move |nodes| {
        ExplorerIntent::DirectoryReloaded {
            directory: directory.clone(),
            nodes,
        }
    })
}

/// What a finished read of a pressed folder leads to: its files, or the
/// failure to tell the user about.
fn folder_loaded(
    path: TreePath,
    result: Result<Vec<FileNode>, String>,
) -> ExplorerIntent {
    match result {
        Ok(nodes) => ExplorerIntent::FolderLoaded { path, nodes },
        Err(message) => ExplorerIntent::FolderLoadFailed { path, message },
    }
}

/// Read a directory the user did not press in the background and produce a
/// completion event. Its failure is only logged.
fn load_directory_async<F>(dir: PathBuf, on_complete: F) -> Task<AppEvent>
where
    F: Fn(Vec<FileNode>) -> ExplorerIntent + Send + 'static,
{
    Task::perform(async move { read_dir_nodes(&dir) }, move |result| {
        let intent = match result {
            Ok(nodes) => on_complete(nodes),
            Err(err) => ExplorerIntent::LoadFailed {
                message: format!("{err}"),
            },
        };

        AppEvent::Explorer(intent)
    })
}

/// Load direct children for a file system directory.
///
/// Returns nodes sorted folders-first, then alphabetical.
fn read_dir_nodes(path: &Path) -> Result<Vec<FileNode>, ExplorerError> {
    let mut nodes = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!("explorer failed to read entry: {err}");
                continue;
            },
        };

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                log::warn!("explorer failed to read entry type: {err}");
                continue;
            },
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let is_folder = file_type.is_dir();

        nodes.push(FileNode::new(name, path, is_folder));
    }

    nodes.sort();

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use iced::Task;
    use iced::futures::StreamExt;
    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    /// Run a task that reads the disk on the runtime the app runs on and
    /// return the Files intents it sent.
    fn run_to_intents(task: Task<AppEvent>) -> Vec<ExplorerIntent> {
        let runtime = iced::executor::Default::new().expect("a tokio runtime");
        let stream = into_stream(task).expect("a task");

        let actions: Vec<Action<AppEvent>> = runtime.block_on(stream.collect());

        actions
            .into_iter()
            .filter_map(|action| match action {
                Action::Output(AppEvent::Explorer(intent)) => Some(intent),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn given_directory_with_files_when_loaded_then_nodes_are_returned() {
        let root = TempDir::new("explorer-load-directory");
        fs::create_dir_all(root.path().join("b-dir"))
            .expect("folder should be created");
        fs::write(root.path().join("a.txt"), "ok")
            .expect("file should be created");

        let nodes = read_dir_nodes(root.path()).expect("directory should load");

        assert_eq!(nodes.len(), 2);
        assert!(nodes[0].is_folder());
        assert_eq!(nodes[1].name(), "a.txt");
    }

    #[test]
    fn a_read_pressed_folder_gives_its_files() {
        let nodes = vec![FileNode::new(
            String::from("main.rs"),
            PathBuf::from("/tmp/project/src/main.rs"),
            false,
        )];

        let intent = folder_loaded(vec![String::from("src")], Ok(nodes));

        assert!(matches!(
            intent,
            ExplorerIntent::FolderLoaded { path, nodes }
                if path == [String::from("src")] && nodes.len() == 1
        ));
    }

    #[test]
    fn a_pressed_folder_that_cannot_be_read_gives_its_own_failure() {
        let intent = folder_loaded(
            vec![String::from("src")],
            Err(String::from("no access")),
        );

        assert!(matches!(
            intent,
            ExplorerIntent::FolderLoadFailed { path, message }
                if path == [String::from("src")] && message == "no access"
        ));
    }

    /// Only the load that a press started reports its own failure; the
    /// root load that follows the terminal and the reload after a change on
    /// disk keep the log-only one.
    #[test]
    fn only_the_load_of_a_pressed_folder_reports_a_failure_to_tell() {
        let root = TempDir::new("explorer-effects-missing");
        let missing = root.path().join("missing");

        let pressed = run_to_intents(load_pressed_folder(
            vec![String::from("missing")],
            missing.clone(),
        ));
        let followed = run_to_intents(load_root(missing.clone()));
        let reloaded = run_to_intents(reload_folder(missing));

        assert!(matches!(
            pressed.as_slice(),
            [ExplorerIntent::FolderLoadFailed { path, .. }]
                if path == &[String::from("missing")]
        ));
        assert!(matches!(
            followed.as_slice(),
            [ExplorerIntent::LoadFailed { .. }]
        ));
        assert!(matches!(
            reloaded.as_slice(),
            [ExplorerIntent::LoadFailed { .. }]
        ));
    }

    /// A read folder comes back with its entries, under the folder it was
    /// asked for.
    #[test]
    fn a_read_root_and_a_reloaded_folder_give_their_files() {
        let root = TempDir::new("explorer-effects-read");
        fs::write(root.path().join("notes.txt"), "ok")
            .expect("file should be created");
        let folder = root.path().to_path_buf();

        let followed = run_to_intents(load_root(folder.clone()));
        let reloaded = run_to_intents(reload_folder(folder.clone()));

        assert!(matches!(
            followed.as_slice(),
            [ExplorerIntent::RootLoaded { root, nodes }]
                if *root == folder && nodes.len() == 1
        ));
        assert!(matches!(
            reloaded.as_slice(),
            [ExplorerIntent::DirectoryReloaded { directory, nodes }]
                if *directory == folder && nodes[0].name() == "notes.txt"
        ));
    }
}
