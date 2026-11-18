//! Reads the answers of the model and their tokens from one Codex session
//! file, plain or compressed.
//!
//! After each answer Codex writes an `event_msg` record whose payload is a
//! `token_count` with `info.last_token_usage` (this answer) and
//! `info.total_token_usage` (the whole session so far), each with
//! `input_tokens`, `cached_input_tokens` and `output_tokens`. The input
//! includes the cached part. The model is named by the `turn_context`
//! record that starts each turn, and the project by the `session_meta`
//! record on the first line. This follows CodexBar's `docs/codex.md`:
//! <https://github.com/steipete/CodexBar>.
//!
//! The rules for the same answer written more than once:
//!
//! - Codex can write the same `token_count` twice in a row. A record whose
//!   session total did not change since the previous one is skipped.
//! - The same record can be in two files: the plain and the compressed
//!   copy of a session, or a session and its archived copy, or the history
//!   a forked session copies. Each answer carries the identity of its
//!   record (its time and its numbers), so the copies count once.
//!
//! Reading is tolerant: a broken line, a `token_count` with no numbers or
//! with no time is skipped, and a file that breaks in the middle gives what
//! was read before.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use super::session_file::open_session_file;
use crate::jsonl::{for_each_line, has_marker};
use crate::spend_files::{CountedAnswer, FileSpend};
use crate::timestamp::parse_rfc3339;
use crate::token_count::TokenCount;

/// One line of a session file: its time, its type and its payload.
#[derive(Debug, Deserialize)]
pub(super) struct Line {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    payload: Option<Value>,
}

impl Line {
    /// The time the line was written, as Codex wrote it.
    pub(super) fn timestamp(&self) -> Option<&str> {
        self.timestamp.as_deref()
    }

    /// The type of the record, such as `event_msg`.
    pub(super) fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// The payload of the record.
    pub(super) fn payload(&self) -> Option<&Value> {
        self.payload.as_ref()
    }
}

/// The tokens of a `token_count` record, as Codex writes them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

impl Usage {
    /// The tokens in the shape of [`TokenCount`]: the input without its
    /// cached part.
    fn tokens(&self) -> TokenCount {
        TokenCount::new(
            self.input_tokens.saturating_sub(self.cached_input_tokens),
            self.cached_input_tokens,
            self.output_tokens,
        )
    }

    /// How much each number grew since `previous`; a number that went
    /// down grew by nothing.
    fn growth_since(&self, previous: Self) -> Self {
        Self {
            input_tokens: self
                .input_tokens
                .saturating_sub(previous.input_tokens),
            cached_input_tokens: self
                .cached_input_tokens
                .saturating_sub(previous.cached_input_tokens),
            output_tokens: self
                .output_tokens
                .saturating_sub(previous.output_tokens),
        }
    }
}

/// What the lines of one session file told so far.
#[derive(Debug, Default)]
pub(super) struct SessionTokens {
    project: Option<PathBuf>,
    model: Option<String>,
    previous_total: Option<Usage>,
    answers: Vec<CountedAnswer>,
}

impl SessionTokens {
    /// What the lines told: the project and the answers.
    pub(super) fn into_file_spend(self) -> FileSpend {
        FileSpend::new(self.project, self.answers)
    }

    /// Takes what `line` tells: the project of `session_meta`, the model
    /// of `turn_context`, the answer of `token_count`.
    pub(super) fn add(&mut self, line: &Line) {
        let Some(payload) = &line.payload else {
            return;
        };

        match line.kind.as_deref() {
            Some("session_meta") if self.project.is_none() => {
                self.project = payload
                    .get("cwd")
                    .and_then(Value::as_str)
                    .filter(|cwd| !cwd.is_empty())
                    .map(PathBuf::from);
            },
            Some("turn_context") => {
                if let Some(model) =
                    payload.get("model").and_then(Value::as_str)
                {
                    self.model = Some(model.to_string());
                }
            },
            Some("event_msg") => {
                self.add_token_count(line.timestamp.as_deref(), payload);
            },
            _ => {},
        }
    }

