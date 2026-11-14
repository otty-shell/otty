//! The disk operations the user asks for in the Files section: a new
//! file or folder, a rename or a move, a deletion. Each operation runs in the
//! background, and its answer comes back as an explorer intent.

use std::fs::{self, File};
use std::io::{self, ErrorKind};
use std::path::Path;

use iced::Task;
use otty_app_explorer::{DiskOperation, ExplorerIntent};

/// Run `operation` in the background. It answers with
/// [`ExplorerIntent::DiskOperationDone`], or with [`ExplorerIntent::DiskOperationFailed`]
/// and the reason. Nothing is ever replaced: a new entry or a move whose
/// name is taken fails.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_app_explorer::DiskOperation;
///
/// let create =
///     otty_app_explorer_io::run_disk_operation(DiskOperation::CreateFolder {
///         path: PathBuf::from("/tmp/otty-doc-example"),
///     });
///
/// assert_eq!(create.units(), 1);
/// ```
pub fn run_disk_operation(operation: DiskOperation) -> Task<ExplorerIntent> {
    Task::perform(
        async move {
            let result = apply(&operation).map_err(|err| format!("{err}"));
            (operation, result)
        },
        |(operation, result)| operation_answer(operation, result),
    )
}

/// What a finished operation leads to: the operation, or its failure to tell
/// the user about.
fn operation_answer(
    operation: DiskOperation,
    result: Result<(), String>,
) -> ExplorerIntent {
    match result {
        Ok(()) => ExplorerIntent::DiskOperationDone { operation },
        Err(message) => {
            ExplorerIntent::DiskOperationFailed { operation, message }
        },
    }
}

/// Run `operation` on the disk now.
fn apply(operation: &DiskOperation) -> io::Result<()> {
    match operation {
        DiskOperation::CreateFile { path } => {
            File::options().write(true).create_new(true).open(path)?;
            Ok(())
        },
        DiskOperation::CreateFolder { path } => fs::create_dir(path),
        DiskOperation::Move { from, to } => move_entry(from, to),
        DiskOperation::Delete { path } => delete(path),
    }
}

/// Move `from` to `to`, unless something is at `to` already: a plain
/// rename would replace a file there.
fn move_entry(from: &Path, to: &Path) -> io::Result<()> {
    if to.symlink_metadata().is_ok() {
        return Err(io::Error::from(ErrorKind::AlreadyExists));
    }

    fs::rename(from, to)
}

