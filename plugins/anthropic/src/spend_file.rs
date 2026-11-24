//! The answers of the model and their tokens in one Claude Code session
//! file.
//!
//! Each answer of the model is a record of type `assistant` whose
//! `message.usage` has the tokens: `input_tokens`,
//! `cache_creation_input_tokens`, `cache_read_input_tokens` and
//! `output_tokens`. One answer can be written as several records, one for
//! each part of it, and a resumed session copies the earlier answers into
//! its new file, so each record carries the identity of its answer: the
//! pair of `message.id` and `requestId`, or of `sessionId` and
//! `message.id` when the request id is missing. OTTY counts the answers of
//! one identity once.
//!
//! Reading is tolerant: a broken line, a record of another type, a record
//! with no time or with no tokens is skipped. The answers are counted as
//! records seen and understood, so a changed format is told and not taken
//! for fewer tokens.

use otty_plugin_sdk::agents::{
    Answer, Answers, ProviderRef, ReadError, RecordCounts, TokenCount,
    answer_cost,
};
use otty_plugin_sdk::files::{self, BATCH_BYTES};
use serde::Deserialize;
use serde_json::Value;

use crate::project_folder::{
    fallback_project, projects_folder, session_of_file,
};
use crate::session_file::has_marker;
use crate::timestamp::millis_of;

/// What marks a line that may be an answer of the model.
const ANSWER_MARKERS: [&[u8]; 2] = [b"\"usage\"", b"\"type\":\"assistant\""];

/// What marks a line that may tell the working folder.
const FOLDER_MARKER: &[u8] = b"\"cwd\"";

/// How many bytes of lines are read while the working folder is not
/// known yet: every line with a folder comes then, not only answers.
const FOLDER_BATCH_BYTES: u32 = 64 * 1024;

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
        if self.timestamp.as_deref().and_then(millis_of).is_none() {
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

    /// The tokens in the shape of the contract: what was written to the
    /// cache is a part of the input.
    fn tokens(&self) -> TokenCount {
        let written = self.cache_creation_input_tokens.unwrap_or_default();
        let input = self.input_tokens.unwrap_or_default();

        TokenCount {
            input: input.saturating_add(written),
            cached: self.cache_read_input_tokens.unwrap_or_default(),
            cache_written: written,
            output: self.output_tokens.unwrap_or_default(),
            reasoning: 0,
        }
    }
}

/// What one session file told so far: its working folder, its answers
/// and the records seen.
#[derive(Debug, Default)]
pub(crate) struct SpendFile {
    project: Option<String>,
    answers: Vec<Answer>,
    records: RecordCounts,
}

impl SpendFile {
    /// Whether the working folder is still unknown.
    fn wants_folder(&self) -> bool {
        self.project.is_none()
    }

    /// Takes what the record on `line` tells: an answer of the model, and
    /// the working folder while it is unknown. Only the lines that can
    /// tell one of them are parsed.
    pub(crate) fn add_line(&mut self, line: &[u8]) {
        let is_answer = is_answer_line(line);
        let tells_folder =
            self.wants_folder() && has_marker(line, FOLDER_MARKER);
        if !is_answer && !tells_folder {
            return;
        }

        let record = serde_json::from_slice::<Record>(line).ok();
        if is_answer {
            count_answer(line, record.as_ref(), &mut self.records);
        }
        let Some(record) = record else {
            return;
        };

        if self.wants_folder() {
            self.project = record.cwd.clone().filter(|cwd| !cwd.is_empty());
        }
        self.answers.extend(answer_of(record));
    }

    /// The answers for OTTY, each in the session `session`, under the
    /// file's working folder, else `fallback_project`, and the provider
    /// `provider`. Claude Code writes no cost, so each answer is priced at
    /// the API price OTTY gives for its model and that provider (spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 3).
    pub(crate) fn into_answers(
        self,
        fallback_project: &str,
        provider: Option<&ProviderRef>,
        session: Option<&str>,
    ) -> Answers {
        let project =
            self.project.unwrap_or_else(|| fallback_project.to_string());

        let answers = self
            .answers
            .into_iter()
            .map(|answer| {
                let cost = answer_cost(
                    None,
                    answer.model.as_deref(),
                    provider,
                    &answer.tokens,
                );

                Answer {
                    session: session.map(str::to_string),
                    project: Some(project.clone()),
                    provider: provider.cloned(),
                    cost,
                    ..answer
                }
            })
            .collect();

        Answers {
            answers,
            records: self.records,
        }
    }
}

