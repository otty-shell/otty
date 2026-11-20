//! The version of Codex a profile last ran, from its session files: the
//! first line of a session file, its `session_meta` record, names the
//! version of `codex` that started it as `cli_version`.

use otty_plugin_sdk::files::FileInfo;

use crate::session_file::read_cli_version;
use crate::sessions::{newest_first, session_files};

/// How many of the newest session files are looked at for a version: an
/// older file may have been written by a Codex that named none.
const FILES_LOOKED_AT: usize = 10;

/// The version of `codex` the newest session file of the profile in
/// `folder` that names one tells; `None` when none of the newest files
/// does, and OTTY asks `codex --version` itself.
pub(crate) fn version(folder: &str) -> Option<String> {
    let found = session_files(folder).ok()?;

    newest_version(found, read_cli_version)
}

/// The version the newest of `found` that names one tells, as
/// `read_version` reads a file.
fn newest_version(
    found: Vec<FileInfo>,
    read_version: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    newest_first(found)
        .iter()
        .take(FILES_LOOKED_AT)
        .find_map(|file| read_version(&file.path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, file_info, host, path};

    /// The version each made-up file names.
    fn named(path: &str) -> Option<String> {
        match path {
            "old.jsonl.zst" => Some(String::from("0.160.0")),
            "new.jsonl" => Some(String::from("0.169.0")),
            _ => None,
        }
    }

    fn version_of(folder: &str) -> Option<String> {
        let host = host();
        let folder = path(&host, folder);

        as_part(&host, || version(&folder))
    }

    #[test]
    fn the_version_comes_from_the_newest_session_file() {
        let found = vec![
            file_info("old.jsonl.zst", 1_000),
            file_info("new.jsonl", 2_000),
        ];

        assert_eq!(newest_version(found, named).as_deref(), Some("0.169.0"));
    }

    #[test]
    fn a_newest_file_that_names_no_version_gives_way_to_an_older_one() {
        let found = vec![
            file_info("old.jsonl.zst", 1_000),
            file_info("unnamed.jsonl", 2_000),
        ];

        assert_eq!(newest_version(found, named).as_deref(), Some("0.160.0"));
    }

    #[test]
    fn the_version_of_a_profile_is_read_from_its_files() {
        assert_eq!(version_of(".codex").as_deref(), Some("0.168.0"));
        assert_eq!(version_of(".codex-work").as_deref(), Some("0.169.0"));
    }

    #[test]
    fn a_profile_with_no_session_or_no_named_version_has_none() {
        assert_eq!(version_of(".codex_old"), None);
        assert_eq!(version_of(".codex-nomode"), None);
        assert_eq!(version_of(".codex-gone"), None);
    }
}