/// Delete a file, or a folder with everything in it. A link is deleted
/// itself, never what it points to.
fn delete(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;

    if metadata.is_dir() {
        return fs::remove_dir_all(path);
    }

    fs::remove_file(path)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use iced::futures::StreamExt;
    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    /// Run an operation on the runtime the app runs on and return its answer.
    fn run(operation: DiskOperation) -> ExplorerIntent {
        let runtime = iced::executor::Default::new().expect("a tokio runtime");
        let stream =
            into_stream(run_disk_operation(operation)).expect("a task");

        let actions: Vec<Action<ExplorerIntent>> =
            runtime.block_on(stream.collect());

        actions
            .into_iter()
            .find_map(|action| match action {
                Action::Output(intent) => Some(intent),
                _ => None,
            })
            .expect("the operation should answer")
    }

    fn is_done(intent: &ExplorerIntent) -> bool {
        matches!(intent, ExplorerIntent::DiskOperationDone { .. })
    }

    fn is_failed(intent: &ExplorerIntent) -> bool {
        matches!(intent, ExplorerIntent::DiskOperationFailed { .. })
    }

    #[test]
    fn a_new_file_and_a_new_folder_are_created_empty() {
        let root = TempDir::new("explorer-create");
        let file = root.path().join("notes.md");
        let folder = root.path().join("docs");

        let file_answer = run(DiskOperation::CreateFile { path: file.clone() });
        let folder_answer = run(DiskOperation::CreateFolder {
            path: folder.clone(),
        });

        assert!(is_done(&file_answer), "{file_answer:?}");
        assert!(is_done(&folder_answer), "{folder_answer:?}");
        assert_eq!(fs::read_to_string(&file).expect("the file"), "");
        assert!(folder.is_dir());
    }

    /// A new entry never replaces one with its name.
    #[test]
    fn a_new_entry_on_a_taken_name_fails_and_keeps_the_old_one() {
        let root = TempDir::new("explorer-create-taken");
        let file = root.path().join("notes.md");
        fs::write(&file, "keep").expect("file should be created");

        let file_answer = run(DiskOperation::CreateFile { path: file.clone() });
        let folder_answer =
            run(DiskOperation::CreateFolder { path: file.clone() });

        assert!(is_failed(&file_answer), "{file_answer:?}");
        assert!(is_failed(&folder_answer), "{folder_answer:?}");
        assert_eq!(fs::read_to_string(&file).expect("the file"), "keep");
    }

    #[test]
    fn a_moved_folder_takes_what_is_in_it() {
        let root = TempDir::new("explorer-move");
        let from = root.path().join("src");
        fs::create_dir_all(&from).expect("folder should be created");
        fs::write(from.join("lib.rs"), "ok").expect("file should be created");
        let to = root.path().join("code");

        let answer = run(DiskOperation::Move {
            from: from.clone(),
            to: to.clone(),
        });

        assert!(is_done(&answer), "{answer:?}");
        assert!(!from.exists());
        assert_eq!(fs::read_to_string(to.join("lib.rs")).expect("moved"), "ok");
    }

    /// A move never replaces a file at its new place.
    #[test]
    fn a_move_onto_a_taken_name_fails_and_keeps_both() {
        let root = TempDir::new("explorer-move-taken");
        let from = root.path().join("a.txt");
        let to = root.path().join("b.txt");
        fs::write(&from, "a").expect("file should be created");
        fs::write(&to, "b").expect("file should be created");

        let answer = run(DiskOperation::Move {
            from: from.clone(),
            to: to.clone(),
        });

        assert!(is_failed(&answer), "{answer:?}");
        assert_eq!(fs::read_to_string(&from).expect("from"), "a");
        assert_eq!(fs::read_to_string(&to).expect("to"), "b");
    }

    #[test]
    fn a_deleted_file_and_a_deleted_folder_are_gone() {
        let root = TempDir::new("explorer-delete");
        let file = root.path().join("notes.md");
        let folder = root.path().join("docs");
        fs::write(&file, "x").expect("file should be created");
        fs::create_dir_all(folder.join("deep")).expect("folder");
        fs::write(folder.join("deep/a.txt"), "x").expect("file");

        let file_answer = run(DiskOperation::Delete { path: file.clone() });
        let folder_answer = run(DiskOperation::Delete {
            path: folder.clone(),
        });

        assert!(is_done(&file_answer), "{file_answer:?}");
        assert!(is_done(&folder_answer), "{folder_answer:?}");
        assert!(!file.exists());
        assert!(!folder.exists());
    }

    /// Deleting a link to a folder deletes the link, never the folder.
    #[cfg(unix)]
    #[test]
    fn a_deleted_link_leaves_what_it_points_to() {
        let root = TempDir::new("explorer-delete-link");
        let folder = root.path().join("real");
        fs::create_dir_all(&folder).expect("folder should be created");
        fs::write(folder.join("a.txt"), "x").expect("file should be created");
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&folder, &link).expect("link");

        let answer = run(DiskOperation::Delete { path: link.clone() });

        assert!(is_done(&answer), "{answer:?}");
        assert!(link.symlink_metadata().is_err());
        assert!(folder.join("a.txt").exists());
    }

    #[test]
    fn a_change_of_a_missing_entry_fails_with_its_reason() {
        let root = TempDir::new("explorer-missing");
        let missing = root.path().join("missing");

        let answer = run(DiskOperation::Delete {
            path: missing.clone(),
        });

        assert!(matches!(
            answer,
            ExplorerIntent::DiskOperationFailed {
                operation: DiskOperation::Delete { path },
                message,
            } if path == missing && !message.is_empty()
        ));
    }

    #[test]
    fn the_answer_carries_the_change() {
        let operation = DiskOperation::CreateFile {
            path: PathBuf::from("/tmp/a"),
        };

        let answer = operation_answer(operation.clone(), Ok(()));

        assert!(matches!(
            answer,
            ExplorerIntent::DiskOperationDone { operation: done } if done == operation
        ));
    }
}
