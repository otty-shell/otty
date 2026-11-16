//! Helpers that the tests of several files share: made-up programs to run.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use otty_app_testing::temp_dir::TempDir;

/// Write an executable `/bin/sh` script with `body` at `name` in `folder`.
///
/// The file is written by a `/bin/sh` child, not by the test process. A
/// file the test process writes is open for writing for a moment, and
/// another test thread that starts a program then may copy that open file
/// into its child; running the script fails with "Text file busy" until
/// that child has started its program. Here no thread of the test process
/// ever holds the file open for writing, so it can always be run.
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
/// the folder.
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
