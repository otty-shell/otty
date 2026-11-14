use std::path::{Path, PathBuf};

/// A disk operation the user asked for in Files: a new file or folder,
/// a rename or a move, or a deletion.
///
/// The explorer decides on it; `otty-app-explorer-io` runs it and answers
/// with [`ExplorerIntent::DiskOperationDone`](crate::ExplorerIntent::DiskOperationDone) or
/// [`ExplorerIntent::DiskOperationFailed`](crate::ExplorerIntent::DiskOperationFailed).
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_app_explorer::DiskOperation;
///
/// let operation = DiskOperation::Move {
///     from: PathBuf::from("/p/src/main.rs"),
///     to: PathBuf::from("/p/main.rs"),
/// };
///
/// assert_eq!(operation.name(), "main.rs");
/// assert_eq!(
///     operation.changed_folders(),
///     vec![PathBuf::from("/p/src"), PathBuf::from("/p")]
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskOperation {
    /// Create an empty file; it fails when the name is taken.
    CreateFile {
        /// Path of the new file.
        path: PathBuf,
    },
    /// Create an empty folder; it fails when the name is taken.
    CreateFolder {
        /// Path of the new folder.
        path: PathBuf,
    },
    /// Rename a file or a folder in its folder, or move it to another one;
    /// it fails when `to` is taken.
    Move {
        /// Where the entry is now.
        from: PathBuf,
        /// Where the entry goes.
        to: PathBuf,
    },
    /// Delete a file, or a folder with everything in it, for good.
    Delete {
        /// Path of the entry.
        path: PathBuf,
    },
}

impl DiskOperation {
    /// Return the name of the entry the operation is about, as the user knows
    /// it: the new name of a new entry, the old one of a moved or deleted
    /// entry.
    pub fn name(&self) -> String {
        let path = match self {
            DiskOperation::CreateFile { path }
            | DiskOperation::CreateFolder { path }
            | DiskOperation::Delete { path } => path,
            DiskOperation::Move { from, .. } => from,
        };

        entry_name(path)
    }

    /// Return the folders whose list of entries the operation alters: the
    /// folder of the entry, and for a move into another folder that folder
    /// too.
    pub fn changed_folders(&self) -> Vec<PathBuf> {
        let mut folders = Vec::new();
        match self {
            DiskOperation::CreateFile { path }
            | DiskOperation::CreateFolder { path }
            | DiskOperation::Delete { path } => push_parent(&mut folders, path),
            DiskOperation::Move { from, to } => {
                push_parent(&mut folders, from);
                push_parent(&mut folders, to);
            },
        }

        folders.dedup();
        folders
    }
}

/// The last part of `path`, or the whole path when it has none.
fn entry_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn push_parent(folders: &mut Vec<PathBuf>, path: &Path) {
    if let Some(parent) = path.parent() {
        folders.push(parent.to_path_buf());
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::DiskOperation;

    #[test]
    fn a_new_entry_is_named_by_its_new_name_and_changes_its_folder() {
        let operation = DiskOperation::CreateFolder {
            path: PathBuf::from("/p/src/utils"),
        };

        assert_eq!(operation.name(), "utils");
        assert_eq!(operation.changed_folders(), vec![PathBuf::from("/p/src")]);
    }

    #[test]
    fn a_rename_in_place_changes_one_folder() {
        let operation = DiskOperation::Move {
            from: PathBuf::from("/p/a.txt"),
            to: PathBuf::from("/p/b.txt"),
        };

        assert_eq!(operation.name(), "a.txt");
        assert_eq!(operation.changed_folders(), vec![PathBuf::from("/p")]);
    }

    #[test]
    fn a_deleted_entry_is_named_by_its_name() {
        let operation = DiskOperation::Delete {
            path: PathBuf::from("/p/old"),
        };

        assert_eq!(operation.name(), "old");
        assert_eq!(operation.changed_folders(), vec![PathBuf::from("/p")]);
    }
}
