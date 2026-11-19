//! Reads the answers of the model and their tokens from one Claude Code
//! session file.
//!
//! Each answer of the model is a record of type `assistant` whose
//! `message.usage` has the tokens: `input_tokens`,
//! `cache_creation_input_tokens`, `cache_read_input_tokens` and
//! `output_tokens`. One answer can be written as several records, one for
//! each part of it, and a resumed session copies the earlier answers into
//! its new file, so each record carries the identity of its answer: the
//! pair of `message.id` and `requestId`, or of `sessionId` and
//! `message.id` when the request id is missing. This follows CodexBar's
//! `docs/claude.md`: <https://github.com/steipete/CodexBar>.
//!
//! Reading is tolerant: a broken line, a record of another type, a record
//! with no time or with no tokens is skipped. The answers are counted as
//! records seen and understood, so a changed format is told and not
//! taken for fewer tokens (spec 2026-11-19, Decision 9).

use std::fs::File;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::jsonl::{for_each_line, has_marker};
use crate::record_format::{RecordCounts, RecordFormat};
use crate::spend_files::{CountedAnswer, FileSpend};
use crate::timestamp::parse_rfc3339;
use crate::token_count::TokenCount;

/// The newest version of Claude Code whose session files this reader was
/// checked against. Raise it after checking the reader with a newer one.
pub(super) const TESTED_VERSION: &str = "2.1.328";

/// The files this reader reads, as a report of a changed format names
/// them, and the version they were checked with.
pub(super) const SESSION_FILES: RecordFormat = RecordFormat::new(
    "session file (projects/<project>/<session>.jsonl), assistant records",
    TESTED_VERSION,
);

/// What marks a line that may be an answer of the model.
const ANSWER_MARKERS: [&[u8]; 2] = [b"\"usage\"", b"\"type\":\"assistant\""];

/// One line of a session file: only the fields the count needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(rename = "type")]
    kind: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
    session_id: Option<String>,
    request_id: Option<String>,
    version: Option<String>,
    message: Option<Message>,
}

impl Record {
    /// What an answer of the model lacks for the count, when it does: the
    /// field that is missing or of another shape, and the field it was
    /// looked for in (empty for the record itself). `None` when the answer
    /// has a time and its tokens.
    fn missing_field(&self) -> Option<(&'static str, &'static str)> {
        if self.timestamp.as_deref().and_then(parse_rfc3339).is_none() {
            return Some(("timestamp", ""));
        }
        let Some(message) = &self.message else {
            return Some(("message", ""));
        };

        match &message.usage {
            None => Some(("message.usage", "message")),
            Some(usage) if !usage.has_counts() => {
                Some(("message.usage", "message.usage"))
            },
            Some(_) => None,
        }
    }
}

/// The `message` of a record.
#[derive(Debug, Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

/// The `usage` of an answer.
#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

impl Usage {
    /// Whether it has any of the counts of tokens: a `usage` whose counts
    /// were all renamed has none.
    fn has_counts(&self) -> bool {
        self.input_tokens.is_some()
            || self.output_tokens.is_some()
            || self.cache_creation_input_tokens.is_some()
            || self.cache_read_input_tokens.is_some()
    }

    /// The tokens in the shape of [`TokenCount`]: what was written to the
    /// cache is a part of the input.
    fn tokens(&self) -> TokenCount {
        let written = self.cache_creation_input_tokens.unwrap_or_default();
        let input = self.input_tokens.unwrap_or_default();

        TokenCount::new(
            input.saturating_add(written),
            self.cache_read_input_tokens.unwrap_or_default(),
            self.output_tokens.unwrap_or_default(),
        )
        .with_cache_written(written)
    }
}

/// The project folder and the answers of the session file at `path`,
/// and how many answers were seen and understood.
///
/// The project is the first `cwd` of the file, else `fallback_project`.
/// A file that can not be opened gives no answers.
pub(super) fn read_spend(path: &Path, fallback_project: &Path) -> FileSpend {
    let Ok(file) = File::open(path) else {
        return FileSpend::default();
    };

    let mut project = None;
    let mut answers = Vec::new();
    let mut records = RecordCounts::default();
    for_each_line(file, |line| {
        if !is_wanted(line, project.is_none()) {
            return;
        }

        let record = serde_json::from_slice::<Record>(line).ok();
        if is_answer_line(line) {
            count_answer(line, record.as_ref(), &mut records);
        }
        let Some(record) = record else {
            return;
        };

        if project.is_none() {
            project = record
                .cwd
                .as_deref()
                .filter(|cwd| !cwd.is_empty())
                .map(PathBuf::from);
        }
        answers.extend(answer(record));
    });

    let project = project.unwrap_or_else(|| fallback_project.to_path_buf());
    FileSpend::new(Some(project), answers).with_records(records)
}