/// The answers in the session file at `path` of the profile in `folder`,
/// counted under `provider`: the host hands over only the lines that may
/// be answers, and, until the working folder is known, the lines that may
/// tell it.
pub(crate) fn read_answers(
    folder: &str,
    path: &str,
    provider: Option<&ProviderRef>,
) -> Result<Answers, ReadError> {
    let with_folder: Vec<Vec<u8>> = ANSWER_MARKERS
        .iter()
        .chain([&FOLDER_MARKER])
        .map(|marker| marker.to_vec())
        .collect();
    let answers_only: Vec<Vec<u8>> = ANSWER_MARKERS
        .iter()
        .map(|marker| marker.to_vec())
        .collect();

    let mut file = SpendFile::default();
    let mut offset = 0;
    loop {
        let (markers, bytes) = if file.wants_folder() {
            (&with_folder, FOLDER_BATCH_BYTES)
        } else {
            (&answers_only, BATCH_BYTES)
        };
        let batch = files::read_lines(path, offset, bytes, markers)
            .map_err(ReadError::Host)?;
        for line in batch.lines() {
            file.add_line(line);
        }

        if batch.at_end || batch.next_offset <= offset {
            break;
        }
        offset = batch.next_offset;
    }

    let projects = projects_folder(folder);
    let fallback = fallback_project(&projects, path);
    let session = session_of_file(&projects, path);

    Ok(file.into_answers(&fallback, provider, session.as_deref()))
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
    records.add_not_understood(missing, place, field_names(found));
}

/// Counts the line `line` that is no record the count can read, in
/// `records`: a line that is not JSON, such as the last line of a file
/// Claude Code is still writing, or an answer whose fields have another
/// shape. Any other record is not counted.
fn count_unparsed(line: &[u8], records: &mut RecordCounts) {
    let Ok(value) = serde_json::from_slice::<Value>(line) else {
        records.add_not_understood("JSON record", "", []);
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
        field_names(found_at(&value, "message")),
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

/// The names of the fields of `value`, when it is an object.
fn field_names(value: Option<&Value>) -> impl Iterator<Item = &str> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|fields| fields.keys().map(String::as_str))
}

