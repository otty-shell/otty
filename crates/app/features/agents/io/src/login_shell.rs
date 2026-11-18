//! Questions to the user's login shell with a time limit, such as whether
//! it finds a program or what value a variable has, and the wait for a
//! process with a time limit.
//!
//! The shell is started as a login and interactive shell, the way the
//! agent tab starts it, so it reads the user's start files. Start files
//! that wait for something never hold up a read: the shell is stopped
//! when it takes too long.

use std::io::Read;
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use otty_app_shell::login_shell_args;

/// How long the user's shell may take to answer. A shell whose start files
/// wait for something is stopped after it.
pub(crate) const SHELL_TIMEOUT: Duration = Duration::from_secs(10);
/// How often a running process is asked whether it ended.
const POLL: Duration = Duration::from_millis(20);

/// Run `script` in the user's login shell at `shell` and return what the
/// shell printed. `None` when the shell can not start, fails, or takes
/// longer than `limit`; a shell that takes too long is killed. Only the
/// shell and the script are ever logged, never what the shell printed,
/// which may be a secret.
pub(crate) fn ask_login_shell(
    shell: &str,
    script: &str,
    limit: Duration,
) -> Option<Vec<u8>> {
    let child = Command::new(shell)
        .args(login_shell_args(script))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(err) => {
            log::warn!("cannot run the shell {shell}: {err}");
            return None;
        },
    };

    let output = child.stdout.take().and_then(read_on_thread);
    let started = Instant::now();

    let Some(status) = wait_with_limit(&mut child, limit) else {
        log::warn!("the shell {shell} did not run `{script}` in time");
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    if !status.success() {
        return None;
    }

    let left = limit.saturating_sub(started.elapsed());
    output?.recv_timeout(left).ok()
}

/// Wait for `child` to end, asking every few milliseconds, for at most
/// `limit`. The status it ended with, or `None` when it still runs after
/// `limit` or can not be asked. An ended process is reaped, so it leaves
/// nothing behind; one that still runs is left running.
pub(crate) fn wait_with_limit(
    child: &mut Child,
    limit: Duration,
) -> Option<ExitStatus> {
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if started.elapsed() < limit => std::thread::sleep(POLL),
            Ok(None) => return None,
            Err(err) => {
                log::warn!(
                    "waiting for the process {} failed: {err}",
                    child.id()
                );
                return None;
            },
        }
    }
}

/// Read `output` to its end on a thread of its own, so a shell that prints
/// a lot never waits for the pipe to be read. The text arrives on the
/// returned channel; `None` when the thread can not start.
fn read_on_thread(mut output: ChildStdout) -> Option<Receiver<Vec<u8>>> {
    let (sender, text) = mpsc::channel();

    let thread = std::thread::Builder::new()
        .name(String::from("otty-shell-output"))
        .spawn(move || {
            let mut read = Vec::new();
            let _ = output.read_to_end(&mut read);
            let _ = sender.send(read);
        });
    if let Err(err) = thread {
        log::warn!("no thread to read the shell's output: {err}");
        return None;
    }

    Some(text)
}

#[cfg(all(test, unix))]
mod tests {
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{executable_script, fake_shell};

    #[test]
    fn the_shell_gives_what_the_script_printed() {
        let folder = TempDir::new("login-shell-output");
        let shell = fake_shell(&folder);

        let printed = ask_login_shell(&shell, "printf 'hello'", SHELL_TIMEOUT);

        assert_eq!(printed.as_deref(), Some(b"hello".as_slice()));
    }

    #[test]
    fn a_script_that_fails_gives_nothing() {
        let folder = TempDir::new("login-shell-fails");
        let shell = fake_shell(&folder);

        let printed =
            ask_login_shell(&shell, "printf 'no'; exit 1", SHELL_TIMEOUT);

        assert_eq!(printed, None);
    }

    #[test]
    fn a_shell_that_does_not_answer_in_time_is_stopped() {
        let folder = TempDir::new("login-shell-slow");
        let shell = executable_script(folder.path(), "slow-shell", "sleep 5");
        let shell = shell.display().to_string();

        let started = Instant::now();
        let printed =
            ask_login_shell(&shell, "true", Duration::from_millis(100));

        assert_eq!(printed, None);
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn a_shell_that_can_not_start_gives_nothing() {
        let printed = ask_login_shell("/no/such/shell", "true", SHELL_TIMEOUT);

        assert_eq!(printed, None);
    }
}
