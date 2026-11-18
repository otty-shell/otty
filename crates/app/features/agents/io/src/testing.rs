//! Helpers that the tests of several files share: running a task to its
//! end, writing files, and made-up programs to run.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

use iced::Task;
use iced::futures::StreamExt;
use iced::futures::executor::block_on;
use iced_test::runtime::Action;
use iced_test::runtime::task::into_stream;
#[cfg(unix)]
use otty_agents::AgentCommand;
use otty_app_agents::AgentsIntent;
#[cfg(unix)]
use otty_app_testing::temp_dir::TempDir;

/// Run `task` to its end and return the intents it answers with.
pub(crate) fn run(task: Task<AgentsIntent>) -> Vec<AgentsIntent> {
    let stream = into_stream(task).expect("a task");
    let actions: Vec<Action<AgentsIntent>> = block_on(stream.collect());

    actions
        .into_iter()
        .filter_map(|action| match action {
            Action::Output(intent) => Some(intent),
            _ => None,
        })
        .collect()
}

/// Write `text` to the file at `path`, making its folders.
#[cfg(unix)]
pub(crate) fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent"))
        .expect("the folder is made");
    std::fs::write(path, text).expect("the file is written");
}

/// Write an executable `/bin/sh` script with `body` at `name` in `folder`.
///
/// The file is written by a `/bin/sh` child, not by the test process. A
/// file the test process writes is open for writing for a moment, and
/// another test thread that starts a program then may copy that open file
/// into its child; running the script fails with "Text file busy" until
/// that child has started its program. Here no thread of the test process
/// ever holds the file open for writing, so it can always be run.
#[cfg(unix)]
pub(crate) fn executable_script(
    folder: &Path,
    name: &str,
    body: &str,
) -> PathBuf {
    let path = folder.join(name);
    let text = format!("#!/bin/sh\n{body}\n");

    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg("printf '%s' \"$2\" > \"$1\"")
        .arg("sh")
        .arg(&path)
        .arg(&text)
        .status()
        .expect("the script should be written");
    assert!(status.success(), "the script should be written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the script should be executable");

    path
}

/// A made-up shell in `folder`: it checks that it was started as a login
/// and interactive shell and runs the script with `/bin/sh`, so the user's
/// own start files are never read. Each start adds a line to `starts` in
/// the folder, which [`starts`] counts.
#[cfg(unix)]
pub(crate) fn fake_shell(folder: &TempDir) -> String {
    let starts = folder.path().join("starts");
    let body = format!(
        "echo started >> '{}'\n\
         [ \"$1\" = -l ] && [ \"$2\" = -i ] && [ \"$3\" = -c ] || exit 2\n\
         exec /bin/sh -c \"$4\"",
        starts.display()
    );

    executable_script(folder.path(), "fake-shell", &body)
        .display()
        .to_string()
}

/// How many times the [`fake_shell`] of `folder` was started.
#[cfg(unix)]
pub(crate) fn starts(folder: &TempDir) -> usize {
    std::fs::read_to_string(folder.path().join("starts"))
        .map_or(0, |text| text.lines().count())
}

/// A made-up agent program in `folder` that runs `body`.
#[cfg(unix)]
pub(crate) fn agent(folder: &TempDir, body: &str) -> AgentCommand {
    let path = executable_script(folder.path(), "agent", body);

    AgentCommand::new(path.display().to_string())
}

/// Whether a process with `pid` still exists.
#[cfg(unix)]
pub(crate) fn is_running(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
