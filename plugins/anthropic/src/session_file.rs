//! One Claude Code session file as a session of the list: its folder,
//! title and time, read from the start and the end of the file. Reading
//! is tolerant: a line that is broken or unknown is skipped.

use std::path::Path;

use otty_plugin_sdk::HostError;
use otty_plugin_sdk::agents::Session;
use otty_plugin_sdk::files::FileInfo;
use serde::Deserialize;
use serde_json::Value;

use crate::file_ends::read_ends;
use crate::timestamp::millis_of;

/// The longest title, in characters, before it is cut.
const MAX_TITLE_CHARS: usize = 100;

/// What marks a record that names the version of `claude` that wrote it.
const VERSION_MARKER: &[u8] = b"\"version\"";

/// One line of a session file: only the fields a session list needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(rename = "type")]
    kind: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
    ai_title: Option<String>,
    last_prompt: Option<String>,
    is_meta: Option<bool>,
    message: Option<Value>,
}

impl Record {
    /// The text the user typed, when this record is a prompt.
    ///
    /// Meta records, tool results and Claude Code's own wrappers of slash
    /// commands and their output (text that starts with `<`) are not
    /// prompts.
    fn prompt(&self) -> Option<&str> {
        if self.kind.as_deref() != Some("user") || self.is_meta == Some(true) {
            return None;
        }

        let content = self.message.as_ref()?.get("content")?;
        let text = match content {
            Value::String(text) => Some(text.as_str()),
            Value::Array(blocks) => blocks
                .iter()
                .filter(|block| {
                    block.get("type").and_then(Value::as_str) == Some("text")
                })
                .find_map(|block| block.get("text").and_then(Value::as_str)),
            _ => None,
        };

        text.filter(|text| !text.trim_start().starts_with('<'))
    }
}

/// One line of a session file: only the version of `claude` that wrote
/// it.
#[derive(Debug, Deserialize)]
struct VersionRecord {
    version: Option<String>,
}

/// What the records read so far tell about a session.
#[derive(Debug, Default)]
pub(crate) struct SessionSummary {
    folder: Option<String>,
    ai_title: Option<String>,
    first_prompt: Option<String>,
    last_prompt: Option<String>,
    changed_ms: Option<i64>,
}

impl SessionSummary {
    /// Takes what the record on `line` tells: the first folder and prompt,
    /// the newest time, and the last title and prompt.
    pub(crate) fn add_line(&mut self, line: &[u8]) {
        let Ok(record) = serde_json::from_slice::<Record>(line) else {
            return;
        };

        if self.first_prompt.is_none() {
            self.first_prompt = record.prompt().and_then(title_line);
        }
        if self.folder.is_none() {
            self.folder = record.cwd.clone().filter(|cwd| !cwd.is_empty());
        }
        if let Some(title) = record.ai_title.as_deref().and_then(title_line) {
            self.ai_title = Some(title);
        }
        if let Some(prompt) = record.last_prompt.as_deref().and_then(title_line)
        {
            self.last_prompt = Some(prompt);
        }

        let time = record.timestamp.as_deref().and_then(millis_of);
        self.changed_ms = self.changed_ms.max(time);
    }

    /// The session `id`, filling what the records did not tell: the
    /// folder from `fallback_folder`, the time from `file_changed_ms`. A
    /// session with no title OTTY shows by its id.
    pub(crate) fn into_session(
        self,
        id: &str,
        fallback_folder: &str,
        file_changed_ms: i64,
    ) -> Session {
        let title = self.ai_title.or(self.first_prompt).or(self.last_prompt);
        let folder = self.folder.unwrap_or_else(|| fallback_folder.to_string());

        Session {
            id: id.to_string(),
            folder: Some(folder),
            title,
            changed_ms: self.changed_ms.unwrap_or(file_changed_ms),
        }
    }
}

/// The session kept in `file`, a session file. `fallback_folder` is the
/// project folder to use when no record tells it.
pub(crate) fn read_session(
    file: &FileInfo,
    fallback_folder: &str,
    end_bytes: u64,
) -> Result<Session, HostError> {
    let id = session_id(&file.path);

    let mut summary = SessionSummary::default();
    for line in read_ends(file, end_bytes)? {
        summary.add_line(&line);
    }

    Ok(summary.into_session(&id, fallback_folder, file.changed_ms))
}

/// The version of `claude` that the last record naming one in `file`
/// tells, such as "2.1.328", read from the start and the end of the file.
/// `None` when the file can not be read or no record names one.
pub(crate) fn read_version(file: &FileInfo, end_bytes: u64) -> Option<String> {
    let lines = read_ends(file, end_bytes).ok()?;

    lines
        .iter()
        .rev()
        .filter(|line| has_marker(line, VERSION_MARKER))
        .find_map(|line| {
            let record = serde_json::from_slice::<VersionRecord>(line).ok()?;
            record.version.filter(|version| !version.is_empty())
        })
}

