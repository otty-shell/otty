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
//! with no time or with no tokens is skipped.

use std::fs::File;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::jsonl::{for_each_line, has_marker};
use crate::spend_files::{CountedAnswer, FileSpend};
use crate::timestamp::parse_rfc3339;
use crate::token_count::TokenCount;

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
    message: Option<Message>,
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

/// The project folder and the answers of the session file at `path`.
///
/// The project is the first `cwd` of the file, else `fallback_project`.
/// A file that can not be opened gives no answers.
pub(super) fn read_spend(path: &Path, fallback_project: &Path) -> FileSpend {
    let Ok(file) = File::open(path) else {
        return FileSpend::default();
    };

    let mut project = None;
    let mut answers = Vec::new();
    for_each_line(file, |line| {
        if let Some(record) = counted_record(line, project.is_none()) {
            if project.is_none() {
                project = record
                    .cwd
                    .as_deref()
                    .filter(|cwd| !cwd.is_empty())
                    .map(PathBuf::from);
            }
            answers.extend(answer(record));
        }
    });

    let project = project.unwrap_or_else(|| fallback_project.to_path_buf());
    FileSpend::new(Some(project), answers)
}

/// The answer on `line` of a session file, when it is an answer of the
/// model with a time and some tokens: read the same way as the lines of
/// [`read_spend`].
pub(super) fn answer_on_line(line: &[u8]) -> Option<CountedAnswer> {
    counted_record(line, false).and_then(answer)
}

/// The record on `line`, when it can tell the tokens of an answer, or the
/// folder when `wants_folder`.
///
/// Most lines are prompts and tool calls: only the ones that can tell what
/// is wanted are parsed.
fn counted_record(line: &[u8], wants_folder: bool) -> Option<Record> {
    let is_wanted = has_marker(line, b"\"usage\"")
        || (wants_folder && has_marker(line, b"\"cwd\""));
    if !is_wanted {
        return None;
    }

    serde_json::from_slice(line).ok()
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
