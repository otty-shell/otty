//! Reads the answers of the model and their tokens from one Codex session
//! file, plain or compressed: the host decodes a compressed file and hands
//! out only the lines that name a record read here.
//!
//! After each answer Codex writes an `event_msg` record whose payload is a
//! `token_count` with `info.last_token_usage` (this answer) and
//! `info.total_token_usage` (the whole session so far), each with
//! `input_tokens`, `cached_input_tokens`, `output_tokens` and
//! `reasoning_output_tokens`. The input includes the cached part and the
//! output the reasoning. The model is named by the `turn_context` record
//! that starts each turn, and the project by the `session_meta` record on
//! the first line. So what a line means depends on the lines before it:
//! that state lives in [`SessionTokens`] for the whole file.
//!
//! The rules for the same answer written more than once:
//!
//! - Codex can write the same `token_count` twice in a row. A record whose
//!   session total did not change since the previous one is skipped.
//! - The same record can be in two files: the plain and the compressed
//!   copy of a session, or a session and its archived copy, or the history
//!   a forked session copies. Each answer carries the identity of its
//!   record (its time and its numbers), so OTTY counts the copies once.
//!
//! Reading is tolerant: a broken line, a `token_count` with no numbers or
//! with no time is skipped. The `token_count` records are counted as seen
//! and understood, so a changed format is told and not taken for fewer
//! tokens.

use otty_plugin_sdk::agents::{
    Answer, Answers, ProviderRef, ReadError, RecordCounts, TokenCount,
    answer_cost,
};
use otty_plugin_sdk::files;
use serde::Deserialize;
use serde_json::Value;

use crate::timestamp::millis_of;

/// What marks the lines the tokens are read from: the answers, the turns
/// that name the model and the first line that names the project.
const ANSWER_MARKERS: [&[u8]; 3] =
    [b"\"token_count\"", b"\"turn_context\"", b"\"session_meta\""];

/// What marks a line that names a `token_count`.
const TOKEN_COUNT_MARKER: &[u8] = b"\"token_count\"";

/// The id of OpenAI, Codex's default provider.
const OPENAI: &str = "openai";

/// The counts of a usage OTTY reads; a usage with none of them is not
/// understood.
const USAGE_COUNTS: [&str; 3] =
    ["input_tokens", "cached_input_tokens", "output_tokens"];

/// One line of a session file: its time, its type and its payload.
#[derive(Debug, Deserialize)]
pub(crate) struct Line {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    payload: Option<Value>,
}

impl Line {
    /// The time the line was written, as Codex wrote it.
    pub(crate) fn timestamp(&self) -> Option<&str> {
        self.timestamp.as_deref()
    }

    /// The type of the record, such as `event_msg`.
    pub(crate) fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// The payload of the record.
    pub(crate) fn payload(&self) -> Option<&Value> {
        self.payload.as_ref()
    }
}

/// The tokens of a `token_count` record, as Codex writes them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    reasoning_output_tokens: u64,
}

impl Usage {
    /// The tokens in the shape of the contract: the input without its
    /// cached part.
    fn tokens(&self) -> TokenCount {
        TokenCount {
            input: self.input_tokens.saturating_sub(self.cached_input_tokens),
            cached: self.cached_input_tokens,
            output: self.output_tokens,
            reasoning: self.reasoning_output_tokens,
            ..TokenCount::default()
        }
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
            reasoning_output_tokens: self
                .reasoning_output_tokens
                .saturating_sub(previous.reasoning_output_tokens),
        }
    }
}

/// What the lines of one session file told so far.
#[derive(Debug, Default)]
pub(crate) struct SessionTokens {
    project: Option<String>,
    /// Whether the first `session_meta`, the session's own, was read.
    has_meta: bool,
    /// The id of the session, from its own `session_meta`.
    session: Option<String>,
    /// The id of the provider of the session, such as `openai`.
    model_provider: Option<String>,
    model: Option<String>,
    previous_total: Option<Usage>,
    answers: Vec<Answer>,
    records: RecordCounts,
}

