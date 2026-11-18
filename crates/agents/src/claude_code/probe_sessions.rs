//! The session files that Claude Code leaves behind when the limits are
//! read from its `/usage` panel, and their removal.
//!
//! Claude Code keeps a session file for every start, in
//! `<config>/projects/<folder name>/<session id>.jsonl`, where the folder
//! name is made from the working folder (see `project_folder.rs`). The
//! panel is read in a working folder of OTTY's own, so every session file
//! in its project folder was left by the reading. As in CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeProbeSessionArtifactCleaner.swift>,
//! only the `*.jsonl` files directly in that folder are removed, then the
//! folder itself when nothing else is left in it.

use std::path::Path;

use super::project_folder::project_folder_name;

/// Removes the session files that Claude Code left in the project folder
/// of `working_dir`, in the config folder `config_dir`. Nothing else is
/// ever removed; a file that can not be removed is left as it is.
pub(super) fn remove_probe_sessions(config_dir: &Path, working_dir: &Path) {
    let Some(name) = project_folder_name(working_dir) else {
        return;
    };
    let project_dir = config_dir.join("projects").join(name);

    let Ok(entries) = std::fs::read_dir(&project_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_session_file =
            path.extension().is_some_and(|ext| ext == "jsonl")
                && entry.file_type().is_ok_and(|kind| kind.is_file());
        if is_session_file {
            let _ = std::fs::remove_file(&path);
        }
    }

    // Removes the folder only when it is empty.
    let _ = std::fs::remove_dir(&project_dir);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::{TempDir, write_file};

    const WORKING_DIR: &str = "/home/user/.cache/otty/agent-probe";
    const FOLDER_NAME: &str = "-home-user--cache-otty-agent-probe";

    fn project_dir(config_dir: &Path) -> PathBuf {
        config_dir.join("projects").join(FOLDER_NAME)
    }

    #[test]
    fn the_session_files_of_the_probe_folder_are_removed() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-test");
        let probe = project_dir(&config_dir);
        write_file(&probe.join("a1.jsonl"), "{}");
        write_file(&probe.join("b2.jsonl"), "{}");

        remove_probe_sessions(&config_dir, Path::new(WORKING_DIR));

        assert!(!probe.join("a1.jsonl").exists());
        assert!(!probe.join("b2.jsonl").exists());
        assert!(!probe.exists(), "the empty folder is removed too");
        assert!(config_dir.join("projects").is_dir());
    }

    #[test]
    fn nothing_but_the_session_files_of_the_probe_folder_is_touched() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-test");
        let probe = project_dir(&config_dir);
        let real_project = config_dir.join("projects").join("-work-otty");
        write_file(&probe.join("a1.jsonl"), "{}");
        write_file(&probe.join("notes.txt"), "kept");
        write_file(&probe.join("a1").join("subagents").join("s.jsonl"), "{}");
        write_file(&real_project.join("c3.jsonl"), "{}");
        write_file(&config_dir.join("history.jsonl"), "{}");

        remove_probe_sessions(&config_dir, Path::new(WORKING_DIR));

        assert!(!probe.join("a1.jsonl").exists());
        assert!(probe.join("notes.txt").is_file());
        assert!(probe.join("a1").join("subagents").join("s.jsonl").is_file());
        assert!(real_project.join("c3.jsonl").is_file());
        assert!(config_dir.join("history.jsonl").is_file());
    }

    #[test]
    fn a_profile_with_no_probe_folder_is_left_alone() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-test");
        let real_project = config_dir.join("projects").join("-work-otty");
        write_file(&real_project.join("c3.jsonl"), "{}");

        remove_probe_sessions(&config_dir, Path::new(WORKING_DIR));
        remove_probe_sessions(
            &temp.path().join("none"),
            Path::new(WORKING_DIR),
        );

        assert!(real_project.join("c3.jsonl").is_file());
    }
}