/// The answer on `line` of a session file, when it is an answer of the
/// model with a time and some tokens: read the same way as the lines of
/// [`read_spend`].
pub(super) fn answer_on_line(line: &[u8]) -> Option<CountedAnswer> {
    counted_record(line, false).and_then(answer)
}

/// The record on `line`, when it can tell the tokens of an answer, or the
/// folder when `wants_folder`.
fn counted_record(line: &[u8], wants_folder: bool) -> Option<Record> {
    if !is_wanted(line, wants_folder) {
        return None;
    }

    serde_json::from_slice(line).ok()
}

/// Whether `line` can tell the tokens of an answer, or the folder when
/// `wants_folder`.
///
/// Most lines are prompts and tool calls: only the ones that can tell what
/// is wanted are parsed.
fn is_wanted(line: &[u8], wants_folder: bool) -> bool {
    is_answer_line(line) || (wants_folder && has_marker(line, b"\"cwd\""))
}

/// Whether `line` may be an answer of the model.
fn is_answer_line(line: &[u8]) -> bool {
    ANSWER_MARKERS.iter().any(|marker| has_marker(line, marker))
}

/// Counts the record on `line`, parsed as `record` when it could be, in
/// `records` when it is an answer of the model: understood when it has a
/// time and its tokens.
fn count_answer(
    line: &[u8],
    record: Option<&Record>,
    records: &mut RecordCounts,
) {
    let Some(record) = record else {
        count_unparsed(line, records);
        return;
    };
    if record.kind.as_deref() != Some("assistant") {
        return;
    }

    if let Some(version) = &record.version {
        records.saw_version(version);
    }
    let Some((missing, place)) = record.missing_field() else {
        records.add_understood();
        return;
    };

    let value = serde_json::from_slice::<Value>(line).ok();
    let found = value.as_ref().and_then(|value| found_at(value, place));
    records.add_not_understood(missing, place, found);
}

/// Counts the line `line` that is no record the count can read, in
/// `records`: a line that is not JSON, such as the last line of a file
/// Claude Code is still writing, or an answer whose fields have another
/// shape. Any other record is not counted.
fn count_unparsed(line: &[u8], records: &mut RecordCounts) {
    let Ok(value) = serde_json::from_slice::<Value>(line) else {
        records.add_not_understood("JSON record", "", None);
        return;
    };
    if value.get("type").and_then(Value::as_str) != Some("assistant") {
        return;
    }

    if let Some(version) = value.get("version").and_then(Value::as_str) {
        records.saw_version(version);
    }
    records.add_not_understood(
        "message.usage",
        "message",
        found_at(&value, "message"),
    );
}

/// What `record` holds at `place`, fields joined by dots, such as
/// `message.usage`; the record itself for an empty `place`.
fn found_at<'a>(record: &'a Value, place: &str) -> Option<&'a Value> {
    place
        .split('.')
        .filter(|field| !field.is_empty())
        .try_fold(record, |value, field| value.get(field))
}

