//! One session file of the made-up agent: JSON lines with one `meta`
//! record, the user's messages, and the model's `answer`s.

use otty_plugin_sdk::agents::{
    Answer, Answers, ProviderRef, ReadError, RecordCounts, TokenCount,
};
use otty_plugin_sdk::files;
use serde_json::Value;

/// What marks the lines the part reads; the host leaves out the others.
const MARKERS: [&[u8]; 2] = [b"\"type\":\"meta\"", b"\"type\":\"answer\""];

/// The provider every answer of the made-up agent is for.
const PROVIDER: &str = "example-cloud";

/// What a session file said: its title and project from the `meta`
/// record, its answers, and the records seen.
#[derive(Debug, Default)]
pub(crate) struct SessionFile {
    title: Option<String>,
    project: Option<String>,
    last_at_ms: Option<i64>,
    answers: Vec<Answer>,
    records: RecordCounts,
}

impl SessionFile {
    /// The title the agent gave the session.
    pub(crate) fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The project folder the session ran in.
    pub(crate) fn project(&self) -> Option<&str> {
        self.project.as_deref()
    }

    /// When the newest answer was written.
    pub(crate) fn last_at_ms(&self) -> Option<i64> {
        self.last_at_ms
    }

    /// The file at `path`, read through the host.
    pub(crate) fn read(path: &str) -> Result<Self, ReadError> {
        let mut file = Self::default();

        for batch in files::lines(path, &MARKERS) {
            let batch = batch.map_err(ReadError::Host)?;
            for line in batch.lines() {
                file.add_line(line);
            }
        }

        Ok(file)
    }

    /// The answers, each in the session `session`, and what was seen, for
    /// OTTY.
    pub(crate) fn into_answers(self, session: &str) -> Answers {
        let answers = self
            .answers
            .into_iter()
            .map(|answer| Answer {
                session: Some(session.to_string()),
                ..answer
            })
            .collect();

        Answers {
            answers,
            records: self.records,
        }
    }

    fn add_line(&mut self, line: &[u8]) {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            self.records.add_broken();
            return;
        };

        match record.get("type").and_then(Value::as_str) {
            Some("meta") => self.add_meta(&record),
            Some("answer") => self.add_answer(&record),
            _ => {},
        }
    }

    fn add_meta(&mut self, record: &Value) {
        let text = |field: &str| record.get(field).and_then(Value::as_str);

        if let Some(version) = text("version") {
            self.records.saw_version(version);
        }
        self.title = text("title").map(str::to_string);
        self.project = text("cwd").map(str::to_string);
    }

    fn add_answer(&mut self, record: &Value) {
        let at_ms = record.get("at_ms").and_then(Value::as_i64);
        let tokens = record.get("tokens").and_then(token_count);
        let (Some(at_ms), Some(tokens)) = (at_ms, tokens) else {
            let missing = if at_ms.is_none() { "at_ms" } else { "tokens" };
            let found = record
                .as_object()
                .into_iter()
                .flat_map(|fields| fields.keys());
            self.records.add_not_understood(
                missing,
                "",
                found.map(String::as_str),
            );
            return;
        };

        self.records.add_understood();
        self.last_at_ms = self.last_at_ms.max(Some(at_ms));
        self.answers.push(Answer {
            identity: record
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string),
            session: None,
            at_ms,
            model: record
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string),
            provider: Some(ProviderRef::Id(PROVIDER.to_string())),
            login: None,
            project: self.project.clone(),
            tokens,
            cost_microdollars: None,
        });
    }
}

/// The tokens of an answer: `input`, `cached` and `output`, all needed.
fn token_count(tokens: &Value) -> Option<TokenCount> {
    let count = |field: &str| tokens.get(field).and_then(Value::as_u64);

    Some(TokenCount {
        input: count("input")?,
        cached: count("cached")?,
        output: count("output")?,
        ..TokenCount::default()
    })
}
