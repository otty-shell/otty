//! Which engine command line tools are installed.
//!
//! `docker` and `podman` are looked for on `PATH` and then in the usual
//! install folders, because an app started from the macOS Finder gets a
//! short `PATH`. The lookup uses std only. A `docker` command that is really
//! Podman (Podman's `podman-docker` wrapper, or a link to the `podman`
//! binary) counts as Podman (spec Decision 8).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

/// Install folder of Homebrew on Intel Macs and of many manual installs.
const LOCAL_BIN: &str = "/usr/local/bin";
/// Install folder of Homebrew on Apple silicon Macs.
const HOMEBREW_BIN: &str = "/opt/homebrew/bin";
/// Docker Desktop's per-user CLI folder, relative to `$HOME`.
const DOCKER_USER_BIN: &str = ".docker/bin";
/// Docker Desktop's CLI folder inside the macOS app.
const DOCKER_APP_BIN: &str = "/Applications/Docker.app/Contents/Resources/bin";
/// Install folder of Podman's macOS installer.
const PODMAN_INSTALLER_BIN: &str = "/opt/podman/bin";

/// Which engine command line tools are installed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CliPresence {
    docker: bool,
    podman: bool,
}

impl CliPresence {
    /// Docker's `docker` command is installed. A `docker` command that is
    /// really Podman does not count here.
    pub fn docker(&self) -> bool {
        self.docker
    }

    /// `podman` is installed, or a `docker` command that is really Podman.
    pub fn podman(&self) -> bool {
        self.podman
    }

    /// Look for `docker` and `podman` in `dirs` and tell whether `docker` is
    /// really Podman. `docker --version` runs only when `docker` is found
    /// and is not a link to `podman`, and is stopped after `limit`.
    pub(crate) async fn find(dirs: &[PathBuf], limit: Duration) -> Self {
        let docker = find_command("docker", dirs);
        let podman_found = find_command("podman", dirs).is_some();

        let docker_is_podman = match &docker {
            Some(command) => is_podman_command(command, limit).await,
            None => false,
        };

        Self::from_lookup(docker.is_some(), docker_is_podman, podman_found)
    }

    /// Combine what the lookup found: a `docker` command that is really
    /// Podman counts as `podman`, not as `docker`.
    pub(crate) fn from_lookup(
        docker_found: bool,
        docker_is_podman: bool,
        podman_found: bool,
    ) -> Self {
        let podman_docker = docker_found && docker_is_podman;

        Self {
            docker: docker_found && !docker_is_podman,
            podman: podman_found || podman_docker,
        }
    }
}

#[cfg(any(test, feature = "fixtures"))]
impl CliPresence {
    /// Create a presence from known answers.
    pub(crate) fn new(docker: bool, podman: bool) -> Self {
        Self { docker, podman }
    }
}

/// The folders to search for commands: the `PATH` folders (empty entries
/// skipped), then the usual install folders in the order of spec
/// Decision 8.
pub(crate) fn command_dirs(
    path: Option<&OsStr>,
    home: Option<&Path>,
) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = match path {
        Some(path) => std::env::split_paths(path)
            .filter(|dir| !dir.as_os_str().is_empty())
            .collect(),
        None => Vec::new(),
    };

    dirs.push(PathBuf::from(LOCAL_BIN));
    dirs.push(PathBuf::from(HOMEBREW_BIN));
    if let Some(home) = home {
        dirs.push(home.join(DOCKER_USER_BIN));
    }
    dirs.push(PathBuf::from(DOCKER_APP_BIN));
    dirs.push(PathBuf::from(PODMAN_INSTALLER_BIN));

    dirs
}

/// The first executable file called `name` in `dirs`.
fn find_command(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable_file(candidate))
}

/// The `docker --version` answer names Podman. Podman's `podman-docker`
/// wrapper runs `podman`, which prints `podman version 5.8.8`; Docker prints
/// `Docker version 29.9.1, build dfc520d`.
fn version_names_podman(output: &str) -> bool {
    output.to_ascii_lowercase().contains("podman")
}

/// The `docker` command, with links followed, is the `podman` binary. Run
/// through such a link Podman prints `docker version 5.8.8`, which does not
/// name Podman.
fn links_to_podman(resolved: &Path) -> bool {
    resolved.file_name() == Some(OsStr::new("podman"))
}

