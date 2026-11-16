//! Reads `<config>/session_index.jsonl`, where Codex keeps the names the
//! sessions were given.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use serde::Deserialize;

use crate::timestamp::parse_rfc3339;

/// One line of the index.
#[derive(Debug, Deserialize)]
struct IndexLine {
    id: Option<String>,
    thread_name: Option<String>,
    updated_at: Option<String>,
}

/// What the index tells about one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IndexEntry {
    name: Option<String>,
    updated_at: Option<SystemTime>,
}

impl IndexEntry {
    /// The name the session was given.
    pub(super) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// When the session last changed.
    pub(super) fn updated_at(&self) -> Option<SystemTime> {
        self.updated_at
    }
}

/// The entries of the index in `config_dir`, by session id.
///
/// When an id has several lines, the last one wins. A missing file gives
/// no entries, and a broken line is skipped.
pub(super) fn read_index(config_dir: &Path) -> HashMap<String, IndexEntry> {
    let Ok(bytes) = std::fs::read(config_dir.join("session_index.jsonl"))
    else {
        return HashMap::new();
    };
    let text = String::from_utf8_lossy(&bytes);

    let mut entries = HashMap::new();
    for line in text.lines() {
        let Ok(line) = serde_json::from_str::<IndexLine>(line) else {
            continue;
        };
        let Some(id) = line.id else {
            continue;
        };

        let entry = IndexEntry {
            name: line.thread_name,
            updated_at: line.updated_at.as_deref().and_then(parse_rfc3339),
        };
        entries.insert(id, entry);
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, at, write_file};

    fn entry(name: Option<&str>, seconds: Option<u64>) -> IndexEntry {
        IndexEntry {
            name: name.map(String::from),
            updated_at: seconds.map(at),
        }
    }

    fn index_of(lines: &[&str]) -> HashMap<String, IndexEntry> {
        let temp = TempDir::new();
        write_file(&temp.path().join("session_index.jsonl"), &lines.join("\n"));

        read_index(temp.path())
    }

    #[test]
    fn each_line_names_one_session() {
        let index = index_of(&[
            r#"{"id":"a1","thread_name":"Fix the tabs","updated_at":"2026-11-16T10:00:00Z"}"#,
            r#"{"id":"b2","thread_name":"Add a test","updated_at":"2026-11-16T10:05:00.5Z"}"#,
        ]);

        assert_eq!(index.len(), 2);
        assert_eq!(
            index["a1"],
            entry(Some("Fix the tabs"), Some(1_794_823_200))
        );
        assert_eq!(index["b2"].name(), Some("Add a test"));
    }

    #[test]
    fn the_last_line_of_an_id_wins() {
        let index = index_of(&[
            r#"{"id":"a1","thread_name":"First name","updated_at":"2026-11-16T10:00:00Z"}"#,
            r#"{"id":"a1","thread_name":"Better name","updated_at":"2026-11-16T10:05:00Z"}"#,
        ]);

        assert_eq!(
            index["a1"],
            entry(Some("Better name"), Some(1_794_823_500))
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
    fn a_missing_index_has_no_entries() {
        let temp = TempDir::new();

        assert!(read_index(temp.path()).is_empty());
    }
}