/// The id of the session file at `path`: its name without `.jsonl`.
pub(crate) fn session_id(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Whether `line` holds `marker`.
pub(crate) fn has_marker(line: &[u8], marker: &[u8]) -> bool {
    line.windows(marker.len()).any(|window| window == marker)
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
    use otty_plugin_sdk::files;

    use super::*;
    use crate::file_ends::END_BYTES;
    use crate::testing::{as_part, host, path};

    const ID: &str = "3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b";

    const USER: &str = r#"{"type":"user","cwd":"/home/dev/otty","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b","gitBranch":"main","timestamp":"2026-11-16T10:00:00.000Z","message":{"role":"user","content":"Fix the tab bar\nIt flickers on resize"}}"#;
    const ASSISTANT: &str = r#"{"type":"assistant","cwd":"/home/dev/otty","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b","gitBranch":"main","timestamp":"2026-11-16T10:05:00.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Done."}]}}"#;
    const AI_TITLE: &str = r#"{"type":"ai-title","aiTitle":"Stop the tab bar flicker","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b"}"#;
    const LAST_PROMPT: &str = r#"{"type":"last-prompt","lastPrompt":"Run the tests","sessionId":"3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b"}"#;

    /// 2026-11-16T10:05:00Z.
    const FIVE_PAST_TEN: i64 = 1_794_823_500_000;

    fn session_from(lines: &[&str]) -> Session {
        let mut summary = SessionSummary::default();
        for line in lines {
            summary.add_line(line.as_bytes());
        }

        summary.into_session(ID, "/home/dev/fallback", 7)
    }

    fn user_prompt(content: &str) -> String {
        format!(
            r#"{{"type":"user","message":{{"role":"user","content":{content}}}}}"#
        )
    }

    #[test]
    fn a_normal_session_has_its_folder_title_and_time() {
        let session = session_from(&[USER, ASSISTANT, AI_TITLE, LAST_PROMPT]);

        assert_eq!(
            session,
            Session {
                id: ID.to_string(),
                folder: Some("/home/dev/otty".to_string()),
                title: Some("Stop the tab bar flicker".to_string()),
                changed_ms: FIVE_PAST_TEN,
            }
        );
    }

    #[test]
    fn a_session_with_no_title_shows_its_first_prompt() {
        assert_eq!(
            session_from(&[USER, ASSISTANT]).title.as_deref(),
            Some("Fix the tab bar")
        );
    }

    #[test]
    fn a_session_with_no_title_and_no_prompt_shows_its_id() {
        assert_eq!(session_from(&[ASSISTANT]).title, None);
    }

    #[test]
    fn the_title_is_the_first_line_of_the_prompt_cut_to_a_readable_length() {
        let multi_line = session_from(&[&user_prompt(
            r#""\n\n   Rename the panel   \nand its tests""#,
        )]);
        let long_text = format!(r#""{}""#, "a".repeat(300));
        let long = session_from(&[&user_prompt(&long_text)]);

        assert_eq!(multi_line.title.as_deref(), Some("Rename the panel"));
        assert_eq!(
            long.title.as_deref(),
            Some(format!("{}...", "a".repeat(100)).as_str())
        );
    }

    #[test]
    fn meta_records_commands_and_tool_results_are_not_the_prompt() {
        let session = session_from(&[
            r#"{"type":"user","isMeta":true,"message":{"content":"Caveat: made up"}}"#,
            &user_prompt(r#""<command-name>/clear</command-name>""#),
            &user_prompt(r#"[{"type":"tool_result","content":"ok"}]"#),
            &user_prompt(
                r#"[{"type":"image","source":{}},{"type":"text","text":"Add a test"}]"#,
            ),
        ]);

        assert_eq!(session.title.as_deref(), Some("Add a test"));
    }

    #[test]
    fn the_last_prompt_is_the_title_when_no_first_prompt_was_read() {
        let session = session_from(&[
            r#"{"type":"last-prompt","lastPrompt":"Run the tests"}"#,
        ]);

        assert_eq!(session.title.as_deref(), Some("Run the tests"));
    }

    #[test]
    fn the_newest_ai_title_wins() {
        let session = session_from(&[
            r#"{"type":"ai-title","aiTitle":"First idea"}"#,
            r#"{"type":"ai-title","aiTitle":"Better title"}"#,
        ]);

        assert_eq!(session.title.as_deref(), Some("Better title"));
    }

    #[test]
    fn broken_lines_in_the_middle_are_skipped() {
        let wrong_field_type = r#"{"type":"user","cwd":42,"timestamp":"2026-12-01T00:00:00.000Z"}"#;

        let session = session_from(&[
            USER,
            "{\"type\":\"assistant\",\"cwd\":",
            "",
            "not json at all",
            wrong_field_type,
            r#"{"type":"some-future-record","payload":{"x":1}}"#,
            ASSISTANT,
            AI_TITLE,
        ]);

        assert_eq!(session.folder.as_deref(), Some("/home/dev/otty"));
        assert_eq!(session.title.as_deref(), Some("Stop the tab bar flicker"));
        assert_eq!(session.changed_ms, FIVE_PAST_TEN);
    }

    #[test]
    fn an_empty_or_garbled_file_gives_no_title_the_fallback_folder_and_file_time()
     {
        let session = session_from(&["\u{0}\u{1}garbage with no line end"]);

        assert_eq!(
            session,
            Session {
                id: ID.to_string(),
                folder: Some("/home/dev/fallback".to_string()),
                title: None,
                changed_ms: 7,
            }
        );
    }

    #[test]
    fn the_version_is_the_last_one_the_file_names() {
        let host = host();
        let glob = path(&host, ".claude-work/projects/*/*.jsonl");

        let version = as_part(&host, || {
            let found = files::list(&glob).expect("the listing");
            read_version(&found[0], END_BYTES)
        });

        assert_eq!(version.as_deref(), Some("2.1.328"));
    }

    #[test]
    fn a_session_file_is_read_through_the_host() {
        let host = host();
        let glob = path(&host, ".claude/projects/-home-dev-blog/*.jsonl");

        let session = as_part(&host, || {
            let found = files::list(&glob).expect("the listing");
            read_session(&found[0], "/fallback", END_BYTES)
        })
        .expect("the session");

        assert_eq!(session.id, "2a1d3c8f-9e5b-4f70-8c1d-2b3e4f5a6b7c");
        assert_eq!(session.folder.as_deref(), Some("/home/dev/blog"));
        assert_eq!(session.title.as_deref(), Some("Write the release notes"));
    }
}