impl SessionTokens {
    /// Takes what the line `line` tells. A line that names a
    /// `token_count` but is not JSON, such as the last line of a file
    /// Codex is still writing, is a record not understood.
    pub(crate) fn add_line(&mut self, line: &[u8]) {
        match serde_json::from_slice::<Line>(line) {
            Ok(parsed) => self.add(&parsed),
            Err(_) if has_marker(line, TOKEN_COUNT_MARKER) => {
                self.records.add_not_understood("JSON record", "", []);
            },
            Err(_) => {},
        }
    }

    /// Takes what `line` tells: the project, the session, the provider and
    /// the version of `session_meta`, the model of `turn_context`, the
    /// answer of `token_count`.
    ///
    /// The session and the provider come from the first `session_meta`
    /// only: a forked session copies the records of its parent after its
    /// own.
    pub(crate) fn add(&mut self, line: &Line) {
        let Some(payload) = &line.payload else {
            return;
        };

        if line.kind() == Some("session_meta") && !self.has_meta {
            self.has_meta = true;
            self.session = filled_text(payload, "id");
            self.model_provider = filled_text(payload, "model_provider");
            if let Some(version) = filled_text(payload, "cli_version") {
                self.records.saw_version(&version);
            }
        }

        match line.kind() {
            Some("session_meta") if self.project.is_none() => {
                self.project = filled_text(payload, "cwd");
            },
            Some("turn_context") => {
                if let Some(model) =
                    payload.get("model").and_then(Value::as_str)
                {
                    self.model = Some(model.to_string());
                }
            },
            Some("event_msg") => {
                self.add_token_count(line.timestamp(), payload);
            },
            _ => {},
        }
    }

