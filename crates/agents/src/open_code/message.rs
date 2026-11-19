//! One message of the database read as what OTTY counts: a question of
//! the user, or an answer of a model with its provider, model, tokens and
//! the cost the agent wrote.
//!
//! Both shapes write an answer the same way, apart from where the model
//! is: `providerID` and `modelID` in the first, `model.providerID` and
//! `model.id` in the second. The tokens are `input`, `output`,
//! `reasoning` and `cache.read` and `cache.write`, none of them part of
//! another; the cost is in dollars.

use serde_json::Value;

use super::database::MessageRow;
use crate::dollars::Dollars;
use crate::spend_files::CountedAnswer;
use crate::timestamp::time_from_millis;
use crate::token_count::TokenCount;

/// What one message is to OTTY.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Record {
    /// A message the user wrote.
    Question,
    /// An answer of a model, through the provider the agent names, such
    /// as `openai`.
    Answer {
        /// The provider's id as the agent writes it.
        provider: Option<String>,
        /// When, which model, the tokens and the cost.
        answer: CountedAnswer,
    },
    /// An answer OTTY does not understand: the field it needs that was
    /// missing or of another shape, the field it looked in (empty for the
    /// message itself) and what it found there.
    NotUnderstood {
        /// The field that was missing or of another shape.
        missing: &'static str,
        /// The field it was looked for in.
        place: &'static str,
        /// What was found there.
        found: Option<Value>,
    },
    /// Anything else: an answer still being written or one that failed.
    Other,
}

/// What `row` is to OTTY.
pub(super) fn read_record(row: &MessageRow) -> Record {
    let Ok(data) = serde_json::from_str::<Value>(&row.data) else {
        return not_understood("JSON record", "", None);
    };

    let role = row
        .kind
        .as_deref()
        .or_else(|| data.get("role").and_then(Value::as_str));
    match role {
        Some("assistant") => {},
        Some("user") => return Record::Question,
        Some(_) => return Record::Other,
        None => return not_understood("role", "", Some(&data)),
    }

    let tokens = data.get("tokens");
    if tokens.is_none() && !is_finished(&data) {
        return Record::Other;
    }
    let Some(tokens) = tokens.and_then(token_count) else {
        return not_understood("tokens", "tokens", tokens.or(Some(&data)));
    };
    let Some(at) = time_from_millis(row.at_ms as f64) else {
        return not_understood("time_created", "", None);
    };

    let model =
        text_at(&data, "modelID").or_else(|| text_at(&data, "/model/id"));
    let provider = text_at(&data, "providerID")
        .or_else(|| text_at(&data, "/model/providerID"));
    let cost = data
        .get("cost")
        .and_then(Value::as_f64)
        .and_then(Dollars::from_dollars);

    let answer = CountedAnswer::new(at, model, tokens)
        .with_cost(cost)
        .with_identity(&row.id);

    Record::Answer {
        provider: provider.map(String::from),
        answer,
    }
}

/// Whether the answer `data` is finished: it says why it stopped or when
/// it was completed, or that it failed.
fn is_finished(data: &Value) -> bool {
    data.get("finish").is_some()
        || data.get("error").is_some()
        || data.pointer("/time/completed").is_some()
}

/// The tokens of an answer in OTTY's terms: the reasoning is part of the
/// output, as OpenAI counts it, and the input written to the cache is part
/// of the input. `None` when `input` or `output` is not a number.
fn token_count(tokens: &Value) -> Option<TokenCount> {
    let input = count_at(tokens, "/input")?;
    let output = count_at(tokens, "/output")?;
    let reasoning = count_at(tokens, "/reasoning").unwrap_or(0);
    let read = count_at(tokens, "/cache/read").unwrap_or(0);
    let written = count_at(tokens, "/cache/write").unwrap_or(0);

    let count = TokenCount::new(
        input.saturating_add(written),
        read,
        output.saturating_add(reasoning),
    );

    Some(count.with_cache_written(written))
}

/// The count at `pointer` of `value`: a whole number that is not below
/// zero, which the agents may write as a decimal number.
fn count_at(value: &Value, pointer: &str) -> Option<u64> {
    let number = value.pointer(pointer)?;

    number.as_u64().or_else(|| {
        number
            .as_f64()
            .filter(|count| count.is_finite() && *count >= 0.0)
            .map(|count| count.round() as u64)
    })
}

