//! One message of the database read as what OTTY counts: a question of
//! the user, or an answer of a model with its provider as OpenCode names
//! it, model, tokens and the cost OpenCode wrote.
//!
//! Both shapes write an answer the same way, apart from where the model
//! is: `providerID` and `modelID` in the first, `model.providerID` and
//! `model.id` in the second. The tokens are `input`, `output`,
//! `reasoning` and `cache.read` and `cache.write`, none of them part of
//! another; the cost is in dollars.

use otty_plugin_sdk::agents::{Answer, ProviderRef, TokenCount, answer_cost};
use serde_json::Value;

use crate::database::MessageRow;

/// Millionths of a dollar in a dollar.
const MICRODOLLARS_IN_DOLLAR: f64 = 1_000_000.0;

/// What one message is to OTTY.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Record {
    /// A message the user wrote.
    Question,
    /// An answer of a model, with its provider as OpenCode names it; its
    /// project is left for the caller, who knows the session.
    Answer(Box<Answer>),
    /// An answer OTTY does not understand: the field it needs that was
    /// missing or of another shape, the field it looked in (empty for the
    /// message itself) and the names of the fields found there.
    NotUnderstood {
        /// The field that was missing or of another shape.
        missing: &'static str,
        /// The field it was looked for in.
        place: &'static str,
        /// The names of the fields found there, never their values.
        found: Vec<String>,
    },
    /// Anything else: an answer still being written or one that failed.
    Other,
}

/// What `row` is to OTTY.
pub(crate) fn read_record(row: &MessageRow) -> Record {
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
        return match tokens {
            Some(tokens) => not_understood("tokens", "tokens", Some(tokens)),
            None => not_understood("tokens", "", Some(&data)),
        };
    };

    let model =
        text_at(&data, "modelID").or_else(|| text_at(&data, "/model/id"));
    let provider = text_at(&data, "providerID")
        .or_else(|| text_at(&data, "/model/providerID"));
    let written = data
        .get("cost")
        .and_then(Value::as_f64)
        .and_then(microdollars_of);
    let provider = provider.and_then(provider_as_written);
    let cost = answer_cost(written, model, provider.as_ref(), &tokens);

    Record::Answer(Box::new(Answer {
        identity: Some(row.id.clone()),
        session: None,
        at_ms: row.at_ms,
        model: model.map(String::from),
        provider,
        project: None,
        tokens,
        cost,
    }))
}

/// The provider OpenCode names `id`, as it wrote it (issue 23): OTTY joins
/// the name to the provider part that serves it, and gives the price of a
/// model by it even when no part does, such as `google` or a local model.
/// `None` for a name of only spaces.
fn provider_as_written(id: &str) -> Option<ProviderRef> {
    let id = id.trim();

    (!id.is_empty()).then(|| ProviderRef::Id(id.to_string()))
}

/// Whether the answer `data` is finished: it says why it stopped or when
/// it was completed, or that it failed.
fn is_finished(data: &Value) -> bool {
    data.get("finish").is_some()
        || data.get("error").is_some()
        || data.pointer("/time/completed").is_some()
}

/// The tokens of an answer in the shape of the contract: the input
/// written to the cache is part of the input, and the reasoning part of
/// the output, as OpenAI counts it. `None` when `input` or `output` is
/// not a number.
fn token_count(tokens: &Value) -> Option<TokenCount> {
    let input = count_at(tokens, "/input")?;
    let output = count_at(tokens, "/output")?;
    let reasoning = count_at(tokens, "/reasoning").unwrap_or(0);
    let read = count_at(tokens, "/cache/read").unwrap_or(0);
    let written = count_at(tokens, "/cache/write").unwrap_or(0);

    Some(TokenCount {
        input: input.saturating_add(written),
        cached: read,
        cache_written: written,
        output: output.saturating_add(reasoning),
        reasoning,
    })
}

/// The count at `pointer` of `value`: a whole number that is not below
/// zero, which OpenCode may write as a decimal number.
fn count_at(value: &Value, pointer: &str) -> Option<u64> {
    let number = value.pointer(pointer)?;

    number.as_u64().or_else(|| {
        number
            .as_f64()
            .filter(|count| count.is_finite() && *count >= 0.0)
            .map(|count| count.round() as u64)
    })
}

/// `dollars` in millionths of a dollar; `None` for a cost below zero or
/// that is no number.
fn microdollars_of(dollars: f64) -> Option<u64> {
    if !dollars.is_finite() || dollars < 0.0 {
        return None;
    }

    Some((dollars * MICRODOLLARS_IN_DOLLAR).round() as u64)
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
    let found = found
        .and_then(Value::as_object)
        .map(|fields| fields.keys().cloned().collect())
        .unwrap_or_default();

    Record::NotUnderstood {
        missing,
        place,
        found,
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::AnswerCost;

    use super::*;
    use crate::testing::{
        NOW, first_shape_answer, first_shape_question, millis,
        second_shape_answer,
    };

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

    fn expected_answer(provider: &str, model: &str) -> Answer {
        Answer {
            identity: Some("msg_1".to_string()),
            session: None,
            at_ms: millis(NOW),
            model: Some(model.to_string()),
            provider: Some(ProviderRef::Id(provider.to_string())),
            project: None,
            tokens: TokenCount {
                input: 15,
                cached: 300,
                cache_written: 5,
                output: 3,
                reasoning: 1,
            },
            cost: Some(AnswerCost::Written(250_000)),
        }
    }

    #[test]
    fn an_answer_of_the_first_shape_has_its_provider_tokens_and_cost() {
        let data = first_shape_answer("openai", "gpt-5.5", TOKENS, 0.25);

        let record = read_record(&row(None, &data));

        assert_eq!(
            record,
            Record::Answer(Box::new(expected_answer("openai", "gpt-5.5")))
        );
    }

    #[test]
    fn an_answer_of_the_second_shape_has_its_model_in_model() {
        let data = second_shape_answer("anthropic", "claude-x", TOKENS, 0.25);

        let record = read_record(&row(Some("assistant"), &data));

        assert_eq!(
            record,
            Record::Answer(Box::new(expected_answer("anthropic", "claude-x")))
        );
    }

    /// Issue 23: the provider passes as OpenCode wrote it, even when no
    /// plugin serves it; OTTY joins it to the part that serves it, and
    /// prices the answer by it.
    #[test]
    fn an_answer_keeps_the_provider_name_opencode_wrote() {
        let google = first_shape_answer("google", "gemini-x", TOKENS, 0.0);
        let local = first_shape_answer("test", "qwen-local", TOKENS, 0.0);
        let router = second_shape_answer(
            " openrouter ",
            "anthropic/claude-x",
            TOKENS,
            0.0,
        );

        for (data, provider) in
            [(google, "google"), (local, "test"), (router, "openrouter")]
        {
            let record = read_record(&row(Some("assistant"), &data));

            let Record::Answer(answer) = record else {
                panic!("an answer: {data}");
            };
            assert_eq!(
                answer.provider,
                Some(ProviderRef::Id(provider.to_string()))
            );
            assert_eq!(answer.cost, Some(AnswerCost::Written(0)));
        }
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
        assert!(found.contains(&"usage".to_string()), "{found:?}");
        let Record::NotUnderstood { place, found, .. } = parts else {
            panic!("not understood: {parts:?}");
        };
        assert_eq!(place, "tokens");
        assert_eq!(found, ["prompt"]);
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