    /// What the lines told: each answer with the session, the project and
    /// the provider of the session, and the records seen. Codex writes no
    /// cost, so each answer is priced at the API price OTTY gives for its
    /// model and provider (spec 2026-11-23-accounts-apart-from-profiles,
    /// Decision 3).
    pub(crate) fn into_answers(self) -> Answers {
        let provider = provider_of(self.model_provider.as_deref());

        let answers = self
            .answers
            .into_iter()
            .map(|answer| {
                let cost = answer_cost(
                    None,
                    answer.model.as_deref(),
                    provider.as_ref(),
                    &answer.tokens,
                );

                Answer {
                    session: self.session.clone(),
                    project: self.project.clone(),
                    provider: provider.clone(),
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

    /// Takes the answer of a `token_count` payload written at `timestamp`,
    /// and counts the record as understood or not.
    ///
    /// The tokens of the answer are `last_token_usage`; with none, how much
    /// `total_token_usage` grew since the previous record. A record whose
    /// total did not change is a repeat and is skipped. A record whose
    /// `info` is `null`, the limits alone, has nothing to count; a record
    /// with no `info` at all is not understood.
    fn add_token_count(&mut self, timestamp: Option<&str>, payload: &Value) {
        if payload.get("type").and_then(Value::as_str) != Some("token_count") {
            return;
        }
        let Some(info) = payload.get("info") else {
            self.records.add_not_understood(
                "payload.info",
                "payload",
                field_names(payload),
            );
            return;
        };
        if info.is_null() {
            self.records.add_understood();
            return;
        }

        let last = usage(info, "last_token_usage");
        let total = usage(info, "total_token_usage");
        let (Some(last), Some(total)) = (last, total) else {
            self.add_no_usage(info);
            return;
        };
        if last.is_none() && total.is_none() {
            self.add_no_usage(info);
            return;
        }
        let at_ms = timestamp.and_then(millis_of);
        match at_ms {
            Some(_) => self.records.add_understood(),
            None => self.records.add_not_understood("timestamp", "", []),
        }

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

        let Some(at_ms) = at_ms else {
            return;
        };
        let tokens = usage.tokens();
        if tokens == TokenCount::default() {
            return;
        }

        self.answers.push(Answer {
            identity: Some(identity(
                timestamp.unwrap_or_default(),
                last,
                total,
            )),
            session: None,
            at_ms,
            model: self.model.clone(),
            provider: None,
            project: None,
            tokens,
            cost: None,
        });
    }

    /// Counts a `token_count` record whose `info` has no usage OTTY reads
    /// as not understood, with the names of the fields of `info`.
    fn add_no_usage(&mut self, info: &Value) {
        self.records.add_not_understood(
            "payload.info.last_token_usage",
            "payload.info",
            field_names(info),
        );
    }
}

/// The answers of the session file at `path`, each priced, with the
/// records seen. Nothing ties an answer to the login that paid for it
/// (spec 2026-11-23-accounts-apart-from-profiles, Decision 1).
pub(crate) fn read_profile_answers(path: &str) -> Result<Answers, ReadError> {
    Ok(read_session(path)?.into_answers())
}

/// What the lines of the session file at `path` tell.
///
/// A file that can not be read fails; a file that breaks in the middle
/// gives what was read before.
fn read_session(path: &str) -> Result<SessionTokens, ReadError> {
    let mut session = SessionTokens::default();

    for (number, batch) in files::lines(path, &ANSWER_MARKERS).enumerate() {
        let batch = match batch {
            Ok(batch) => batch,
            Err(error) if number == 0 => return Err(ReadError::Host(error)),
            Err(_) => break,
        };
        for line in batch.lines() {
            session.add_line(line);
        }
    }

    Ok(session)
}

/// Whether `line` holds `marker`.
fn has_marker(line: &[u8], marker: &[u8]) -> bool {
    line.windows(marker.len()).any(|window| window == marker)
}

/// The provider of a session from the id of its `model_provider`: OpenAI
/// for `openai` and for a session that names none, which runs on Codex's
/// default provider. Any other id is a provider of the user's own config,
/// such as a local model, and is not known.
fn provider_of(model_provider: Option<&str>) -> Option<ProviderRef> {
    match model_provider {
        None | Some(OPENAI) => Some(ProviderRef::Id(OPENAI.to_string())),
        Some(_) => None,
    }
}

/// What tells the answer of a record apart: its time and its numbers,
/// never its text.
fn identity(
    timestamp: &str,
    last: Option<Usage>,
    total: Option<Usage>,
) -> String {
    let numbers = |usage: Option<Usage>| match usage {
        Some(usage) => format!(
            "{},{},{},{}",
            usage.input_tokens,
            usage.cached_input_tokens,
            usage.output_tokens,
            usage.reasoning_output_tokens
        ),
        None => String::from("-"),
    };

    format!("{timestamp}/{}/{}", numbers(last), numbers(total))
}

/// The text in the field `field` of `payload`, when it is filled.
fn filled_text(payload: &Value, field: &str) -> Option<String> {
    let text = payload.get(field).and_then(Value::as_str)?.trim();

    (!text.is_empty()).then(|| text.to_string())
}

/// The names of the fields of `value`, an object; none for another value.
fn field_names(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .map(|fields| fields.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

/// The usage in the field `field` of `info`: `Some(None)` when the field is
/// missing or `null`, `None` when it is there but not a usage, such as a
/// usage with none of the counts OTTY reads, which makes the whole record
/// broken.
fn usage(info: &Value, field: &str) -> Option<Option<Usage>> {
    match info.get(field) {
        None | Some(Value::Null) => Some(None),
        Some(value)
            if USAGE_COUNTS.iter().all(|count| value.get(count).is_none()) =>
        {
            None
        },
        Some(value) => Usage::deserialize(value).ok().map(Some),
    }
}

/// Test-only values of the tokens of a session file.
#[cfg(test)]
pub(crate) mod testing {
    use otty_plugin_sdk::agents::TokenCount;

    /// The tokens `input`, `cached` and `output`, with no reasoning.
    pub(crate) fn tokens(input: u64, cached: u64, output: u64) -> TokenCount {
        TokenCount {
            input,
            cached,
            output,
            ..TokenCount::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::HostError;
    use otty_plugin_sdk::agents::{
        AnswerCost, ModelPrice, PricedCost, TokenPrices,
    };

    use super::testing::tokens;
    use super::*;
    use crate::testing::{BLOG_SESSION, TEN, as_part, host, path};

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

    /// The answers of a file of `lines`, as the host hands them out: only
    /// the lines with a marker.
    fn spend_of(lines: &[&str]) -> Answers {
        let mut session = SessionTokens::default();
        for line in lines {
            let line = line.as_bytes();
            if ANSWER_MARKERS.iter().any(|marker| has_marker(line, marker)) {
                session.add_line(line);
            }
        }

        session.into_answers()
    }

    fn tokens_of(spend: &Answers) -> Vec<TokenCount> {
        spend.answers.iter().map(|answer| answer.tokens).collect()
    }

    fn openai() -> Option<ProviderRef> {
        Some(ProviderRef::Id(OPENAI.to_string()))
    }

    /// The session meta of a session served by `provider` and paid by
    /// `account`, each left out when `None`.
    fn meta_of(provider: Option<&str>, account: Option<&str>) -> String {
        let provider = provider
            .map(|provider| format!(r#","model_provider":"{provider}""#))
            .unwrap_or_default();
        let account = account
            .map(|account| format!(r#","creator_account_id":"{account}""#))
            .unwrap_or_default();

        format!(
            r#"{{"timestamp":"2026-11-16T09:00:00.000Z","type":"session_meta","payload":{{"id":"019a8f3e","cwd":"/home/user/otty","source":"cli","cli_version":"0.80.0"{provider}{account}}}}}"#
        )
    }

    fn fixture_answers(relative: &str) -> Result<Answers, ReadError> {
        let host = host();
        let file = path(&host, relative);

        as_part(&host, || read_profile_answers(&file))
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 3: Codex
    /// writes no cost, so each answer is priced at the API price OTTY
    /// gives for its model and provider, and marked so.
    #[test]
    fn each_answer_is_priced_at_the_price_otty_gives() {
        let price = ModelPrice {
            standard: TokenPrices {
                input: 1_000_000,
                cached: 100_000,
                cache_written: 0,
                output: 10_000_000,
            },
            long_prompt: None,
        };
        let host = host().with_price("gpt-5-codex", openai(), price);
        let meta = meta_of(Some("openai"), None);
        let turn = turn("gpt-5-codex");
        let count =
            count("2026-11-16T10:00:00.000Z", (1000, 800, 50), (1000, 800, 50));

        let spend = as_part(&host, || spend_of(&[&meta, &turn, &count]));

        // 200 input at $1, 800 cached at $0.10 and 50 output at $10 a
        // million tokens, each kind apart next to the total (issue 16 of
        // the spec).
        assert_eq!(
            spend.answers[0].cost,
            Some(AnswerCost::AtApiPrices(PricedCost {
                total: 200 + 80 + 500,
                input: 200,
                cache_written: 0,
                cached: 80,
                output: 500,
            }))
        );
    }

    #[test]
    fn a_model_with_no_price_has_no_cost() {
        let host = host();
        let meta = meta_of(Some("openai"), None);
        let turn = turn("gpt-5-codex");
        let count =
            count("2026-11-16T10:00:00.000Z", (1000, 800, 50), (1000, 800, 50));

        let spend = as_part(&host, || spend_of(&[&meta, &turn, &count]));

        assert_eq!(spend.answers[0].cost, None);
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

        assert_eq!(
            tokens_of(&spend),
            [tokens(200, 800, 50), tokens(200, 100, 20)]
        );
        let answers = &spend.answers;
        assert_eq!(answers[0].project.as_deref(), Some("/home/user/otty"));
        assert_eq!(answers[0].session.as_deref(), Some("019a8f3e"));
        assert_eq!(answers[0].at_ms, TEN);
        assert_eq!(answers[0].model.as_deref(), Some("gpt-5-codex"));
        assert_eq!(answers[1].model.as_deref(), Some("gpt-5.1-codex"));
        assert!(answers[0].identity.is_some());
        assert_ne!(answers[0].identity, answers[1].identity);
    }

    #[test]
    fn a_record_written_again_with_the_same_total_is_skipped() {
        let first = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));
        let again = count("2026-11-16T10:00:01Z", (10, 0, 5), (10, 0, 5));
        let next = count("2026-11-16T10:02:00Z", (10, 0, 5), (20, 0, 10));

        let spend = spend_of(&[META, &first, &again, &next]);

        assert_eq!(spend.answers.len(), 2);
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

        assert_eq!(tokens_of(&spend), [tokens(60, 40, 5), tokens(0, 50, 3)]);
    }

    #[test]
    fn a_compressed_file_is_read() {
        let spend = fixture_answers(&format!(
            ".codex/sessions/2026/11/15/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl.zst"
        ))
        .expect("the answers");

        assert_eq!(
            tokens_of(&spend),
            [tokens(500, 0, 40), tokens(100, 300, 25)]
        );
        assert_eq!(spend.answers[0].model.as_deref(), Some("o3"));
        assert_eq!(spend.answers[0].project.as_deref(), Some("/home/dev/blog"));
        assert_eq!(spend.records.version.as_deref(), Some("0.168.0"));
    }

    /// The host decodes a `.zst` file: it gives the same answers, with
    /// the same identities, as its plain copy.
    #[test]
    fn a_compressed_file_gives_what_its_plain_copy_gives() {
        let compressed = fixture_answers(&format!(
            ".codex/sessions/2026/11/15/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl.zst"
        ));
        let plain = fixture_answers(&format!(
            ".codex/archived_sessions/rollout-2026-11-15T09-00-00-{BLOG_SESSION}.jsonl"
        ));

        assert_eq!(compressed, plain);
    }

    #[test]
    fn the_reasoning_is_the_share_of_the_output_spent_thinking() {
        let host = host();
        let file = path(
            &host,
            ".codex/sessions/2026/11/16/rollout-2026-11-16T10-00-00-019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b.jsonl",
        );

        let spend = as_part(&host, || read_profile_answers(&file))
            .expect("the answers");

        let reasoning: Vec<u64> = spend
            .answers
            .iter()
            .map(|answer| answer.tokens.reasoning)
            .collect();
        assert_eq!(reasoning, [20, 5]);
        assert_eq!((spend.records.seen, spend.records.understood), (3, 3));
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

        assert_eq!(tokens_of(&spend), [tokens(10, 0, 5)]);
    }

    #[test]
    fn each_token_count_is_a_record_understood_with_the_version_of_codex() {
        let limits_only = r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":null,"rate_limits":{}}}"#;
        let first = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));
        let again = count("2026-11-16T10:00:01Z", (10, 0, 5), (10, 0, 5));
        let message = r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"agent_message","message":"token_count"}}"#;

        let spend = spend_of(&[
            &meta_of(None, None),
            limits_only,
            &first,
            &again,
            message,
        ]);

        let records = &spend.records;
        assert_eq!((records.seen, records.understood), (3, 3));
        assert_eq!(records.version.as_deref(), Some("0.80.0"));
    }

    /// A `token_count` whose fields were renamed is seen but not
    /// understood; the names of its fields are kept, never their values.
    #[test]
    fn token_counts_with_renamed_fields_are_seen_but_not_understood() {
        let renamed_info = r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"token_count","usage_info":{"last":{"input_tokens":5}}}}"#;
        let renamed_usage = r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"last_usage":{"input_tokens":5}}}}"#;
        let renamed_tokens = r#"{"timestamp":"2026-11-16T10:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"inputTokens":5}}}}"#;
        let no_time = count("not a time", (1, 0, 1), (1, 0, 1));
        let broken = r#"{"type":"event_msg","payload":{"type":"token_count""#;

        let spend = spend_of(&[
            META,
            renamed_info,
            renamed_usage,
            renamed_tokens,
            &no_time,
            broken,
        ]);

        let records = &spend.records;
        assert_eq!((records.seen, records.understood), (5, 0));
        let mut missing = records.missing.clone();
        missing.sort();
        assert_eq!(
            missing,
            [
                "JSON record",
                "payload.info",
                "payload.info.last_token_usage",
                "timestamp"
            ]
        );
        for field in [
            "payload.usage_info",
            "payload.info.last_usage",
            "payload.info.last_token_usage",
        ] {
            assert!(records.fields.iter().any(|name| name == field), "{field}");
        }
        assert!(spend.answers.is_empty());
    }

    #[test]
    fn a_file_with_no_session_meta_has_no_project_and_no_model() {
        let line = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));

        let spend = spend_of(&[&line]);

        assert_eq!(spend.answers[0].project, None);
        assert_eq!(spend.answers[0].session, None);
        assert_eq!(spend.answers[0].model, None);
    }

