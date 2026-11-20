//! `files.remove`: deletes one file, or one empty folder, that the called
//! part declares in `write`, such as the session files a probe of an
//! agent caused.

use std::path::Path;
use std::{fs, io};

use otty_plugin_manifest::Permissions;

use crate::operations::v0_1::otty::host::types::HostError;

/// Removes the file or the empty folder `path`, when the part with
/// `permissions` may write it. A folder that is not empty is kept; a
/// link is removed itself, never what it points to.
pub(crate) fn remove(
    permissions: &Permissions,
    path: &Path,
) -> Result<(), HostError> {
    if !permissions.may_write(path) {
        return Err(HostError::NotAllowed(path.display().to_string()));
    }

    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error(&error, path))?;

    let removed = if metadata.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };
    removed.map_err(|error| io_error(&error, path))
}

/// An I/O error of `path` in the words of a host error.
fn io_error(error: &io::Error, path: &Path) -> HostError {
    match error.kind() {
        io::ErrorKind::NotFound => {
            HostError::NotFound(path.display().to_string())
        },
        io::ErrorKind::DirectoryNotEmpty => HostError::Failed(format!(
            "{} is a folder that is not empty",
            path.display()
        )),
        _ => HostError::Failed(format!("{}: {error}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::Environment;

    use super::*;
    use crate::testing::{TempDir, programs_permissions};

    /// The permissions of the test part with `home` as the home folder:
    /// it writes in `~/.test-agent/probe/`.
    fn permissions(home: &Path) -> std::sync::Arc<Permissions> {
        programs_permissions("test-agent", &Environment::new(home.into()))
    }

    fn write(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("the folder");
        }

        fs::write(path, b"{}\n").expect("the file");
    }

    #[test]
    fn a_file_inside_the_write_globs_is_removed() {
        let home = TempDir::new();
        let file = home.path().join(".test-agent/probe/a.jsonl");
        write(&file);

        let removed = remove(&permissions(home.path()), &file);

        assert_eq!(format!("{removed:?}"), "Ok(())");
        assert!(!file.exists());
    }

    #[test]
    fn removal_outside_the_write_globs_is_refused() {
        let home = TempDir::new();
        let state = home.path().join(".test-agent/state.json");
        let sneaky = home.path().join(".test-agent/probe/../state.json");
        write(&state);

        let outside = remove(&permissions(home.path()), &state);
        let through_parent = remove(&permissions(home.path()), &sneaky);

        assert!(matches!(outside, Err(HostError::NotAllowed(_))));
        assert!(matches!(through_parent, Err(HostError::NotAllowed(_))));
        assert!(state.exists());
    }

    #[test]
    fn an_empty_folder_is_removed_and_a_full_one_is_kept() {
        let home = TempDir::new();
        let empty = home.path().join(".test-agent/probe/empty");
        let full = home.path().join(".test-agent/probe/full");
        fs::create_dir_all(&empty).expect("the folder");
        write(&full.join("a.jsonl"));

        let removed_empty = remove(&permissions(home.path()), &empty);
        let removed_full = remove(&permissions(home.path()), &full);

        assert_eq!(format!("{removed_empty:?}"), "Ok(())");
        assert!(!empty.exists());
        assert!(matches!(removed_full, Err(HostError::Failed(_))));
        assert!(full.join("a.jsonl").exists());
    }

    #[test]
    fn a_missing_file_is_not_found() {
        let home = TempDir::new();
        let missing = home.path().join(".test-agent/probe/missing.jsonl");

        let removed = remove(&permissions(home.path()), &missing);

        assert!(matches!(removed, Err(HostError::NotFound(_))));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_removed_and_what_it_points_to_is_kept() {
        let home = TempDir::new();
        let target = home.path().join("kept.txt");
        let link = home.path().join(".test-agent/probe/link");
        write(&target);
        fs::create_dir_all(link.parent().expect("a parent")).expect("folder");
        std::os::unix::fs::symlink(&target, &link).expect("a link");

        let removed = remove(&permissions(home.path()), &link);

        assert_eq!(format!("{removed:?}"), "Ok(())");
        assert!(target.exists());
        assert!(fs::symlink_metadata(&link).is_err());
    }
}
