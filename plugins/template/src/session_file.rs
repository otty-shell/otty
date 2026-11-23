//! One session file of the made-up agent: JSON lines with one `meta`
//! record, the user's messages, and the model's `answer`s. Each answer
//! carries its money: the cost the agent wrote in `cost_microdollars`, as
//! it is, else its tokens at the API price OTTY's price list gives.

use otty_plugin_sdk::agents::{
    Answer, Answers, ProviderRef, ReadError, RecordCounts, TokenCount,
    answer_cost,
};
use otty_plugin_sdk::files;
use serde_json::Value;

/// What marks the lines the part reads; the host leaves out the others.
const MARKERS: [&[u8]; 2] = [b"\"type\":\"meta\"", b"\"type\":\"answer\""];

/// Who served every answer of the made-up agent. Name the provider as
/// your agent writes it, even one no plugin serves: OTTY joins the name to
/// the provider part with that id and gives the price of the model that
/// provider serves.
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

        let model = record.get("model").and_then(Value::as_str);
        let provider = ProviderRef::Id(PROVIDER.to_string());
        let written = record.get("cost_microdollars").and_then(Value::as_u64);
        let cost = answer_cost(written, model, Some(&provider), &tokens);

        self.answers.push(Answer {
            identity: record
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string),
            session: None,
            at_ms,
            model: model.map(str::to_string),
            provider: Some(provider),
            project: self.project.clone(),
            tokens,
            cost,
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

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::AnswerCost;

    use super::*;

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 3: a cost
    /// the agent wrote is given as it is, with no price asked.
    #[test]
    fn a_cost_the_agent_wrote_is_given_as_it_is() {
        let mut file = SessionFile::default();

        file.add_line(
            br#"{"type":"answer","id":"c1","at_ms":1,"model":"example-large","tokens":{"input":10,"cached":0,"output":5},"cost_microdollars":420}"#,
        );

        let answers = file.into_answers("s1").answers;
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].cost, Some(AnswerCost::Written(420)));
    }
}