/// The answer `record` tells, when it is an answer of the model with a
/// time and some tokens.
fn answer_of(record: Record) -> Option<Answer> {
    if record.kind.as_deref() != Some("assistant") {
        return None;
    }
    let at_ms = record.timestamp.as_deref().and_then(millis_of)?;
    let message = record.message?;
    let tokens = message.usage.as_ref()?.tokens();
    if tokens.input == 0 && tokens.cached == 0 && tokens.output == 0 {
        return None;
    }

    let identity = match (&message.id, &record.request_id, &record.session_id) {
        (Some(id), Some(request), _) => Some(format!("request:{id}:{request}")),
        (Some(id), None, Some(session)) => {
            Some(format!("session:{session}:{id}"))
        },
        _ => None,
    };

    Some(Answer {
        identity,
        session: None,
        at_ms,
        model: message.model,
        provider: None,
        project: None,
        tokens,
        cost: None,
    })
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::{
        AnswerCost, ModelPrice, PricedCost, TokenPrices,
    };

    use super::*;
    use crate::provider::ANTHROPIC;
    use crate::testing::{as_part, host, path};

    /// 2026-11-16T10:00:00Z.
    const TEN: i64 = 1_794_823_200_000;

    fn assistant(id: &str, request: &str, time: &str, usage: &str) -> String {
        format!(
            r#"{{"type":"assistant","cwd":"/home/dev/otty","sessionId":"s1","requestId":"{request}","timestamp":"{time}","message":{{"id":"{id}","model":"claude-opus-4-5","role":"assistant","content":[{{"type":"text","text":"Done."}}],"usage":{usage}}}}}"#
        )
    }

    /// `record` with the version of Claude Code `version` that wrote it.
    fn with_version(record: &str, version: &str) -> String {
        record.replacen('{', &format!(r#"{{"version":"{version}","#), 1)
    }

    fn tokens(
        input: u64,
        cached: u64,
        written: u64,
        output: u64,
    ) -> TokenCount {
        TokenCount {
            input,
            cached,
            cache_written: written,
            output,
            reasoning: 0,
        }
    }

    fn spend_of(lines: &[&str]) -> Answers {
        let mut file = SpendFile::default();
        for line in lines {
            file.add_line(line.as_bytes());
        }

        file.into_answers("/fallback", None, None)
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 3: Claude
    /// Code writes no cost, so each answer is priced at the API price OTTY
    /// gives for its model and provider, and marked so.
    #[test]
    fn each_answer_is_priced_at_the_price_otty_gives() {
        let anthropic = ProviderRef::Id(ANTHROPIC.to_string());
        let price = ModelPrice {
            standard: TokenPrices {
                input: 3_000_000,
                cached: 300_000,
                cache_written: 3_750_000,
                output: 15_000_000,
            },
            long_prompt: None,
        };
        let host = host().with_price("claude-opus-4-5", None, price);
        let usage = r#"{"input_tokens":1000,"output_tokens":100}"#;
        let mut file = SpendFile::default();
        file.add_line(
            assistant("msg_1", "req_1", "2026-11-16T10:00:00.000Z", usage)
                .as_bytes(),
        );

        let priced = as_part(&host, || {
            file.into_answers("/fallback", Some(&anthropic), None)
        });

        // 1,000 input at $3 and 100 output at $15 a million tokens, each
        // kind apart next to the total (issue 16 of the spec).
        assert_eq!(
            priced.answers[0].cost,
            Some(AnswerCost::AtApiPrices(PricedCost {
                total: 3_000 + 1_500,
                input: 3_000,
                cache_written: 0,
                cached: 0,
                output: 1_500,
            }))
        );
    }

    #[test]
    fn a_model_with_no_price_has_no_cost() {
        let usage = r#"{"input_tokens":1000,"output_tokens":100}"#;
        let line =
            assistant("msg_1", "req_1", "2026-11-16T10:00:00.000Z", usage);

        let spend = as_part(&host(), || spend_of(&[&line]));

        assert_eq!(spend.answers[0].cost, None);
    }

    #[test]
    fn each_answer_gives_its_time_model_and_tokens() {
        let usage = r#"{"input_tokens":12,"cache_creation_input_tokens":100,"cache_read_input_tokens":2000,"output_tokens":30}"#;
        let user = r#"{"type":"user","cwd":"/home/dev/otty","timestamp":"2026-11-16T09:59:00.000Z","message":{"role":"user","content":"Fix it"}}"#;
        let first =
            assistant("msg_1", "req_1", "2026-11-16T10:00:00.000Z", usage);
        let second = assistant(
            "msg_2",
            "req_2",
            "2026-11-16T10:01:00.000Z",
            r#"{"input_tokens":1,"output_tokens":2}"#,
        );

        let spend = spend_of(&[user, &first, &second]);

        let answers = spend.answers;
        assert_eq!(answers.len(), 2);
        assert_eq!(answers[0].project.as_deref(), Some("/home/dev/otty"));
        assert_eq!(answers[0].at_ms, TEN);
        assert_eq!(answers[0].model.as_deref(), Some("claude-opus-4-5"));
        assert_eq!(answers[0].tokens, tokens(112, 2000, 100, 30));
        assert_eq!(answers[1].tokens, tokens(1, 0, 0, 2));
        assert_ne!(answers[0].identity, answers[1].identity);
    }

    #[test]
    fn parts_of_one_answer_share_its_identity() {
        let time = "2026-11-16T10:00:00.000Z";
        let part = assistant("msg_1", "req_1", time, r#"{"output_tokens":1}"#);
        let whole = assistant("msg_1", "req_1", time, r#"{"output_tokens":9}"#);

        let answers = spend_of(&[&part, &whole]).answers;

        assert_eq!(answers.len(), 2);
        assert!(answers[0].identity.is_some());
        assert_eq!(answers[0].identity, answers[1].identity);
    }

    #[test]
    fn with_no_request_id_the_session_and_message_ids_tell_the_answer() {
        let line = |session: &str, id: &str| {
            format!(
                r#"{{"type":"assistant","sessionId":"{session}","timestamp":"2026-11-16T10:00:00Z","message":{{"id":"{id}","usage":{{"output_tokens":3}}}}}}"#
            )
        };
        let no_ids = r#"{"type":"assistant","timestamp":"2026-11-16T10:00:00Z","message":{"usage":{"output_tokens":3}}}"#;

        let answers = spend_of(&[
            &line("s1", "msg_1"),
            &line("s1", "msg_1"),
            &line("s2", "msg_1"),
            no_ids,
        ])
        .answers;

        assert!(answers[0].identity.is_some());
        assert_eq!(answers[0].identity, answers[1].identity);
        assert_ne!(answers[0].identity, answers[2].identity);
        assert_eq!(answers[3].identity, None);
        assert_eq!(answers[3].model, None);
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

        let answers = spend_of(&lines).answers;

        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].tokens, tokens(0, 0, 0, 5));
    }

    #[test]
    fn each_answer_is_a_record_understood_with_the_version_of_the_agent() {
        let usage = r#"{"input_tokens":1,"output_tokens":2}"#;
        let user = r#"{"type":"user","version":"2.1.300","cwd":"/home/dev/otty","timestamp":"2026-11-16T09:59:00.000Z","message":{"role":"user","content":"Fix it"}}"#;
        let zero = r#"{"input_tokens":0,"output_tokens":0}"#;
        let first = with_version(
            &assistant("msg_1", "req_1", "2026-11-16T10:00:00Z", usage),
            "2.1.327",
        );
        let second = with_version(
            &assistant("msg_2", "req_2", "2026-11-16T10:01:00Z", zero),
            "2.1.328",
        );

        let records = spend_of(&[user, &first, &second]).records;

        assert_eq!((records.seen, records.understood), (2, 2));
        assert_eq!(records.version.as_deref(), Some("2.1.328"));
    }

    /// An answer whose fields were renamed is seen but not understood; the
    /// names of its fields are kept, never their values.
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

        let records = spend.records;
        assert_eq!((records.seen, records.understood), (5, 0));
        assert_eq!(records.version.as_deref(), Some("2.2.0"));
        assert_eq!(
            records.missing,
            ["message.usage", "timestamp", "JSON record"]
        );
        for field in ["message.tokenUsage", "message.usage.inputTokens", "at"] {
            assert!(
                records.fields.iter().any(|found| found == field),
                "{field}"
            );
        }
        assert!(spend.answers.is_empty());
    }

    #[test]
    fn a_file_with_no_folder_uses_the_fallback_project() {
        let spend = spend_of(&[
            r#"{"type":"assistant","cwd":"","timestamp":"2026-11-16T10:00:00Z","message":{"usage":{"output_tokens":3}}}"#,
        ]);

        assert_eq!(spend.answers.len(), 1);
        assert_eq!(spend.answers[0].project.as_deref(), Some("/fallback"));
    }

    #[test]
    fn a_session_file_is_read_through_the_host_with_its_provider() {
        let host = host();
        let folder = path(&host, ".claude");
        let session = path(
            &host,
            ".claude/projects/-home-dev-shop/1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b.jsonl",
        );
        let anthropic = ProviderRef::Id(ANTHROPIC.to_string());

        let spend = as_part(&host, || {
            read_answers(&folder, &session, Some(&anthropic))
        })
        .expect("the answers");

        assert_eq!(spend.answers.len(), 3);
        assert_eq!((spend.records.seen, spend.records.understood), (3, 3));
        assert_eq!(spend.records.version.as_deref(), Some("2.1.328"));
        for answer in &spend.answers {
            assert_eq!(answer.project.as_deref(), Some("/home/dev/shop"));
            assert_eq!(answer.provider.as_ref(), Some(&anthropic));
            assert_eq!(
                answer.session.as_deref(),
                Some("1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b")
            );
        }
    }

    #[test]
    fn a_helper_file_is_read_like_a_session_file() {
        let host = host();
        let folder = path(&host, ".claude");
        let helper = path(
            &host,
            ".claude/projects/-home-dev-shop/1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b/subagents/agent-a1b2c3.jsonl",
        );

        let spend = as_part(&host, || read_answers(&folder, &helper, None))
            .expect("the answers");

        assert_eq!(spend.answers.len(), 1);
        assert_eq!(spend.answers[0].project.as_deref(), Some("/home/dev/shop"));
        // The answers of a helper agent count in the session that ran it.
        assert_eq!(
            spend.answers[0].session.as_deref(),
            Some("1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b")
        );
    }

    #[test]
    fn a_file_that_can_not_be_read_is_an_error() {
        let host = host();
        let folder = path(&host, ".claude");
        let gone = path(&host, ".claude/projects/-home-dev-shop/gone.jsonl");

        let read = as_part(&host, || read_answers(&folder, &gone, None));

        assert!(matches!(read, Err(ReadError::Host(_))));
    }
}