    /// Takes the answer of a `token_count` payload written at `timestamp`.
    ///
    /// The tokens of the answer are `last_token_usage`; with none, how much
    /// `total_token_usage` grew since the previous record. A record whose
    /// total did not change is a repeat and is skipped.
    fn add_token_count(&mut self, timestamp: Option<&str>, payload: &Value) {
        if payload.get("type").and_then(Value::as_str) != Some("token_count") {
            return;
        }
        let Some(info) = payload.get("info") else {
            return;
        };
        let Some(last) = usage(info, "last_token_usage") else {
            return;
        };
        let Some(total) = usage(info, "total_token_usage") else {
            return;
        };
        if total.is_some() && total == self.previous_total {
            return;
        }

        let usage = match (last, total, self.previous_total) {
            (Some(last), _, _) => last,
            (None, Some(total), Some(previous)) => total.growth_since(previous),
            (None, Some(total), None) => total,
            (None, None, _) => return,
        };
        if total.is_some() {
            self.previous_total = total;
        }

        let Some(at) = timestamp.and_then(parse_rfc3339) else {
            return;
        };
        let tokens = usage.tokens();
        if tokens.is_zero() {
            return;
        }

        let answer = CountedAnswer::new(at, self.model.as_deref(), tokens)
            .with_identity(&(timestamp, last, total));
        self.answers.push(answer);
    }
}

/// The project folder and the answers of the session file at `path`.
///
/// A file that can not be opened gives no answers.
pub(super) fn read_spend(path: &Path) -> FileSpend {
    let Ok(reader) = open_session_file(path) else {
        return FileSpend::default();
    };

    let mut session = SessionTokens::default();
    for_each_line(reader, |line| {
        if is_counted_line(line)
            && let Ok(parsed) = serde_json::from_slice::<Line>(line)
        {
            session.add(&parsed);
        }
    });

    session.into_file_spend()
}

/// Whether `line` of a session file names a record the count reads.
///
/// Most lines are messages: only the ones that name such a record are
/// parsed.
pub(super) fn is_counted_line(line: &[u8]) -> bool {
    let markers: [&[u8]; 3] =
        [b"token_count", b"turn_context", b"session_meta"];

    markers.iter().any(|marker| has_marker(line, marker))
}

