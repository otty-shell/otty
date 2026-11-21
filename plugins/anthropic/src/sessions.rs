//! The sessions of a Claude Code profile, read from the session files in
//! `<config>/projects/`.
//!
//! Each folder in `projects/` is one project, and each `*.jsonl` file
//! right inside it is one session. The sessions of helper agents are kept
//! deeper, in `<session id>/subagents/`, so they are not listed.

use otty_plugin_sdk::HostError;
use otty_plugin_sdk::agents::{Place, PlaceItem, PlaceKind, Session};
use otty_plugin_sdk::files::{self, FileInfo};

use crate::file_ends::END_BYTES;
use crate::glob::escaped;
use crate::project_folder::{decoded_folder, parent_name, projects_folder};
use crate::session_file::read_session;

/// Where the sessions of the profile in `folder` are: its session files.
pub(crate) fn session_places(folder: &str) -> Vec<Place> {
    vec![Place {
        glob: session_files_glob(folder),
        kind: PlaceKind::Files,
    }]
}

/// The session the file of `item` keeps; none when the file can not be
/// read.
pub(crate) fn read_sessions(item: &PlaceItem) -> Vec<Session> {
    let fallback = decoded_folder(&parent_name(&item.file.path));

    read_session(&item.file, &fallback, END_BYTES)
        .into_iter()
        .collect()
}

/// The session files of the profile in `folder`: the `*.jsonl` files right
/// inside its project folders.
pub(crate) fn session_files(folder: &str) -> Result<Vec<FileInfo>, HostError> {
    files::list(&session_files_glob(folder))
}

/// The glob of the session files of the profile in `folder`.
fn session_files_glob(folder: &str) -> String {
    let projects = escaped(&projects_folder(folder));

    format!("{projects}/*/*.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{AGENT, as_part, host, path};

    /// What every session file of the profile in `folder` tells, as OTTY
    /// asks, newest first.
    fn sessions_of(folder: &str) -> Vec<Session> {
        let host = host();
        let items =
            host.place_items(AGENT, &session_places(&path(&host, folder)));

        let mut sessions: Vec<Session> = items
            .iter()
            .flat_map(|item| as_part(&host, || read_sessions(item)))
            .collect();
        sessions.sort_by_key(|session| std::cmp::Reverse(session.changed_ms));
        sessions
    }

    #[test]
    fn each_session_file_tells_its_session_with_its_folder_and_title() {
        let found = sessions_of(".claude");

        let listed: Vec<(&str, Option<&str>, Option<&str>, i64)> = found
            .iter()
            .map(|session| {
                (
                    session.id.as_str(),
                    session.folder.as_deref(),
                    session.title.as_deref(),
                    session.changed_ms,
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                (
                    "3b2e4d9a-af6c-4081-9d2e-3c4f5a6b7c8d",
                    Some("/home/dev/shop"),
                    Some("Run the tests"),
                    1_794_826_830_000
                ),
                (
                    "1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b",
                    Some("/home/dev/shop"),
                    Some("Fix the cart total rounding"),
                    1_794_825_600_000
                ),
                (
                    "2a1d3c8f-9e5b-4f70-8c1d-2b3e4f5a6b7c",
                    Some("/home/dev/blog"),
                    Some("Write the release notes"),
                    1_794_733_240_000
                ),
            ]
        );
    }

    /// Helper sessions in `<id>/subagents/`, other files and a session
    /// file outside a project folder are not session files.
    #[test]
    fn only_session_files_in_project_folders_are_read() {
        let found = sessions_of(".claude");

        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|session| !session.id.starts_with("agent")));
        assert!(found.iter().all(|session| session.id != "stray"));
    }

    #[test]
    fn a_profile_with_no_projects_folder_has_no_sessions() {
        assert_eq!(sessions_of(".claude-token"), Vec::new());
        assert_eq!(sessions_of(".claude-gone"), Vec::new());
    }
}
