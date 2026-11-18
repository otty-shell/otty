//! Reads one Claude Code session file into a session.
//!
//! A session file holds one JSON record on each line and can be large, so
//! only its start and its end are read. Reading is tolerant: a line that is
//! broken or unknown is skipped.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;
use serde_json::Value;

use crate::jsonl::has_marker;
use crate::session::{AgentSession, SessionId};
use crate::timestamp::parse_rfc3339;
use crate::title::title_line;

/// How much is read from the start and from the end of a session file.
const END_SIZE: usize = 128 * 1024;

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

/// What the records read so far tell about a session.
#[derive(Debug, Default)]
struct SessionSummary {
    folder: Option<PathBuf>,
    ai_title: Option<String>,
    first_prompt: Option<String>,
    last_prompt: Option<String>,
    changed_at: Option<SystemTime>,
}

impl SessionSummary {
    /// Takes what `record` tells: the first folder and prompt, the newest
    /// time, and the last title and prompt.
    fn add(&mut self, record: Record) {
        if self.first_prompt.is_none() {
            self.first_prompt = record.prompt().and_then(title_line);
        }
        if self.folder.is_none() {
            self.folder =
                record.cwd.filter(|cwd| !cwd.is_empty()).map(PathBuf::from);
        }
        if let Some(title) = record.ai_title.as_deref().and_then(title_line) {
            self.ai_title = Some(title);
        }
        if let Some(prompt) = record.last_prompt.as_deref().and_then(title_line)
        {
            self.last_prompt = Some(prompt);
        }

        let time = record.timestamp.as_deref().and_then(parse_rfc3339);
        self.changed_at = self.changed_at.max(time);
    }

    /// The session with the id `id`, filling what the records did not tell:
    /// the folder from `fallback_folder`, the title from the id, the time
    /// from `file_time`.
    fn into_session(
        self,
        id: &str,
        fallback_folder: &Path,
        file_time: Option<SystemTime>,
    ) -> AgentSession {
        let folder =
            self.folder.unwrap_or_else(|| fallback_folder.to_path_buf());
        let title = self
            .ai_title
            .or(self.first_prompt)
            .or(self.last_prompt)
            .unwrap_or_else(|| id.to_owned());
        let changed_at = self
            .changed_at
            .or(file_time)
            .unwrap_or(SystemTime::UNIX_EPOCH);

        AgentSession::new(SessionId::new(id), folder, title, changed_at)
    }
}

/// The session kept in the file at `path`.
///
/// `fallback_folder` is the project folder to use when no record tells it.
/// `None` only when the file can not be opened.
pub(super) fn read_session(
    path: &Path,
    fallback_folder: &Path,
) -> Option<AgentSession> {
    let id = path.file_stem()?.to_string_lossy();

    let (parts, file_time) = read_ends(path).ok()?;

    let mut summary = SessionSummary::default();
    for line in parts.iter().flat_map(|part| part.lines()) {
        if let Ok(record) = serde_json::from_str::<Record>(line) {
            summary.add(record);
        }
    }

    Some(summary.into_session(&id, fallback_folder, file_time))
}

/// When the prompt on `line` of a session file was written, when the line
/// is a prompt the user typed, with a time: the same prompts the session
/// list takes its title from.
pub(super) fn prompt_time_on_line(line: &[u8]) -> Option<SystemTime> {
    // Most lines are answers and tool calls: only the records of the user
    // are parsed.
    if !has_marker(line, b"\"type\":\"user\"") {
        return None;
    }

    let record: Record = serde_json::from_slice(line).ok()?;
    record.prompt()?;

    record.timestamp.as_deref().and_then(parse_rfc3339)
}