/// The `docker` command at `command` is really Podman.
async fn is_podman_command(command: &Path, limit: Duration) -> bool {
    let resolved = std::fs::canonicalize(command)
        .unwrap_or_else(|_| command.to_path_buf());
    if links_to_podman(&resolved) {
        return true;
    }

    read_version(command, limit)
        .await
        .is_some_and(|output| version_names_podman(&output))
}

/// Run `<command> --version` and return what it printed on stdout. `None`
/// when it cannot run, fails, or takes longer than `limit`; the process is
/// killed then, so discovery never waits on it.
async fn read_version(command: &Path, limit: Duration) -> Option<String> {
    let run = Command::new(command)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output();

    let path = command.display();
    let output = match tokio::time::timeout(limit, run).await {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            log::debug!("docker discovery: {path} --version: {error}");
            return None;
        },
        Err(_) => {
            log::debug!("docker discovery: {path} --version timed out");
            return None;
        },
    };
    if !output.status.success() {
        let status = output.status;
        log::debug!("docker discovery: {path} --version: {status}");
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).into_owned())
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
    use std::time::Instant;

    use super::*;
    use crate::test_support::TempDir;

    /// What Docker's own CLI prints (Docker 29.9.1 on Fedora 43).
    const DOCKER_VERSION: &str = "Docker version 29.9.1, build dfc520d\n";
    /// What Podman's `podman-docker` wrapper prints on stdout (Podman 5.8.8).
    const PODMAN_DOCKER_VERSION: &str = "podman version 5.8.8\n";
    /// A time limit long enough for a shell script on a busy test machine.
    const LIMIT: Duration = Duration::from_secs(5);

    fn write_file(dir: &Path, name: &str, mode: u32) {
        write_script(dir, name, "", mode);
    }

    fn write_script(dir: &Path, name: &str, body: &str, mode: u32) {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}"))
            .expect("write test file");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
            .expect("set test file mode");
    }

    fn path_of(dirs: &[&Path]) -> OsString {
        std::env::join_paths(dirs).expect("join test PATH")
    }

    fn install_dirs_with_home(home: &str) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from(home).join(".docker/bin"),
            PathBuf::from("/Applications/Docker.app/Contents/Resources/bin"),
            PathBuf::from("/opt/podman/bin"),
        ]
    }

    #[test]
    fn command_dirs_lists_path_then_install_folders() {
        let path = OsString::from("/usr/bin:/bin");

        let dirs = command_dirs(Some(&path), Some(Path::new("/Users/ann")));

        let mut expected =
            vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")];
        expected.extend(install_dirs_with_home("/Users/ann"));
        assert_eq!(dirs, expected);
    }

    #[test]
    fn command_dirs_skips_empty_path_entries_and_home_folder_without_home() {
        let path = OsString::from(":/usr/bin:");

        let dirs = command_dirs(Some(&path), None);

        assert_eq!(
            dirs,
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from(
                    "/Applications/Docker.app/Contents/Resources/bin"
                ),
                PathBuf::from("/opt/podman/bin"),
            ]
        );
    }

    #[test]
    fn command_dirs_without_path_still_has_install_folders() {
        let dirs = command_dirs(None, Some(Path::new("/home/ann")));

        assert_eq!(dirs, install_dirs_with_home("/home/ann"));
    }

    #[test]
    fn finds_a_command_on_path() {
        let first = TempDir::new();
        let second = TempDir::new();
        write_file(second.path(), "podman", 0o755);
        let dirs =
            vec![first.path().to_path_buf(), second.path().to_path_buf()];

        let found = find_command("podman", &dirs);

        assert_eq!(found, Some(second.path().join("podman")));
    }

    #[test]
    fn finds_docker_only_in_an_install_folder() {
        let path_dir = TempDir::new();
        let home = TempDir::new();
        let docker_bin = home.path().join(".docker/bin");
        std::fs::create_dir_all(&docker_bin).expect("create .docker/bin");
        write_file(&docker_bin, "docker", 0o755);
        let path = path_of(&[path_dir.path()]);
        let dirs = command_dirs(Some(&path), Some(home.path()));

        let found = find_command("docker", &dirs);

        assert!(found.is_some());
        assert!(dirs.contains(&docker_bin));
    }

    #[test]
    fn missing_command_is_not_found() {
        let dir = TempDir::new();
        let dirs =
            vec![dir.path().to_path_buf(), PathBuf::from("/no/such/dir")];

        assert_eq!(find_command("docker", &dirs), None);
        assert_eq!(find_command("docker", &[]), None);
    }

    #[test]
    fn file_without_execute_bit_does_not_count() {
        let dir = TempDir::new();
        write_file(dir.path(), "docker", 0o644);

        let found = find_command("docker", &[dir.path().to_path_buf()]);

        assert_eq!(found, None);
    }

    #[test]
    fn directory_with_the_tool_name_does_not_count() {
        let dir = TempDir::new();
        std::fs::create_dir(dir.path().join("podman")).expect("create dir");

        let found = find_command("podman", &[dir.path().to_path_buf()]);

        assert_eq!(found, None);
    }

    #[test]
    fn docker_version_answer_does_not_name_podman() {
        assert!(!version_names_podman(DOCKER_VERSION));
        assert!(!version_names_podman(""));
    }

    #[test]
    fn podman_docker_version_answer_names_podman() {
        assert!(version_names_podman(PODMAN_DOCKER_VERSION));
        assert!(version_names_podman("Podman version 4.9.4"));
    }

    #[test]
    fn docker_linked_to_the_podman_binary_is_podman() {
        assert!(links_to_podman(Path::new("/usr/bin/podman")));
        assert!(!links_to_podman(Path::new("/usr/bin/docker")));
        assert!(!links_to_podman(Path::new("/")));
    }

    #[test]
    fn real_docker_counts_as_docker() {
        let presence = CliPresence::from_lookup(true, false, false);

        assert_eq!(presence, CliPresence::new(true, false));
    }

    #[test]
    fn docker_that_is_podman_counts_as_podman() {
        assert_eq!(
            CliPresence::from_lookup(true, true, true),
            CliPresence::new(false, true)
        );
        assert_eq!(
            CliPresence::from_lookup(true, true, false),
            CliPresence::new(false, true)
        );
    }

    #[test]
    fn nothing_found_is_no_cli() {
        let presence = CliPresence::from_lookup(false, false, false);

        assert_eq!(presence, CliPresence::default());
    }

    #[tokio::test]
    async fn find_reads_docker_version_answer() {
        let dir = TempDir::new();
        write_script(
            dir.path(),
            "docker",
            &format!("printf '{DOCKER_VERSION}'\n"),
            0o755,
        );
        write_file(dir.path(), "podman", 0o755);

        let presence =
            CliPresence::find(&[dir.path().to_path_buf()], LIMIT).await;

        assert_eq!(presence, CliPresence::new(true, true));
    }

    #[tokio::test]
    async fn find_counts_podman_docker_wrapper_as_podman() {
        let dir = TempDir::new();
        write_script(
            dir.path(),
            "docker",
            &format!(
                "echo 'Emulate Docker CLI using podman.' >&2\nprintf '{PODMAN_DOCKER_VERSION}'\n"
            ),
            0o755,
        );

        let presence =
            CliPresence::find(&[dir.path().to_path_buf()], LIMIT).await;

        assert_eq!(presence, CliPresence::new(false, true));
    }

    #[tokio::test]
    async fn find_counts_docker_link_to_podman_as_podman() {
        let bin = TempDir::new();
        let dir = TempDir::new();
        write_script(
            bin.path(),
            "podman",
            "printf 'docker version 5.8.8\\n'\n",
            0o755,
        );
        std::os::unix::fs::symlink(
            bin.path().join("podman"),
            dir.path().join("docker"),
        )
        .expect("link docker to podman");

        let presence =
            CliPresence::find(&[dir.path().to_path_buf()], LIMIT).await;

        assert_eq!(presence, CliPresence::new(false, true));
    }

    #[tokio::test]
    async fn find_does_not_wait_for_a_hanging_docker() {
        let dir = TempDir::new();
        write_script(dir.path(), "docker", "exec sleep 10\n", 0o755);
        let started = Instant::now();

        let presence = CliPresence::find(
            &[dir.path().to_path_buf()],
            Duration::from_millis(200),
        )
        .await;

        assert_eq!(presence, CliPresence::new(true, false));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn find_keeps_docker_when_its_version_cannot_run() {
        let dir = TempDir::new();
        write_script(dir.path(), "docker", "exit 1\n", 0o755);

        let presence =
            CliPresence::find(&[dir.path().to_path_buf()], LIMIT).await;

        assert_eq!(presence, CliPresence::new(true, false));
    }

    #[tokio::test]
    async fn find_without_commands_finds_nothing() {
        let dir = TempDir::new();

        let presence =
            CliPresence::find(&[dir.path().to_path_buf()], LIMIT).await;

        assert_eq!(presence, CliPresence::default());
    }
}
