//! Whether a file can be run as a program.

use std::path::Path;

/// Return whether `path` is a file the user may run: on Unix a file with
/// an execute bit set, elsewhere any file. A folder, a missing path or one
/// that can not be read is not.
///
/// # Example
///
/// ```
/// use std::path::Path;
///
/// use otty_app_shell::is_executable_file;
///
/// assert!(!is_executable_file(Path::new("/no/such/program")));
/// ```
pub fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    fn file_with_mode(folder: &TempDir, name: &str, mode: u32) -> PathBuf {
        let path = folder.path().join(name);
        std::fs::write(&path, "#!/bin/sh\n").expect("the file is written");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
            .expect("the mode is set");

        path
    }

    #[test]
    fn a_file_with_an_execute_bit_can_be_run() {
        let folder = TempDir::new("shell-program-file");
        let program = file_with_mode(&folder, "program", 0o755);
        let text = file_with_mode(&folder, "notes.txt", 0o644);

        assert!(is_executable_file(&program));
        assert!(!is_executable_file(&text));
    }

    #[test]
    fn a_folder_or_a_missing_path_can_not_be_run() {
        let folder = TempDir::new("shell-program-folder");

        assert!(!is_executable_file(folder.path()));
        assert!(!is_executable_file(&folder.path().join("missing")));
    }
}
