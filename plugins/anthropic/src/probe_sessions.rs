//! The session files that Claude Code leaves behind when the limits are
//! read from its `/usage` panel, and their removal.
//!
//! Claude Code keeps a session file for every start, in
//! `<config>/projects/<folder name>/<session id>.jsonl`, where the folder
//! name is made from the working folder (see `project_folder.rs`). The
//! panel is read in OTTY's working folder of hidden terminals, so every
//! session file in its project folder was left by a reading. As in
//! CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeProbeSessionArtifactCleaner.swift>,
//! only the `*.jsonl` files directly in that folder are removed, then the
//! folder itself when nothing else is left in it. The part's `write`
//! permission allows nothing else: OTTY refuses any other path.

use otty_plugin_sdk::files;

use crate::glob::escaped;
use crate::project_folder::{project_folder_name, projects_folder};

/// Removes the session files that Claude Code left in the project folder
/// of each of `working_folders`, in the profile in `folder`. Nothing else
/// is ever removed; a file that can not be removed is left as it is.
pub(crate) fn remove_probe_sessions(folder: &str, working_folders: &[String]) {
    for working_folder in working_folders {
        let Some(name) = project_folder_name(working_folder) else {
            continue;
        };
        let project = format!("{}/{name}", projects_folder(folder));

        let found = files::list(&format!("{}/*.jsonl", escaped(&project)))
            .unwrap_or_default();
        for file in found {
            let _ = files::remove(&file.path);
        }

        // OTTY removes the folder only when it is empty.
        let _ = files::remove(&project);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_plugin_sdk::HostError;

    use super::*;
    use crate::testing::{as_provider_in, host, login_folder, path};

    const PROBE_FOLDER: &str = "/home/dev/.cache/otty/agent-probe";
    const PROJECT: &str =
        ".claudeprobe/projects/-home-dev--cache-otty-agent-probe";

    #[test]
    fn only_the_session_files_of_the_probe_folder_are_removed() {
        let host = host();
        let folder = path(&host, ".claudeprobe");
        let folders = [login_folder(&host, ".claudeprobe")];

        as_provider_in(&host, &folders, || {
            remove_probe_sessions(&folder, &[PROBE_FOLDER.to_string()]);
        });

        let mut removed = host.removed();
        removed.sort();
        assert_eq!(
            removed,
            [
                PathBuf::from(path(&host, &format!("{PROJECT}/a1.jsonl"))),
                PathBuf::from(path(&host, &format!("{PROJECT}/b2.jsonl"))),
            ]
        );
    }

    /// A working folder of a real project never matches the `write`
    /// permission, so OTTY refuses to remove its sessions.
    #[test]
    fn the_sessions_of_another_folder_are_never_removed() {
        let host = host();
        let folder = path(&host, ".claudeprobe");
        let other =
            path(&host, ".claudeprobe/projects/-home-dev-shop/c3.jsonl");
        let folders = [login_folder(&host, ".claudeprobe")];

        let refused = as_provider_in(&host, &folders, || {
            remove_probe_sessions(&folder, &["/home/dev/shop".to_string()]);
            files::remove(&other)
        });

        assert_eq!(refused, Err(HostError::NotAllowed(other)));
        assert!(host.removed().is_empty());
    }

    #[test]
    fn a_profile_with_no_probe_folder_is_left_alone() {
        let host = host();
        let folder = path(&host, ".claude-work");
        let folders = [login_folder(&host, ".claude-work")];

        as_provider_in(&host, &folders, || {
            remove_probe_sessions(&folder, &[PROBE_FOLDER.to_string()]);
        });

        assert!(host.removed().is_empty());
    }
}
