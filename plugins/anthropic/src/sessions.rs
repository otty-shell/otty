//! The sessions of a Claude Code profile, read from the session files in
//! `<config>/projects/`.
//!
//! Each folder in `projects/` is one project, and each `*.jsonl` file
//! right inside it is one session. The sessions of helper agents are kept
//! deeper, in `<session id>/subagents/`, so they are not listed.

use otty_plugin_sdk::files::{self, FileInfo};
use otty_plugin_sdk::{HostError, ReadError, Session};

use crate::file_ends::END_BYTES;
use crate::glob::escaped;
use crate::project_folder::{decoded_folder, parent_name, projects_folder};
use crate::session_file::read_session;

/// The sessions of the profile in `folder`, newest first. A missing
/// `projects` folder has none; a session file that can not be read is
/// left out.
pub(crate) fn sessions(folder: &str) -> Result<Vec<Session>, ReadError> {
    let found = session_files(folder).map_err(ReadError::Host)?;

    let mut sessions: Vec<Session> = found
        .iter()
        .filter_map(|file| {
            let fallback = decoded_folder(&parent_name(&file.path));
            read_session(file, &fallback, END_BYTES).ok()
        })
        .collect();
    sessions.sort_by(|one, other| {
        other
            .changed_ms
            .cmp(&one.changed_ms)
            .then_with(|| one.id.cmp(&other.id))
    });

    Ok(sessions)
}

/// The session files of the profile in `folder`: the `*.jsonl` files right
/// inside its project folders.
pub(crate) fn session_files(folder: &str) -> Result<Vec<FileInfo>, HostError> {
    let projects = escaped(&projects_folder(folder));

    files::list(&format!("{projects}/*/*.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    fn sessions_of(folder: &str) -> Result<Vec<Session>, ReadError> {
        let host = host();
        let folder = path(&host, folder);

        as_part(&host, || sessions(&folder))
    }

    #[test]
    fn sessions_are_newest_first_with_their_folders_and_titles() {
        let found = sessions_of(".claude").expect("the sessions");

        let listed: Vec<(&str, &str, &str, i64)> = found
            .iter()
            .map(|session| {
                (
                    session.id.as_str(),
                    session.folder.as_str(),
                    session.title.as_str(),
                    session.changed_ms,
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                (
                    "3b2e4d9a-af6c-4081-9d2e-3c4f5a6b7c8d",
                    "/home/dev/shop",
                    "Run the tests",
                    1_794_826_830_000
                ),
                (
                    "1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b",
                    "/home/dev/shop",
                    "Fix the cart total rounding",
                    1_794_825_600_000
                ),
                (
                    "2a1d3c8f-9e5b-4f70-8c1d-2b3e4f5a6b7c",
                    "/home/dev/blog",
                    "Write the release notes",
                    1_794_733_240_000
                ),
            ]
        );
    }

    /// Helper sessions in `<id>/subagents/`, other files and a session
    /// file outside a project folder are not sessions.
    #[test]
    fn only_session_files_in_project_folders_are_listed() {
        let found = sessions_of(".claude").expect("the sessions");

        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|session| !session.id.starts_with("agent")));
        assert!(found.iter().all(|session| session.id != "stray"));
    }

    #[test]
    fn a_profile_with_no_projects_folder_has_no_sessions() {
        assert_eq!(sessions_of(".claude-token"), Ok(Vec::new()));
        assert_eq!(sessions_of(".claude-gone"), Ok(Vec::new()));
    }
}
