//! Opens the top data folder of an agent and finds the files in a folder
//! and in the folders inside it, at any depth, such as the session files
//! the agents keep in folders by project or by date.

use std::fs::ReadDir;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{SourceError, Unavailable};

/// The files among `entries`, and in the folders among them at any depth,
/// whose path `is_wanted` picks.
///
/// A folder inside that can not be read is skipped. Links are not
/// followed, so a link can not lead in a loop.
pub(crate) fn files_in_tree(
    entries: ReadDir,
    is_wanted: impl Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    add_files(entries, &is_wanted, &mut files);

    files
}

/// Opens `folder`, a top data folder of an agent such as its `sessions` or
/// `projects` folder.
///
/// A missing folder is `None`: the agent has not written anything there
/// yet. A file in its place is "not available", and a folder that can not
/// be read is "unreadable".
pub(crate) fn open_data_folder(
    folder: &Path,
) -> Result<Option<ReadDir>, SourceError> {
    match std::fs::read_dir(folder) {
        Ok(entries) => Ok(Some(entries)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) if error.kind() == ErrorKind::NotADirectory => Err(
            SourceError::NotAvailable(Unavailable::NotAFolder(folder.into())),
        ),
        Err(error) => Err(SourceError::Unreadable {
            path: folder.into(),
            detail: error.to_string(),
        }),
    }
}

/// Adds the files among `entries` that `is_wanted` picks, and those in the
/// folders among them, to `files`.
fn add_files(
    entries: ReadDir,
    is_wanted: &dyn Fn(&Path) -> bool,
    files: &mut Vec<PathBuf>,
) {
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();

        if file_type.is_dir() {
            if let Ok(inner) = std::fs::read_dir(&path) {
                add_files(inner, is_wanted, files);
            }
        } else if file_type.is_file() && is_wanted(&path) {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, write_file};

    #[test]
    fn the_wanted_files_are_found_at_any_depth() {
        let temp = TempDir::new();
        write_file(&temp.path().join("a.jsonl"), "{}");
        write_file(&temp.path().join("deep/er/b.jsonl"), "{}");
        write_file(&temp.path().join("deep/notes.txt"), "kept out");
        std::fs::create_dir_all(temp.path().join("c.jsonl"))
            .expect("create a folder named like a wanted file");
        let entries = std::fs::read_dir(temp.path()).expect("read the folder");

        let mut files = files_in_tree(entries, |path| {
            path.extension().is_some_and(|ext| ext == "jsonl")
        });

        files.sort();
        assert_eq!(
            files,
            [
                temp.path().join("a.jsonl"),
                temp.path().join("deep/er/b.jsonl")
            ]
        );
    }
}
