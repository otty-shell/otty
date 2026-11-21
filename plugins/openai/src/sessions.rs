//! The sessions of a Codex profile, read from the session files in
//! `<config>/sessions/` and `<config>/archived_sessions/`.
//!
//! Each `rollout-*.jsonl` or `rollout-*.jsonl.zst` file in the two session
//! folders, at any depth, is one session. Only its first line is read.
//! The names come from `session_index.jsonl`.

use std::collections::BTreeMap;

use otty_plugin_sdk::HostError;
use otty_plugin_sdk::agents::{ReadError, Session};
use otty_plugin_sdk::files::{self, FileInfo};

use crate::glob::escaped;
use crate::session_file::{SessionMeta, read_session_meta};
use crate::session_index::{IndexEntry, read_index};

/// The folders of the config folder that hold session files.
const SESSION_FOLDERS: [&str; 2] = ["sessions", "archived_sessions"];

/// The endings of a session file's name: plain, or compressed with zstd.
const SESSION_FILE_ENDINGS: [&str; 2] = [".jsonl", ".jsonl.zst"];

/// The longest title, in characters, before it is cut.
const MAX_TITLE_CHARS: usize = 100;

/// The sessions of the profile in `folder`, newest first. A missing
/// session folder has none; a file whose first line can not be read is
/// left out.
pub(crate) fn sessions(folder: &str) -> Result<Vec<Session>, ReadError> {
    let found = session_files(folder).map_err(ReadError::Host)?;
    let index = read_index(folder);

    let read: Vec<(FileInfo, SessionMeta)> = found
        .into_iter()
        .filter_map(|file| {
            let meta = read_session_meta(&file.path)?;
            Some((file, meta))
        })
        .collect();

    Ok(listed(&read, &index))
}

/// The session files of the profile in `folder`, in both session folders
/// and at any depth.
pub(crate) fn session_files(folder: &str) -> Result<Vec<FileInfo>, HostError> {
    files_named(folder, "*")
}

/// The session files of the profile in `folder` that can hold the session
/// `session`: Codex ends the name of a session file with the session's id.
pub(crate) fn files_of_session(
    folder: &str,
    session: &str,
) -> Result<Vec<FileInfo>, HostError> {
    files_named(folder, &format!("*{}", escaped(session)))
}

/// `found` with the file that changed last first. Of two files that
/// changed at the same time, the one with the later path comes first.
pub(crate) fn newest_first(mut found: Vec<FileInfo>) -> Vec<FileInfo> {
    found.sort_by(|one, other| {
        other
            .changed_ms
            .cmp(&one.changed_ms)
            .then_with(|| other.path.cmp(&one.path))
    });

    found
}

/// The globs of the session files named `rollout-<stem>.jsonl` or
/// `rollout-<stem>.jsonl.zst` in both session folders of the config
/// folders `folders`, at any depth. `folders` and `stem` are glob
/// patterns.
pub(crate) fn session_file_globs(folders: &str, stem: &str) -> Vec<String> {
    let mut globs = Vec::new();

    for session_folder in SESSION_FOLDERS {
        for ending in SESSION_FILE_ENDINGS {
            globs.push(format!(
                "{folders}/{session_folder}/**/rollout-{stem}{ending}"
            ));
        }
    }

    globs
}

/// The session files of the profile in `folder` named
/// `rollout-<stem>.jsonl` or `rollout-<stem>.jsonl.zst`, where `stem` is
/// a glob pattern.
fn files_named(folder: &str, stem: &str) -> Result<Vec<FileInfo>, HostError> {
    let mut found = Vec::new();

    for glob in session_file_globs(&escaped(folder), stem) {
        found.extend(files::list(&glob)?);
    }

    Ok(found)
}

/// The sessions the files of `read` tell about, one for each id, newest
/// first, named from `index`.
///
/// The same session can be found twice, in both session folders, or as a
/// plain and a compressed file while Codex compresses it: the file that
/// changed last tells it.
fn listed(
    read: &[(FileInfo, SessionMeta)],
    index: &BTreeMap<String, IndexEntry>,
) -> Vec<Session> {
    let mut by_id: BTreeMap<&str, Session> = BTreeMap::new();
    for (file, meta) in read {
        let session = session(file, meta, index);
        let is_newer = by_id
            .get(meta.id())
            .is_none_or(|kept| session.changed_ms > kept.changed_ms);
        if is_newer {
            by_id.insert(meta.id(), session);
        }
    }

    let mut sessions: Vec<Session> = by_id.into_values().collect();
    sessions.sort_by(|one, other| {
        other
            .changed_ms
            .cmp(&one.changed_ms)
            .then_with(|| one.id.cmp(&other.id))
    });

    sessions
}

