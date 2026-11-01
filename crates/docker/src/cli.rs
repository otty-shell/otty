//! Which engine command line tools are installed, found on `PATH` with std
//! only.

use std::ffi::OsStr;
use std::path::Path;

/// Which engine command line tools are on `PATH`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CliPresence {
    docker: bool,
    podman: bool,
}

impl CliPresence {
    /// `docker` is on `PATH`.
    pub fn docker(&self) -> bool {
        self.docker
    }

    /// `podman` is on `PATH`.
    pub fn podman(&self) -> bool {
        self.podman
    }

    /// Look for `docker` and `podman` in a `PATH` value.
    pub(crate) fn from_path(path: Option<&OsStr>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };

        Self {
            docker: is_on_path("docker", path),
            podman: is_on_path("podman", path),
        }
    }
}

#[cfg(test)]
impl CliPresence {
    /// Create a presence from known answers.
    pub(crate) fn new(docker: bool, podman: bool) -> Self {
        Self { docker, podman }
    }
}

/// `name` is an executable file in one of the `PATH` directories.
fn is_on_path(name: &str, path: &OsStr) -> bool {
    std::env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .any(|dir| is_executable_file(&dir.join(name)))
}

fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path).is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::test_support::TempDir;

    fn write_file(dir: &Path, name: &str, mode: u32) {
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").expect("write test file");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
            .expect("set test file mode");
    }

    fn path_of(dirs: &[&Path]) -> OsString {
        std::env::join_paths(dirs).expect("join test PATH")
    }

    #[test]
    fn finds_executables_in_any_path_dir() {
        let first = TempDir::new();
        let second = TempDir::new();
        write_file(first.path(), "docker", 0o755);
        write_file(second.path(), "podman", 0o755);
        let path = path_of(&[first.path(), second.path()]);

        let presence = CliPresence::from_path(Some(&path));

        assert_eq!(presence, CliPresence::new(true, true));
    }

    #[test]
    fn file_without_execute_bit_does_not_count() {
        let dir = TempDir::new();
        write_file(dir.path(), "docker", 0o644);
        let path = path_of(&[dir.path()]);

        let presence = CliPresence::from_path(Some(&path));

        assert!(!presence.docker());
    }

    #[test]
    fn directory_with_the_tool_name_does_not_count() {
        let dir = TempDir::new();
        std::fs::create_dir(dir.path().join("podman")).expect("create dir");
        let path = path_of(&[dir.path()]);

        let presence = CliPresence::from_path(Some(&path));

        assert!(!presence.podman());
    }

    #[test]
    fn missing_dirs_and_missing_path_find_nothing() {
        let path = OsString::from("/no/such/dir:");

        assert_eq!(CliPresence::from_path(Some(&path)), CliPresence::default());
        assert_eq!(CliPresence::from_path(None), CliPresence::default());
    }
}