/// The answer `record` tells, when it is an answer of the model with a
/// time and some tokens.
fn answer(record: Record) -> Option<CountedAnswer> {
    if record.kind.as_deref() != Some("assistant") {
        return None;
    }
    let at = record.timestamp.as_deref().and_then(parse_rfc3339)?;
    let message = record.message?;
    let tokens = message.usage.as_ref()?.tokens();
    if tokens.is_zero() {
        return None;
    }

    let answer = CountedAnswer::new(at, message.model.as_deref(), tokens);

    let identity = match (message.id, record.request_id, record.session_id) {
        (Some(id), Some(request), _) => Some(("request", id, request)),
        (Some(id), None, Some(session)) => Some(("session", session, id)),
        _ => None,
    };
    match identity {
        Some(parts) => Some(answer.with_identity(&parts)),
        None => Some(answer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{NOW, TempDir, at, write_file};

    fn assistant(id: &str, request: &str, time: &str, usage: &str) -> String {
        format!(
            r#"{{"type":"assistant","cwd":"/home/user/otty","sessionId":"s1","requestId":"{request}","timestamp":"{time}","message":{{"id":"{id}","model":"claude-opus-4-5","role":"assistant","content":[{{"type":"text","text":"Done."}}],"usage":{usage}}}}}"#
        )
    }

    /// `record` with the version of Claude Code `version` that wrote it.
    fn with_version(record: &str, version: &str) -> String {
        record.replacen('{', &format!(r#"{{"version":"{version}","#), 1)
    }

    fn spend_of(lines: &[&str]) -> FileSpend {
        let temp = TempDir::new();
        let path = temp.path().join("s1.jsonl");
        write_file(&path, &lines.join("\n"));

        read_spend(&path, Path::new("/fallback"))
    }

    #[test]
    fn each_answer_gives_its_time_model_and_tokens() {
        let usage = r#"{"input_tokens":12,"cache_creation_input_tokens":100,"cache_read_input_tokens":2000,"output_tokens":30}"#;
        let user = r#"{"type":"user","cwd":"/home/user/otty","timestamp":"2026-11-16T09:59:00.000Z","message":{"role":"user","content":"Fix it"}}"#;
        let first =
            assistant("msg_1", "req_1", "2026-11-16T10:00:00.000Z", usage);
        let second = assistant(
            "msg_2",
            "req_2",
            "2026-11-16T10:01:00.000Z",
            r#"{"input_tokens":1,"output_tokens":2}"#,
        );

        let spend = spend_of(&[user, &first, &second]);

        assert_eq!(spend.project(), Some(Path::new("/home/user/otty")));
        let answers = spend.answers();
        assert_eq!(answers.len(), 2);
        assert_eq!(answers[0].at(), at(NOW));
        assert_eq!(answers[0].model(), Some("claude-opus-4-5"));
        assert_eq!(
            answers[0].tokens(),
            TokenCount::new(112, 2000, 30).with_cache_written(100)
        );
        assert_eq!(answers[1].tokens(), TokenCount::new(1, 0, 2));
        assert_ne!(answers[0].identity(), answers[1].identity());
    }

    #[test]
    fn parts_of_one_answer_share_its_identity() {
        let time = "2026-11-16T10:00:00.000Z";
        let part = assistant("msg_1", "req_1", time, r#"{"output_tokens":1}"#);
        let whole = assistant("msg_1", "req_1", time, r#"{"output_tokens":9}"#);

        let spend = spend_of(&[&part, &whole]);

        let answers = spend.answers();
        assert_eq!(answers.len(), 2);
        assert!(answers[0].identity().is_some());
        assert_eq!(answers[0].identity(), answers[1].identity());
    }

    #[test]
    fn with_no_request_id_the_session_and_message_ids_tell_the_answer() {
        let line = |session: &str, id: &str| {
            format!(
                r#"{{"type":"assistant","sessionId":"{session}","timestamp":"2026-11-16T10:00:00Z","message":{{"id":"{id}","usage":{{"output_tokens":3}}}}}}"#
            )
        };
        let no_ids = r#"{"type":"assistant","timestamp":"2026-11-16T10:00:00Z","message":{"usage":{"output_tokens":3}}}"#;

        let spend = spend_of(&[
            &line("s1", "msg_1"),
            &line("s1", "msg_1"),
            &line("s2", "msg_1"),
            no_ids,
        ]);

        let identities: Vec<_> = spend
            .answers()
            .iter()
            .map(CountedAnswer::identity)
            .collect();
        assert!(identities[0].is_some());
        assert_eq!(identities[0], identities[1]);
        assert_ne!(identities[0], identities[2]);
        assert_eq!(identities[3], None);
        assert_eq!(spend.answers()[3].model(), None);
    }

    #[test]
    fn broken_lines_and_records_with_no_numbers_are_skipped() {
        let time = "2026-11-16T10:00:00.000Z";
        let good = assistant("msg_1", "req_1", time, r#"{"output_tokens":5}"#);
        let lines = [
            "{\"type\":\"assistant\",\"message\":",
            "",
            "not json",
            r#"{"type":"assistant","timestamp":"2026-11-16T10:00:00Z","message":"text"}"#,
            &assistant("msg_2", "req_2", time, r#"{"input_tokens":-1}"#),
            &assistant(
                "msg_3",
                "req_3",
                time,
                r#"{"input_tokens":0,"output_tokens":0}"#,
            ),
            &assistant("msg_4", "req_4", "yesterday", r#"{"output_tokens":5}"#),
            &assistant("msg_5", "req_5", time, "null"),
            r#"{"type":"user","timestamp":"2026-11-16T10:00:00Z","message":{"usage":{"output_tokens":5}}}"#,
            &good,
        ];

        let spend = spend_of(&lines);

        assert_eq!(spend.answers().len(), 1);
        assert_eq!(spend.answers()[0].tokens(), TokenCount::new(0, 0, 5));
    }

    #[test]
    fn each_answer_is_a_record_understood_with_the_version_of_the_agent() {
        let usage = r#"{"input_tokens":1,"output_tokens":2}"#;
        let user = r#"{"type":"user","version":"2.1.300","cwd":"/home/user/otty","timestamp":"2026-11-16T09:59:00.000Z","message":{"role":"user","content":"Fix it"}}"#;
        let zero = r#"{"input_tokens":0,"output_tokens":0}"#;
        let first = with_version(
            &assistant("msg_1", "req_1", "2026-11-16T10:00:00Z", usage),
            "2.1.327",
        );
        let second = with_version(
            &assistant("msg_2", "req_2", "2026-11-16T10:01:00Z", zero),
            "2.1.328",
        );

        let spend = spend_of(&[user, &first, &second]);

        let records = spend.records();
        assert_eq!((records.seen(), records.understood()), (2, 2));
        assert_eq!(records.version(), Some("2.1.328"));
    }

    /// Spec 2026-11-19, Decision 9: an answer whose fields were renamed is
    /// seen but not understood; the names of its fields are kept, never
    /// their values.
    #[test]
    fn answers_with_renamed_fields_are_seen_but_not_understood() {
        let renamed = r#"{"type":"assistant","version":"2.2.0","timestamp":"2026-11-16T10:00:00Z","message":{"id":"msg_1","model":"claude-opus-9","tokenUsage":{"in":5}}}"#;
        let inner = r#"{"type":"assistant","timestamp":"2026-11-16T10:00:00Z","message":{"usage":{"inputTokens":5}}}"#;
        let no_time = r#"{"type":"assistant","at":"2026-11-16T10:00:00Z","message":{"usage":{"output_tokens":5}}}"#;
        let shape = r#"{"type":"assistant","timestamp":"2026-11-16T10:00:00Z","message":{"usage":{"output_tokens":-5}}}"#;
        let broken = r#"{"type":"assistant","message":{"usage":"#;
        let tool_result = r#"{"type":"user","timestamp":"2026-11-16T10:00:00Z","toolUseResult":{"usage":{"output_tokens":5}}}"#;

        let spend =
            spend_of(&[renamed, inner, no_time, shape, broken, tool_result]);

        let records = spend.records();
        assert_eq!((records.seen(), records.understood()), (5, 0));
        assert_eq!(records.version(), Some("2.2.0"));
        assert_eq!(
            records.missing(),
            ["JSON record", "message.usage", "timestamp"]
        );
        for field in ["message.tokenUsage", "message.usage.inputTokens", "at"] {
            assert!(records.fields().contains(field), "{field}");
        }
        assert!(spend.answers().is_empty());
    }

    #[test]
    fn a_file_with_no_folder_uses_the_fallback_project() {
        let spend = spend_of(&[
            r#"{"type":"assistant","cwd":"","timestamp":"2026-11-16T10:00:00Z","message":{"usage":{"output_tokens":3}}}"#,
        ]);

        assert_eq!(spend.project(), Some(Path::new("/fallback")));
        assert_eq!(spend.answers().len(), 1);
    }

    #[test]
    fn a_file_that_can_not_be_opened_gives_no_answers() {
        let temp = TempDir::new();

        let spend =
            read_spend(&temp.path().join("gone.jsonl"), Path::new("/f"));

        assert_eq!(spend.answers(), []);
    }
}