/// The session the first line `meta` of `file` tells about, named from
/// `index`.
///
/// The title is the name in the index, else the id. The time is the newer
/// of the time in the index and the time the file last changed.
fn session(
    file: &FileInfo,
    meta: &SessionMeta,
    index: &BTreeMap<String, IndexEntry>,
) -> Session {
    let entry = index.get(meta.id());

    let title = entry
        .and_then(IndexEntry::name)
        .and_then(title_line)
        .unwrap_or_else(|| meta.id().to_string());
    let changed_ms = entry
        .and_then(IndexEntry::updated_ms)
        .map_or(file.changed_ms, |updated| updated.max(file.changed_ms));

    Session {
        id: meta.id().to_string(),
        folder: meta.folder().to_string(),
        title,
        changed_ms,
    }
}

/// The first line of `text` that is not blank, trimmed and cut to
/// [`MAX_TITLE_CHARS`] characters. `None` when `text` is blank.
fn title_line(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;

    if line.chars().count() <= MAX_TITLE_CHARS {
        return Some(line.to_string());
    }

    let cut: String = line.chars().take(MAX_TITLE_CHARS).collect();
    Some(format!("{cut}..."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_index::testing::index_of;
    use crate::testing::{
        BLOG_SESSION, SHOP_SESSION, TEN, as_part, file_info, host, path,
    };

    const ID: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";
    const OTHER_ID: &str = "019a9000-1111-7222-8333-444455556666";
    /// The time the made-up files last changed.
    const FILE_TIME: i64 = 1_794_000_000_000;

    fn meta(id: &str, folder: &str) -> SessionMeta {
        SessionMeta::new(id, folder)
    }

    fn plain_session(id: &str) -> Session {
        Session {
            id: id.to_string(),
            folder: "/home/user/otty".to_string(),
            title: id.to_string(),
            changed_ms: FILE_TIME,
        }
    }

    fn sorted(mut sessions: Vec<Session>) -> Vec<Session> {
        sessions.sort_by(|one, other| one.id.cmp(&other.id));
        sessions
    }

    fn ids(sessions: &[Session]) -> Vec<&str> {
        sessions.iter().map(|session| session.id.as_str()).collect()
    }

    fn sessions_of(folder: &str) -> Result<Vec<Session>, ReadError> {
        let host = host();
        let folder = path(&host, folder);

        as_part(&host, || sessions(&folder))
    }

    #[test]
    fn a_plain_session_has_its_folder_and_file_time() {
        let read =
            [(file_info("a.jsonl", FILE_TIME), meta(ID, "/home/user/otty"))];

        assert_eq!(listed(&read, &BTreeMap::new()), [plain_session(ID)]);
    }

    #[test]
    fn a_compressed_and_an_archived_session_are_listed() {
        let found = sorted(sessions_of(".codex").expect("the sessions"));

        assert_eq!(ids(&found), [SHOP_SESSION, BLOG_SESSION]);
        assert_eq!(found[1].folder, "/home/dev/blog");
        assert_eq!(found[1].title, "Write the release notes");

        let old = sessions_of(".codex_old").expect("the sessions");
        assert_eq!(ids(&old), ["019a0000-4444-7555-8666-777788889999"]);
        assert_eq!(old[0].folder, "/home/dev/shop");
    }

    /// The helper agent's session, the file with a broken first line, the
    /// broken compressed file, `notes.txt` and `history.jsonl` are not
    /// sessions.
    #[test]
    fn helper_sessions_broken_files_and_other_files_are_not_listed() {
        let found = sessions_of(".codex").expect("the sessions");

        assert_eq!(found.len(), 2);
        assert_eq!(
            found
                .iter()
                .find(|session| session.id == SHOP_SESSION)
                .map(|session| session.title.as_str()),
            Some("Fix the cart total rounding")
        );
    }

    #[test]
    fn the_name_and_the_time_come_from_the_index() {
        let read = [
            (file_info("a.jsonl", FILE_TIME), meta(ID, "/a")),
            (file_info("b.jsonl", FILE_TIME), meta(OTHER_ID, "/b")),
        ];
        let index = index_of(&[
            &format!(
                r#"{{"id":"{ID}","thread_name":"Fix the tabs","updated_at":"2026-11-16T10:00:00Z"}}"#
            ),
            &format!(
                r#"{{"id":"{ID}","thread_name":"Stop the tab flicker","updated_at":"2026-11-16T10:05:00Z"}}"#
            ),
            &format!(
                r#"{{"id":"{OTHER_ID}","thread_name":"  ","updated_at":"2026-11-16T10:00:00Z"}}"#
            ),
        ]);

        let sessions = sorted(listed(&read, &index));

        assert_eq!(sessions[0].title, "Stop the tab flicker");
        assert_eq!(sessions[0].changed_ms, TEN + 300_000);
        assert_eq!(sessions[1].title, OTHER_ID);
        assert_eq!(sessions[1].changed_ms, TEN);
    }

    #[test]
    fn the_file_time_wins_when_it_is_newer_than_the_index() {
        let read = [(file_info("a.jsonl", FILE_TIME), meta(ID, "/a"))];
        let index = index_of(&[&format!(
            r#"{{"id":"{ID}","thread_name":"Fix the tabs","updated_at":"2026-01-01T00:00:00Z"}}"#
        )]);

        let sessions = listed(&read, &index);

        assert_eq!(sessions[0].title, "Fix the tabs");
        assert_eq!(sessions[0].changed_ms, FILE_TIME);
    }

    #[test]
    fn a_session_found_twice_is_listed_once_with_its_newest_file() {
        let read = [
            (file_info("sessions/a.jsonl", FILE_TIME), meta(ID, "/a")),
            (file_info("sessions/a.jsonl.zst", FILE_TIME), meta(ID, "/a")),
            (
                file_info("archived_sessions/a.jsonl.zst", FILE_TIME + 60_000),
                meta(ID, "/b"),
            ),
            (
                file_info("sessions/c.jsonl", FILE_TIME),
                meta(OTHER_ID, "/c"),
            ),
        ];

        let sessions = sorted(listed(&read, &BTreeMap::new()));

        assert_eq!(ids(&sessions), [ID, OTHER_ID]);
        assert_eq!(sessions[0].folder, "/b");
        assert_eq!(sessions[0].changed_ms, FILE_TIME + 60_000);
    }

    #[test]
    fn sessions_are_newest_first() {
        let read = [
            (file_info("a.jsonl", FILE_TIME), meta(ID, "/a")),
            (file_info("b.jsonl", FILE_TIME + 1), meta(OTHER_ID, "/b")),
        ];

        assert_eq!(ids(&listed(&read, &BTreeMap::new())), [OTHER_ID, ID]);
    }

    #[test]
    fn a_missing_sessions_folder_gives_an_empty_list() {
        assert_eq!(sessions_of(".codex-nomode"), Ok(Vec::new()));
        assert_eq!(sessions_of(".codex-gone"), Ok(Vec::new()));
    }

    #[test]
    fn the_files_of_a_session_are_found_by_its_id() {
        let host = host();
        let folder = path(&host, ".codex");

        let found = as_part(&host, || files_of_session(&folder, BLOG_SESSION))
            .expect("the listing");

        let mut names: Vec<String> = found
            .iter()
            .map(|file| file.path.replace(&folder, ""))
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                format!(
                    "/archived_sessions/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl"
                ),
                format!(
                    "/sessions/2026/11/15/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl.zst"
                ),
            ]
        );
    }

    #[test]
    fn the_newest_file_comes_first_and_a_tie_goes_to_the_later_path() {
        let found = vec![
            file_info("a", 1_000),
            file_info("c", 2_000),
            file_info("b", 2_000),
        ];

        let paths: Vec<String> = newest_first(found)
            .into_iter()
            .map(|file| file.path)
            .collect();

        assert_eq!(paths, ["c", "b", "a"]);
    }

    #[test]
    fn the_title_is_the_first_line_that_is_not_blank_cut_to_a_length() {
        assert_eq!(
            title_line("\n\n   Rename the panel   \nand its tests").as_deref(),
            Some("Rename the panel")
        );
        assert_eq!(
            title_line(&"a".repeat(300)),
            Some(format!("{}...", "a".repeat(MAX_TITLE_CHARS)))
        );
        assert_eq!(title_line("   "), None);
    }
}
