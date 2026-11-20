//! The version of Claude Code a profile last ran, from its newest session
//! file: each record of a session names the version of `claude` that
//! wrote it.

use otty_plugin_sdk::files::FileInfo;

use crate::file_ends::END_BYTES;
use crate::session_file::read_version;
use crate::sessions::session_files;

/// The version of `claude` the newest record of the newest session file
/// of the profile in `folder` names; `None` with no session or no record
/// that names one, and OTTY asks `claude --version` itself.
pub(crate) fn version(folder: &str) -> Option<String> {
    let found = session_files(folder).ok()?;
    let newest = newest(&found)?;

    read_version(newest, END_BYTES)
}

/// The file of `found` that changed last.
fn newest(found: &[FileInfo]) -> Option<&FileInfo> {
    found.iter().max_by_key(|file| file.changed_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    fn file(path: &str, changed_ms: i64) -> FileInfo {
        FileInfo {
            path: path.to_string(),
            size: 10,
            changed_ms,
        }
    }

    #[test]
    fn the_newest_file_is_the_one_that_changed_last() {
        let found = [file("old.jsonl", 1_000), file("new.jsonl", 2_000)];

        assert_eq!(
            newest(&found).map(|file| file.path.as_str()),
            Some("new.jsonl")
        );
        assert_eq!(newest(&[]), None);
    }

    #[test]
    fn the_version_comes_from_the_newest_session_file() {
        let host = host();
        let work = path(&host, ".claude-work");
        let empty = path(&host, ".claude-token");

        let (version, none) =
            as_part(&host, || (version(&work), version(&empty)));

        assert_eq!(version.as_deref(), Some("2.1.328"));
        assert_eq!(none, None);
    }
}
