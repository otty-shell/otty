//! The sessions of a Codex profile, file by file: each `rollout-*.jsonl`
//! or `rollout-*.jsonl.zst` file in `<config>/sessions/` and
//! `<config>/archived_sessions/`, at any depth, is one session, and only
//! its first line is read. The names Codex gave the sessions are in
//! `session_index.jsonl`. OTTY joins what the files tell about one id, so
//! a session found twice, in both session folders or as a plain and a
//! compressed file while Codex compresses it, is listed once.

use std::collections::BTreeMap;

use otty_plugin_sdk::HostError;
use otty_plugin_sdk::agents::{Place, PlaceItem, PlaceKind, Session};
use otty_plugin_sdk::files::{self, FileInfo};

use crate::glob::escaped;
use crate::session_file::read_session_meta;
use crate::session_index::{IndexEntry, index_path, is_index, read_index};

/// The folders of the config folder that hold session files.
const SESSION_FOLDERS: [&str; 2] = ["sessions", "archived_sessions"];

/// The endings of a session file's name: plain, or compressed with zstd.
const SESSION_FILE_ENDINGS: [&str; 2] = [".jsonl", ".jsonl.zst"];

/// The longest title, in characters, before it is cut.
const MAX_TITLE_CHARS: usize = 100;

/// Where the sessions of the profile in `folder` are: every session file
/// and the index of their names.
pub(crate) fn session_places(folder: &str) -> Vec<Place> {
    let folder = escaped(folder);
    let mut globs = session_file_globs(&folder, "*");
    globs.push(index_path(&folder));

    globs
        .into_iter()
        .map(|glob| Place {
            glob,
            kind: PlaceKind::Files,
        })
        .collect()
}

/// What the file of `item` tells about its sessions: the index, the name
/// and the time of each session it names; a session file, the id and the
/// folder of its session, at the time the file last changed. A file whose
/// first line can not be read, or the file of a helper agent's session,
/// tells none.
pub(crate) fn read_sessions(item: &PlaceItem) -> Vec<Session> {
    if is_index(&item.file.path) {
        return index_sessions(&read_index(&item.file.path));
    }

    let Some(meta) = read_session_meta(&item.file.path) else {
        return Vec::new();
    };

    vec![Session {
        id: meta.id().to_string(),
        folder: Some(meta.folder().to_string()),
        title: None,
        changed_ms: item.file.changed_ms,
    }]
}

/// The session files of the profile in `folder`, in both session folders
/// and at any depth.
pub(crate) fn session_files(folder: &str) -> Result<Vec<FileInfo>, HostError> {
    files_named(folder, "*")
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

/// The sessions the index `entries` names: each with its name and the
/// time it was last updated, but no folder; a session with no time there
/// has the time 0, the start of 1970, so the time of its file wins.
fn index_sessions(entries: &BTreeMap<String, IndexEntry>) -> Vec<Session> {
    entries
        .iter()
        .map(|(id, entry)| Session {
            id: id.clone(),
            folder: None,
            title: entry.name().and_then(title_line),
            changed_ms: entry.updated_ms().unwrap_or(0),
        })
        .collect()
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
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::session_index::testing::index_of;
    use crate::testing::{
        AGENT, BLOG_SESSION, SHOP_SESSION, TEN, as_part, file_info, host, path,
    };

    /// The items OTTY asks to read in the config folder `folder` of `host`.
    fn items_of(host: &TestHost, folder: &str) -> Vec<PlaceItem> {
        let places = session_places(&path(host, folder));

        host.place_items(AGENT, &places)
    }

    /// What every file of the session places of `folder` tells, file by
    /// file, as OTTY asks.
    fn read_all(folder: &str) -> Vec<(PlaceItem, Vec<Session>)> {
        let host = host();

        items_of(&host, folder)
            .into_iter()
            .map(|item| {
                let sessions = as_part(&host, || read_sessions(&item));
                (item, sessions)
            })
            .collect()
    }

    /// The `(id, folder)` of what the session files of `folder` tell.
    fn file_sessions(folder: &str) -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = read_all(folder)
            .into_iter()
            .flat_map(|(_, sessions)| sessions)
            .filter_map(|session| Some((session.id, session.folder?)))
            .collect();
        found.sort();
        found
    }

    #[test]
    fn a_session_file_tells_its_id_and_folder_at_the_time_it_changed() {
        let read = read_all(".codex");

        let (item, sessions) = read
            .iter()
            .find(|(item, sessions)| {
                item.file.path.ends_with(".jsonl.zst") && !sessions.is_empty()
            })
            .expect("a compressed session file");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, None);
        assert_eq!(sessions[0].changed_ms, item.file.changed_ms);
    }

    /// The blog session is kept compressed in `sessions/` and as a plain
    /// copy in `archived_sessions/`: both tell it, and OTTY lists it once.
    #[test]
    fn a_compressed_and_an_archived_session_are_told() {
        let blog = || (BLOG_SESSION.to_string(), "/home/dev/blog".to_string());
        let shop = (SHOP_SESSION.to_string(), "/home/dev/shop".to_string());

        assert_eq!(file_sessions(".codex"), [shop, blog(), blog()]);
        assert_eq!(
            file_sessions(".codex_old"),
            [(
                "019a0000-4444-7555-8666-777788889999".to_string(),
                "/home/dev/shop".to_string()
            )]
        );
    }

    /// The helper agent's session, the file with a broken first line and
    /// the broken compressed file tell no session; `notes.txt` and
    /// `history.jsonl` are not read at all.
    #[test]
    fn helper_sessions_broken_files_and_other_files_tell_no_session() {
        let read = read_all(".codex");

        let paths: Vec<&str> = read
            .iter()
            .map(|(item, _)| item.file.path.as_str())
            .collect();
        assert!(paths.iter().all(|path| !path.ends_with("notes.txt")));
        assert!(paths.iter().all(|path| !path.ends_with("history.jsonl")));
        assert_eq!(file_sessions(".codex").len(), 3);
    }

    #[test]
    fn the_index_tells_the_names_and_the_times_but_no_folders() {
        let read = read_all(".codex");

        let (_, index) = read
            .iter()
            .find(|(item, _)| is_index(&item.file.path))
            .expect("the index is read");
        let shop = index
            .iter()
            .find(|session| session.id == SHOP_SESSION)
            .expect("the shop session");
        assert_eq!(index.len(), 2);
        assert!(index.iter().all(|session| session.folder.is_none()));
        assert_eq!(shop.title.as_deref(), Some("Fix the cart total rounding"));
        assert_eq!(shop.changed_ms, TEN + 1_860_000);
    }

    #[test]
    fn a_blank_name_is_no_title_and_no_time_is_the_earliest() {
        let index = index_of(&[
            r#"{"id":"a1","thread_name":"  ","updated_at":"2026-11-16T10:00:00Z"}"#,
            r#"{"id":"b2","thread_name":"Fix the tabs"}"#,
        ]);

        let sessions = index_sessions(&index);

        assert_eq!(
            sessions,
            [
                Session {
                    id: "a1".to_string(),
                    folder: None,
                    title: None,
                    changed_ms: TEN,
                },
                Session {
                    id: "b2".to_string(),
                    folder: None,
                    title: Some("Fix the tabs".to_string()),
                    changed_ms: 0,
                },
            ]
        );
    }

    #[test]
    fn a_missing_sessions_folder_has_no_session_files() {
        let host = host();

        let nomode = items_of(&host, ".codex-nomode");
        let gone = items_of(&host, ".codex-gone");

        assert!(nomode.iter().all(|item| is_index(&item.file.path)));
        assert!(gone.is_empty());
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