/// The text at `pointer` of `data`, or at its key `pointer` when it is a
/// plain key; `None` when it is not a text that is filled.
fn text_at<'a>(data: &'a Value, pointer: &str) -> Option<&'a str> {
    let value = if pointer.starts_with('/') {
        data.pointer(pointer)
    } else {
        data.get(pointer)
    };

    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

/// An answer not understood because of `missing`, looked for in `place`
/// of what was `found`.
fn not_understood(
    missing: &'static str,
    place: &'static str,
    found: Option<&Value>,
) -> Record {
    Record::NotUnderstood {
        missing,
        place,
        found: found.cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_code::testing::{
        first_shape_answer, first_shape_question, millis, second_shape_answer,
    };
    use crate::testing::{NOW, at};

    /// 10 input, 2 output, 1 reasoning, 300 read from the cache and 5
    /// written to it.
    const TOKENS: (u64, u64, u64, u64, u64) = (10, 2, 1, 300, 5);

    fn row(kind: Option<&str>, data: &str) -> MessageRow {
        MessageRow {
            id: String::from("msg_1"),
            session_id: String::from("ses_1"),
            kind: kind.map(String::from),
            at_ms: millis(NOW),
            data: data.to_string(),
        }
    }

    fn expected_answer(model: &str) -> CountedAnswer {
        let tokens = TokenCount::new(15, 300, 3).with_cache_written(5);

        CountedAnswer::new(at(NOW), Some(model), tokens)
            .with_cost(Some(Dollars::from_micro_dollars(250_000)))
            .with_identity(&"msg_1")
    }

    #[test]
    fn an_answer_of_the_first_shape_has_its_provider_tokens_and_cost() {
        let data = first_shape_answer("openai", "gpt-5.5", TOKENS, 0.25);

        let record = read_record(&row(None, &data));

        assert_eq!(
            record,
            Record::Answer {
                provider: Some(String::from("openai")),
                answer: expected_answer("gpt-5.5"),
            }
        );
    }

    #[test]
    fn an_answer_of_the_second_shape_has_its_model_in_model() {
        let data = second_shape_answer("anthropic", "claude-x", TOKENS, 0.25);

        let record = read_record(&row(Some("assistant"), &data));

        assert_eq!(
            record,
            Record::Answer {
                provider: Some(String::from("anthropic")),
                answer: expected_answer("claude-x"),
            }
        );
    }

    #[test]
    fn a_question_and_an_unfinished_answer_are_told_apart() {
        let unfinished =
            r#"{"model":{"id":"m","providerID":"p"},"time":{"created":1}}"#;

        assert_eq!(
            read_record(&row(None, &first_shape_question())),
            Record::Question
        );
        assert_eq!(read_record(&row(Some("user"), "{}")), Record::Question);
        assert_eq!(
            read_record(&row(Some("assistant"), unfinished)),
            Record::Other
        );
    }

    #[test]
    fn an_answer_with_renamed_tokens_is_not_understood_and_names_its_fields() {
        let renamed = r#"{"role":"assistant","modelID":"m","providerID":"p",
            "usage":{"in":1,"out":2},"finish":"stop"}"#;
        let renamed_parts = r#"{"role":"assistant","tokens":{"prompt":1},
            "finish":"stop"}"#;

        let whole = read_record(&row(None, renamed));
        let parts = read_record(&row(None, renamed_parts));

        let Record::NotUnderstood { missing, found, .. } = whole else {
            panic!("not understood: {whole:?}");
        };
        assert_eq!(missing, "tokens");
        let found = found.expect("the message is kept for its field names");
        assert!(found.get("usage").is_some());
        let Record::NotUnderstood { place, found, .. } = parts else {
            panic!("not understood: {parts:?}");
        };
        assert_eq!(place, "tokens");
        assert!(
            found
                .and_then(|tokens| tokens.get("prompt").cloned())
                .is_some()
        );
    }

    #[test]
    fn a_message_that_is_not_json_is_not_understood() {
        let record = read_record(&row(Some("assistant"), "{broken"));

        assert!(matches!(
            record,
            Record::NotUnderstood {
                missing: "JSON record",
                ..
            }
        ));
    }
}