/// The usage in the field `field` of `info`: `Some(None)` when the field is
/// missing or `null`, `None` when it is there but not a usage, which makes
/// the whole record broken.
fn usage(info: &Value, field: &str) -> Option<Option<Usage>> {
    match info.get(field) {
        None | Some(Value::Null) => Some(None),
        Some(value) => Usage::deserialize(value).ok().map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{NOW, TempDir, at, write_compressed, write_file};

    const META: &str = r#"{"timestamp":"2026-11-16T09:00:00.000Z","type":"session_meta","payload":{"id":"019a8f3e","cwd":"/home/user/otty","source":"cli"}}"#;

    fn turn(model: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-11-16T09:00:01.000Z","type":"turn_context","payload":{{"cwd":"/home/user/otty","model":"{model}"}}}}"#
        )
    }

    /// A `token_count` record at `time` with `last` and `total`, each as
    /// input, cached input and output.
    fn count(
        time: &str,
        last: (u64, u64, u64),
        total: (u64, u64, u64),
    ) -> String {
        let usage = |(input, cached, output): (u64, u64, u64)| {
            format!(
                r#"{{"input_tokens":{input},"cached_input_tokens":{cached},"output_tokens":{output},"reasoning_output_tokens":0,"total_tokens":{}}}"#,
                input + output
            )
        };
        let last = usage(last);
        let total = usage(total);
        format!(
            r#"{{"timestamp":"{time}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{total},"last_token_usage":{last},"model_context_window":272000}},"rate_limits":null}}}}"#
        )
    }

    fn spend_of(lines: &[&str]) -> FileSpend {
        let temp = TempDir::new();
        let path = temp
            .path()
            .join("rollout-2026-11-16T09-00-00-019a8f3e.jsonl");
        write_file(&path, &lines.join("\n"));

        read_spend(&path)
    }

    fn tokens(spend: &FileSpend) -> Vec<TokenCount> {
        spend.answers().iter().map(CountedAnswer::tokens).collect()
    }

    #[test]
    fn each_answer_gives_its_time_model_and_tokens() {
        let spend = spend_of(&[
            META,
            &turn("gpt-5-codex"),
            &count(
                "2026-11-16T10:00:00.000Z",
                (1000, 800, 50),
                (1000, 800, 50),
            ),
            &turn("gpt-5.1-codex"),
            &count("2026-11-16T10:01:00.000Z", (300, 100, 20), (1300, 900, 70)),
        ]);

        assert_eq!(spend.project(), Some(Path::new("/home/user/otty")));
        assert_eq!(
            tokens(&spend),
            [TokenCount::new(200, 800, 50), TokenCount::new(200, 100, 20)]
        );
        let answers = spend.answers();
        assert_eq!(answers[0].at(), at(NOW));
        assert_eq!(answers[0].model(), Some("gpt-5-codex"));
        assert_eq!(answers[1].model(), Some("gpt-5.1-codex"));
        assert!(answers[0].identity().is_some());
        assert_ne!(answers[0].identity(), answers[1].identity());
    }

    #[test]
    fn a_record_written_again_with_the_same_total_is_skipped() {
        let first = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));
        let again = count("2026-11-16T10:00:01Z", (10, 0, 5), (10, 0, 5));
        let next = count("2026-11-16T10:02:00Z", (10, 0, 5), (20, 0, 10));

        let spend = spend_of(&[META, &first, &again, &next]);

        assert_eq!(spend.answers().len(), 2);
    }

    #[test]
    fn with_no_last_usage_the_growth_of_the_total_is_the_answer() {
        let only_total = |time: &str, total: &str| {
            format!(
                r#"{{"timestamp":"{time}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{total}}}}}}}"#
            )
        };

        let spend = spend_of(&[
            META,
            &only_total(
                "2026-11-16T10:00:00Z",
                r#"{"input_tokens":100,"cached_input_tokens":40,"output_tokens":5}"#,
            ),
            &only_total(
                "2026-11-16T10:01:00Z",
                r#"{"input_tokens":150,"cached_input_tokens":90,"output_tokens":8}"#,
            ),
        ]);

        assert_eq!(
            tokens(&spend),
            [TokenCount::new(60, 40, 5), TokenCount::new(0, 50, 3)]
        );
    }

    #[test]
    fn a_compressed_file_is_read() {
        let temp = TempDir::new();
        let path = temp.path().join("rollout-2025-01-02T09-00-00-a1.jsonl.zst");
        let line = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));
        write_compressed(&path, &format!("{META}\n{line}\n"));

        let spend = read_spend(&path);

        assert_eq!(tokens(&spend), [TokenCount::new(10, 0, 5)]);
    }

    #[test]
    fn broken_lines_and_records_with_no_numbers_are_skipped() {
        let good = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));
        let lines = [
            META,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\"",
            "not json, but it says token_count",
            r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":null}}"#,
            r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{}}}"#,
            r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":"many"}}}}"#,
            &count("2026-11-16T10:00:00Z", (0, 0, 0), (0, 0, 0)),
            &count("not a time", (1, 0, 1), (1, 0, 1)),
            r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"agent_message","message":"token_count"}}"#,
            &good,
        ];

        let spend = spend_of(&lines);

        assert_eq!(tokens(&spend), [TokenCount::new(10, 0, 5)]);
    }

    #[test]
    fn a_file_with_no_session_meta_has_no_project_and_no_model() {
        let line = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));

        let spend = spend_of(&[&line]);

        assert_eq!(spend.project(), None);
        assert_eq!(spend.answers()[0].model(), None);
    }

    #[test]
    fn a_file_that_can_not_be_opened_gives_no_answers() {
        let temp = TempDir::new();

        let spend = read_spend(&temp.path().join("rollout-gone.jsonl"));

        assert_eq!(spend, FileSpend::default());
    }
}
