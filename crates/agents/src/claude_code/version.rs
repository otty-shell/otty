//! The version of Claude Code and whether a profile has sessions, from
//! the session files in `<config>/projects/<folder name>/`. Each record of
//! a session names the version of `claude` that wrote it.

use std::path::{Path, PathBuf};

use super::project_folder::project_dirs;
use super::session_file::read_version;

/// Whether a project folder of the profile in `config_dir` holds a session
/// file. Nothing is read from the files.
pub(super) fn has_sessions(config_dir: &Path) -> bool {
    let Ok(dirs) = project_dirs(config_dir) else {
        return false;
    };

    dirs.iter()
        .any(|project_dir| !session_files_in(project_dir).is_empty())
}

/// The version of `claude` the newest record of the newest session file of
/// the profile in `config_dir` names; `None` with no session or no record
/// that names one.
pub(super) fn version(config_dir: &Path) -> Option<String> {
    let newest = newest_session_file(config_dir)?;

    read_version(&newest)
}

/// The session file of the profile in `config_dir` that changed last.
fn newest_session_file(config_dir: &Path) -> Option<PathBuf> {
    let dirs = project_dirs(config_dir).ok()?;

    dirs.iter()
        .flat_map(|project_dir| session_files_in(project_dir))
        .filter_map(|path| {
            let changed_at = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((changed_at, path))
        })
        .max_by_key(|(changed_at, _)| *changed_at)
        .map(|(_, path)| path)
}

/// The session files right in `project_dir`: the `*.jsonl` files. The
/// sessions of helper agents, kept deeper, are left out.
fn session_files_in(project_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(project_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, at, set_file_time, write_file};

    /// A made-up record of a prompt written by `claude` of `version`.
    fn record(version: &str) -> String {
        format!(
            r#"{{"type":"user","version":"{version}","cwd":"/work","message":{{"content":"Fix it"}}}}"#
        )
    }

    fn write_session(config_dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        let path = config_dir.join("projects/-work").join(name);
        write_file(&path, &lines.join("\n"));
        path
    }

    #[test]
    fn the_version_is_the_last_one_of_the_newest_session() {
        let temp = TempDir::new();
        let old = write_session(temp.path(), "old.jsonl", &[&record("2.0.9")]);
        let new = write_session(
            temp.path(),
            "new.jsonl",
            &[&record("2.1.327"), &record("2.1.328")],
        );
        set_file_time(&old, at(1_000));
        set_file_time(&new, at(2_000));

        assert_eq!(version(temp.path()), Some(String::from("2.1.328")));
    }

    #[test]
    fn records_with_no_version_are_skipped() {
        let temp = TempDir::new();
        write_session(
            temp.path(),
            "a.jsonl",
            &[
                &record("2.1.300"),
                r#"{"type":"ai-title","aiTitle":"Fix it"}"#,
                "not json",
            ],
        );

        assert_eq!(version(temp.path()), Some(String::from("2.1.300")));
    }

    #[test]
    fn a_profile_with_no_session_has_no_version_and_no_sessions() {
        let temp = TempDir::new();
        write_file(&temp.path().join("projects/-work/notes.txt"), "");
        write_file(
            &temp
                .path()
                .join("projects/-work/a1/subagents/agent-1.jsonl"),
            &record("2.1.1"),
        );

        assert_eq!(version(temp.path()), None);
        assert!(!has_sessions(temp.path()));
        assert!(!has_sessions(&temp.path().join("missing")));
    }

    #[test]
    fn a_session_file_in_a_project_folder_is_a_session() {
        let temp = TempDir::new();
        write_session(temp.path(), "a1.jsonl", &[]);

        assert!(has_sessions(temp.path()));
        assert_eq!(version(temp.path()), None);
    }
}
