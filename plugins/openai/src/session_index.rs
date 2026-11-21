//! Reads `<config>/session_index.jsonl`, where Codex keeps the names the
//! sessions were given.

use std::collections::BTreeMap;

use otty_plugin_sdk::files;
use serde::Deserialize;

use crate::timestamp::millis_of;

/// The name of the index in the config folder.
const INDEX_FILE: &str = "session_index.jsonl";

/// One line of the index.
#[derive(Debug, Deserialize)]
struct IndexLine {
    id: Option<String>,
    thread_name: Option<String>,
    updated_at: Option<String>,
}

/// What the index tells about one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexEntry {
    name: Option<String>,
    updated_ms: Option<i64>,
}

impl IndexEntry {
    /// The name the session was given.
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// When the session last changed, in milliseconds since the Unix
    /// epoch.
    pub(crate) fn updated_ms(&self) -> Option<i64> {
        self.updated_ms
    }
}

/// The path of the index of the profile in `folder`.
pub(crate) fn index_path(folder: &str) -> String {
    format!("{folder}/{INDEX_FILE}")
}

/// Whether the file at `path` is the index of a profile.
pub(crate) fn is_index(path: &str) -> bool {
    path.rsplit('/').next() == Some(INDEX_FILE)
}

/// The entries of the index at `path`, by session id.
///
/// When an id has several lines, the last one wins. A missing file gives
/// no entries, and a broken line is skipped.
pub(crate) fn read_index(path: &str) -> BTreeMap<String, IndexEntry> {
    let mut entries = BTreeMap::new();

    for batch in files::lines(path, &[]) {
        let Ok(batch) = batch else {
            break;
        };
        for line in batch.lines() {
            add_line(&mut entries, line);
        }
    }

    entries
}

/// Takes the entry `line` of the index tells, when it is one.
fn add_line(entries: &mut BTreeMap<String, IndexEntry>, line: &[u8]) {
    let Ok(line) = serde_json::from_slice::<IndexLine>(line) else {
        return;
    };
    let Some(id) = line.id else {
        return;
    };

    let entry = IndexEntry {
        name: line.thread_name,
        updated_ms: line.updated_at.as_deref().and_then(millis_of),
    };
    entries.insert(id, entry);
}

/// Test-only values of the index.
#[cfg(test)]
pub(crate) mod testing {
    use std::collections::BTreeMap;

    use super::{IndexEntry, add_line};

    /// The entries an index of `lines` tells.
    pub(crate) fn index_of(lines: &[&str]) -> BTreeMap<String, IndexEntry> {
        let mut entries = BTreeMap::new();
        for line in lines {
            add_line(&mut entries, line.as_bytes());
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::testing::index_of;
    use super::*;
    use crate::testing::{
        BLOG_SESSION, SHOP_SESSION, TEN, as_part, host, path,
    };

    fn entry(name: Option<&str>, millis: Option<i64>) -> IndexEntry {
        IndexEntry {
            name: name.map(String::from),
            updated_ms: millis,
        }
    }

    #[test]
    fn each_line_names_one_session() {
        let index = index_of(&[
            r#"{"id":"a1","thread_name":"Fix the tabs","updated_at":"2026-11-16T10:00:00Z"}"#,
            r#"{"id":"b2","thread_name":"Add a test","updated_at":"2026-11-16T10:05:00.5Z"}"#,
        ]);

        assert_eq!(index.len(), 2);
        assert_eq!(index["a1"], entry(Some("Fix the tabs"), Some(TEN)));
        assert_eq!(index["b2"].name(), Some("Add a test"));
        assert_eq!(index["b2"].updated_ms(), Some(TEN + 300_500));
    }

    #[test]
    fn the_last_line_of_an_id_wins() {
        let index = index_of(&[
            r#"{"id":"a1","thread_name":"First name","updated_at":"2026-11-16T10:00:00Z"}"#,
            r#"{"id":"a1","thread_name":"Better name","updated_at":"2026-11-16T10:05:00Z"}"#,
        ]);

        assert_eq!(
            index["a1"],
            entry(Some("Better name"), Some(TEN + 300_000))
        );
    }

    #[test]
    fn broken_lines_and_lines_with_no_id_are_skipped() {
        let index = index_of(&[
            "{\"id\":\"a1\",\"thread_name\":",
            "",
            "not json",
            r#"{"thread_name":"No id"}"#,
            r#"{"id":"c3","thread_name":42}"#,
            r#"{"id":"b2","thread_name":"Add a test","updated_at":"yesterday","x":1}"#,
        ]);

        assert_eq!(index.len(), 1);
        assert_eq!(index["b2"], entry(Some("Add a test"), None));
    }

    #[test]
    fn the_index_file_of_a_profile_is_read() {
        let host = host();
        let folder = path(&host, ".codex");

        let index = as_part(&host, || read_index(&index_path(&folder)));

        assert_eq!(index.len(), 2);
        assert_eq!(
            index[SHOP_SESSION],
            entry(Some("Fix the cart total rounding"), Some(TEN + 1_860_000))
        );
        assert_eq!(index[BLOG_SESSION].name(), Some("Write the release notes"));
    }

    #[test]
    fn the_index_is_told_by_its_name() {
        assert!(is_index("/home/dev/.codex/session_index.jsonl"));
        assert!(!is_index("/home/dev/.codex/sessions/rollout-a.jsonl"));
        assert!(!is_index("/home/dev/.codex/old_session_index.jsonl"));
    }

    #[test]
    fn a_missing_index_has_no_entries() {
        let host = host();
        let folder = path(&host, ".codex-work");

        assert!(as_part(&host, || read_index(&index_path(&folder))).is_empty());
    }
}
