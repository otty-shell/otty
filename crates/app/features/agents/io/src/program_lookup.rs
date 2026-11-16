//! Whether an agent's program can be started, found the way the agent tab
//! starts it: through the user's login shell.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use otty_app_shell::{login_shell_args, quote_word};

/// How long the user's shell may take to say whether it finds a program.
/// A shell whose start files wait for something is stopped after it.
const SHELL_TIMEOUT: Duration = Duration::from_secs(10);
/// How often a running shell is asked whether it ended.
const SHELL_POLL: Duration = Duration::from_millis(20);

/// Finds programs for one read of the profiles, and remembers what it
/// found, so the user's shell starts once per program.
pub(crate) struct ProgramLookup<'a> {
    shell: &'a str,
    timeout: Duration,
    found: RefCell<HashMap<String, bool>>,
}

impl<'a> ProgramLookup<'a> {
    /// A lookup through the user's shell at `shell`, such as `/bin/zsh`.
    pub(crate) fn new(shell: &'a str) -> Self {
        Self::with_timeout(shell, SHELL_TIMEOUT)
    }

    /// Whether `program` can be started: a path is an executable file, a
    /// name is found by the user's login shell.
    pub(crate) fn is_found(&self, program: &str) -> bool {
        if program.contains(std::path::MAIN_SEPARATOR) {
            return is_executable_file(Path::new(program));
        }
        if let Some(found) = self.found.borrow().get(program) {
            return *found;
        }

        let found = self.is_found_by_shell(program);

        self.found.borrow_mut().insert(program.to_string(), found);
        found
    }

    fn with_timeout(shell: &'a str, timeout: Duration) -> Self {
        Self {
            shell,
            timeout,
            found: RefCell::new(HashMap::new()),
        }
    }

    /// Ask the user's login shell whether it finds `program`. A shell that
    /// can not start, fails or takes too long finds nothing.
    fn is_found_by_shell(&self, program: &str) -> bool {
        let script = format!("command -v {}", quote_word(program));
        let child = Command::new(self.shell)
            .args(login_shell_args(&script))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(err) => {
                log::warn!("cannot run the shell {}: {err}", self.shell);
                return false;
            },
        };

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.success(),
                Ok(None) if started.elapsed() < self.timeout => {
                    std::thread::sleep(SHELL_POLL);
                },
                Ok(None) => {
                    log::warn!(
                        "the shell {} took too long to find {program}",
                        self.shell
                    );
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                },
                Err(err) => {
                    log::warn!(
                        "waiting for the shell {} failed: {err}",
                        self.shell
                    );
                    return false;
                },
            }
        }
    }
}

/// Whether `path` is a file the user may run.
#[cfg(unix)]
pub(crate) fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path).is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    })
}

/// Whether `path` is a file the user may run.
#[cfg(not(unix))]
pub(crate) fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;

    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{executable_script, fake_shell};

    /// Write an executable script with `body` at `name` in `folder`.
    fn script(folder: &TempDir, name: &str, body: &str) -> PathBuf {
        executable_script(folder.path(), name, body)
    }

    fn starts(folder: &TempDir) -> usize {
        std::fs::read_to_string(folder.path().join("starts"))
            .map_or(0, |text| text.lines().count())
    }

    #[test]
    fn a_name_is_found_by_the_login_shell() {
        let folder = TempDir::new("lookup-name");
        let shell = fake_shell(&folder);
        let lookup = ProgramLookup::new(&shell);

        assert!(lookup.is_found("sh"));
        assert!(!lookup.is_found("otty-no-such-agent-program"));
    }

    #[test]
    fn each_name_is_asked_once_per_read() {
        let folder = TempDir::new("lookup-once");
        let shell = fake_shell(&folder);
        let lookup = ProgramLookup::new(&shell);

        assert!(lookup.is_found("sh"));
        assert!(lookup.is_found("sh"));

        assert_eq!(starts(&folder), 1);
    }

    #[test]
    fn a_path_is_found_when_it_is_an_executable_file() {
        let folder = TempDir::new("lookup-path");
        let shell = fake_shell(&folder);
        let program = script(&folder, "agent", "exit 0");
        let plain = folder.path().join("notes.txt");
        std::fs::write(&plain, "not a program").expect("the file is written");
        let lookup = ProgramLookup::new(&shell);

        assert!(lookup.is_found(&program.display().to_string()));
        assert!(!lookup.is_found(&plain.display().to_string()));
        assert!(
            !lookup.is_found(&folder.path().join("gone").display().to_string())
        );
        assert!(!lookup.is_found(&folder.path().display().to_string()));
        assert_eq!(starts(&folder), 0, "a path needs no shell");
    }

    #[test]
    fn a_shell_that_does_not_answer_in_time_finds_nothing() {
        let folder = TempDir::new("lookup-slow");
        let shell = script(&folder, "slow-shell", "sleep 5");
        let shell = shell.display().to_string();
        let lookup =
            ProgramLookup::with_timeout(&shell, Duration::from_millis(100));

        let started = Instant::now();
        let found = lookup.is_found("sh");

        assert!(!found);
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn a_shell_that_can_not_start_finds_nothing() {
        let lookup = ProgramLookup::new("/no/such/shell");

        assert!(!lookup.is_found("sh"));
    }
}
