//! The version of Codex and whether a profile has sessions, from the
//! session files in `<config>/sessions/` and `<config>/archived_sessions/`.
//! The first line of a session file, its `session_meta` record, names the
//! version of `codex` that started it as `cli_version`.

use std::path::Path;

use super::session_file::read_cli_version;
use super::sessions::{newest_first, session_files};

/// How many of the newest session files are looked at for a version: an
/// older file may have been written by a Codex that named none.
const FILES_LOOKED_AT: usize = 10;

/// Whether a session folder of the profile in `config_dir` holds a session
/// file. Nothing is read from the files.
pub(super) fn has_sessions(config_dir: &Path) -> bool {
    session_files(config_dir).is_ok_and(|files| !files.is_empty())
}

/// The version of `codex` the newest session file of the profile in
/// `config_dir` that names one tells; `None` when none of the newest files
/// does.
pub(super) fn version(config_dir: &Path) -> Option<String> {
    let files = session_files(config_dir).ok()?;

    newest_first(files)
        .into_iter()
        .take(FILES_LOOKED_AT)
        .find_map(|(path, _)| read_cli_version(&path))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::{
        TempDir, at, set_file_time, write_compressed, write_file,
    };

    /// The first line of a session started by `codex` of `version`.
    fn meta(version: &str) -> String {
        format!(
            r#"{{"type":"session_meta","payload":{{"id":"made-up","cwd":"/work","cli_version":"{version}"}}}}"#
        )
    }

    fn session_path(config_dir: &Path, name: &str) -> PathBuf {
        config_dir
            .join("sessions/2026/11/19")
            .join(format!("rollout-2026-11-19T10-00-00-{name}"))
    }

    #[test]
    fn the_version_comes_from_the_newest_session_file() {
        let temp = TempDir::new();
        let old = session_path(temp.path(), "old.jsonl.zst");
        write_compressed(&old, &meta("0.160.0"));
        let new = session_path(temp.path(), "new.jsonl");
        write_file(&new, &meta("0.169.0"));
        set_file_time(&old, at(1_000));
        set_file_time(&new, at(2_000));

        assert_eq!(version(temp.path()), Some(String::from("0.169.0")));
        assert!(has_sessions(temp.path()));
    }

    #[test]
    fn a_newest_file_that_names_no_version_gives_way_to_an_older_one() {
        let temp = TempDir::new();
        let old = session_path(temp.path(), "old.jsonl");
        write_file(&old, &meta("0.160.0"));
        let new = session_path(temp.path(), "new.jsonl");
        write_file(
            &new,
            r#"{"type":"session_meta","payload":{"id":"made-up","cwd":"/work"}}"#,
        );
        set_file_time(&old, at(1_000));
        set_file_time(&new, at(2_000));

        assert_eq!(version(temp.path()), Some(String::from("0.160.0")));
    }

    #[test]
    fn a_profile_with_no_session_has_no_version_and_no_sessions() {
        let temp = TempDir::new();
        write_file(&temp.path().join("sessions/2026/11/19/notes.txt"), "");

        assert_eq!(version(temp.path()), None);
        assert!(!has_sessions(temp.path()));
        assert!(!has_sessions(&temp.path().join("missing")));
    }
}