    #[test]
    fn the_session_names_its_provider() {
        let line = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));
        let meta = meta_of(Some("openai"), Some("made-up-account"));

        let spend = spend_of(&[&meta, &line]);

        assert_eq!(spend.answers[0].provider, openai());
    }

    /// Codex uses OpenAI unless its config picks another provider, and
    /// older sessions do not name it.
    #[test]
    fn a_session_that_names_no_provider_is_served_by_openai() {
        let line = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));

        let with_meta = spend_of(&[&meta_of(None, None), &line]);
        let without_meta = spend_of(&[&line]);

        assert_eq!(with_meta.answers[0].provider, openai());
        assert_eq!(without_meta.answers[0].provider, openai());
    }

    /// A provider of the user's own config, such as a local model, is not
    /// one OTTY knows.
    #[test]
    fn another_provider_is_not_known() {
        let line = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));

        let spend = spend_of(&[&meta_of(Some("ollama"), None), &line]);

        assert_eq!(spend.answers[0].provider, None);
    }

    /// A forked session copies the records of its parent after its own
    /// session meta; the first one is the session's.
    #[test]
    fn the_first_session_meta_is_the_sessions() {
        let line = count("2026-11-16T10:00:00Z", (10, 0, 5), (10, 0, 5));
        let own = meta_of(Some("openai"), Some("made-up-own"));
        let parent = meta_of(Some("ollama"), Some("made-up-parent"));

        let parent = parent.replace("019a8f3e", "made-up-parent-session");

        let spend = spend_of(&[&own, &parent, &line]);

        assert_eq!(spend.answers[0].provider, openai());
        assert_eq!(spend.answers[0].session.as_deref(), Some("019a8f3e"));
    }

    #[test]
    fn a_file_that_can_not_be_opened_gives_no_answers() {
        let spend = fixture_answers(".codex/sessions/rollout-gone.jsonl");

        assert!(matches!(
            spend,
            Err(ReadError::Host(HostError::NotFound(_)))
        ));
    }

    #[test]
    fn the_identity_is_the_time_and_the_numbers() {
        let usage = Usage {
            input_tokens: 10,
            cached_input_tokens: 2,
            output_tokens: 5,
            reasoning_output_tokens: 1,
        };

        assert_eq!(
            identity("2026-11-16T10:00:00Z", Some(usage), None),
            "2026-11-16T10:00:00Z/10,2,5,1/-"
        );
    }
}