/// The start and the end of the file at `path`, each cut to whole lines,
/// and the time the file last changed.
///
/// A file that is small enough is read whole, as one part.
fn read_ends(path: &Path) -> io::Result<(Vec<String>, Option<SystemTime>)> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_time = metadata.modified().ok();

    if metadata.len() <= 2 * END_SIZE as u64 {
        let mut whole = Vec::new();
        file.read_to_end(&mut whole)?;
        return Ok((vec![text(&whole)], file_time));
    }

    let mut start = vec![0; END_SIZE];
    file.read_exact(&mut start)?;
    let start_lines = match start.iter().rposition(|byte| *byte == b'\n') {
        Some(last_line_end) => &start[..last_line_end],
        None => &[],
    };

    let mut end = Vec::new();
    file.seek(SeekFrom::End(-(END_SIZE as i64)))?;
    file.read_to_end(&mut end)?;
    let end_lines = match end.iter().position(|byte| *byte == b'\n') {
        Some(first_line_end) => &end[first_line_end + 1..],
        None => &[],
    };

    Ok((vec![text(start_lines), text(end_lines)], file_time))
}

/// Bytes of a session file as text. A broken character only spoils its
/// own line, which is then skipped as broken.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::{TempDir, at, write_file};

    fn session_from(lines: &[&str]) -> AgentSession {
        let temp = TempDir::new();
        let path = temp.path().join("a1.jsonl");
        write_file(&path, &lines.join("\n"));

        read_session(&path, Path::new("/fallback")).expect("the file is read")
    }

    fn user_prompt(content: &str) -> String {
        format!(
            r#"{{"type":"user","message":{{"role":"user","content":{content}}}}}"#
        )
    }

    #[test]
    fn a_large_file_is_read_from_its_start_and_its_end() {
        let padding =
            format!(r#"{{"type":"progress","data":"{}"}}"#, "x".repeat(1000));
        let first = r#"{"type":"user","cwd":"/home/user/otty","gitBranch":"main","timestamp":"2026-11-16T10:00:00.000Z","message":{"content":"Rename the panel"}}"#;
        let middle = r#"{"type":"assistant","cwd":"/elsewhere","gitBranch":"middle","timestamp":"2026-12-01T00:00:00.000Z"}"#;
        let last = r#"{"type":"assistant","cwd":"/home/user/otty","gitBranch":"feature","timestamp":"2026-11-20T08:00:00.000Z"}"#;
        let mut lines = vec![first];
        lines.extend(std::iter::repeat_n(padding.as_str(), 300));
        lines.push(middle);
        lines.extend(std::iter::repeat_n(padding.as_str(), 300));
        lines.push(last);

        let session = session_from(&lines);

        assert_eq!(session.folder(), Path::new("/home/user/otty"));
        assert_eq!(session.title(), "Rename the panel");
        assert_eq!(session.changed_at(), at(1_795_161_600));
    }

    #[test]
    fn the_title_is_the_first_line_of_the_prompt_cut_to_a_readable_length() {
        let multi_line = session_from(&[&user_prompt(
            r#""\n\n   Rename the panel   \nand its tests""#,
        )]);
        let long_text = format!(r#""{}""#, "a".repeat(300));
        let long = session_from(&[&user_prompt(&long_text)]);

        assert_eq!(multi_line.title(), "Rename the panel");
        assert_eq!(long.title(), format!("{}...", "a".repeat(100)));
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

        assert_eq!(session.title(), "Add a test");
    }

    #[test]
    fn the_last_prompt_is_the_title_when_no_first_prompt_was_read() {
        let session = session_from(&[
            r#"{"type":"last-prompt","lastPrompt":"Run the tests"}"#,
        ]);

        assert_eq!(session.title(), "Run the tests");
    }

    #[test]
    fn the_newest_ai_title_wins() {
        let session = session_from(&[
            r#"{"type":"ai-title","aiTitle":"First idea"}"#,
            r#"{"type":"ai-title","aiTitle":"Better title"}"#,
        ]);

        assert_eq!(session.title(), "Better title");
    }

    #[test]
    fn a_file_that_can_not_be_opened_gives_no_session() {
        let temp = TempDir::new();
        let path = temp.path().join("gone.jsonl");

        let session = read_session(&path, &PathBuf::from("/fallback"));

        assert_eq!(session, None);
    }
}
