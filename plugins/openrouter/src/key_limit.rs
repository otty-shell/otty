//! The limit of an OpenRouter key, `GET /api/v1/key` with the key itself,
//! as OpenRouter documents it:
//! <https://openrouter.ai/docs/api/reference/limits.md>. `data.usage` is
//! what the key used and `data.limit` its limit, `null` for none, both in
//! US dollars.

use otty_plugin_sdk::agents::{
    AccountFacts, AccountNumbers, Identity, KeyLimit, LoginFacts, Money,
    Provider, ReadError,
};
use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::{GivenKey, HostError, Key, Need};
use serde_json::Value;

use crate::{OpenRouter, needs};

/// The address of the key's own numbers.
const KEY_URL: &str = "https://openrouter.ai/api/v1/key";

/// The page where the user makes a new key.
const KEYS_PAGE: &str = "https://openrouter.ai/settings/keys";

impl Provider for OpenRouter {
    /// OpenRouter does not tell whose account a key is: OTTY knows the
    /// account by the key's fingerprint, so nothing is asked.
    fn account(
        _provider: &str,
        _key: &Key,
        _found: &LoginFacts,
    ) -> Result<AccountFacts, ReadError> {
        Ok(AccountFacts {
            identity: Identity::default(),
            plan: None,
        })
    }

    /// The limit of `key` and what of it was used. The part asks for no
    /// other key.
    ///
    /// A key that is not at its place now is a normal case: the part has
    /// no other way to read the account, so it answers nothing and sends
    /// nothing (issue 29).
    fn numbers(
        _provider: &str,
        _found: &LoginFacts,
        _account: &AccountFacts,
        key: &Key,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        if !is_there(key)? {
            return Ok(no_numbers());
        }

        let answer = get_json(KEY_URL, key)?;

        let data = answer.get("data");
        let used = data
            .and_then(|data| data.get("usage"))
            .and_then(number_in)
            .ok_or_else(|| {
                ReadError::Unexpected(format!(
                    "the answer of {KEY_URL} has no number in data.usage"
                ))
            })?;
        let limit = data.and_then(|data| data.get("limit")).and_then(number_in);

        Ok(AccountNumbers {
            windows: Vec::new(),
            credits: None,
            extra_usage: None,
            left: None,
            key_limit: Some(KeyLimit {
                used: dollars(used),
                limit: limit.map(dollars),
            }),
            spent_this_month: None,
            plan: None,
            recorded_at_ms: None,
        })
    }
}

/// Whether the key of `key` is at its place now.
fn is_there(key: &Key) -> Result<bool, ReadError> {
    match key.fingerprint() {
        Ok(_) => Ok(true),
        Err(HostError::NotFound(_)) => Ok(false),
        Err(error) => Err(ReadError::Host(error)),
    }
}

/// Numbers with nothing in them: what the part answers when it has no way
/// to read the account.
fn no_numbers() -> AccountNumbers {
    AccountNumbers {
        windows: Vec::new(),
        credits: None,
        extra_usage: None,
        left: None,
        key_limit: None,
        spent_this_month: None,
        plan: None,
        recorded_at_ms: None,
    }
}

/// `amount` US dollars.
fn dollars(amount: f64) -> Money {
    Money {
        amount,
        currency: "USD".to_string(),
    }
}

/// A number of an answer, written as a JSON number or as decimal text,
/// such as `"12.30"`; `None` for anything else, `null` and infinity
/// included.
fn number_in(value: &Value) -> Option<f64> {
    let number = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }?;

    number.is_finite().then_some(number)
}

/// The JSON answer of a `GET` of `url` with `key` as its bearer token. A
/// refused key (401 or 403) asks the user for a new one where the old one
/// sits: in OTTY when the user gave it there, else where the agent keeps
/// it. The error never holds what the answer said.
fn get_json(url: &str, key: &Key) -> Result<Value, ReadError> {
    let request = Request::get(url)
        .header("accept", "application/json")
        .header_with_key("authorization", "Bearer ", key);

    let response = http::request(&request).map_err(ReadError::Host)?;

    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|_| {
            ReadError::Unexpected(format!("the answer of {url} is not JSON"))
        }),
        401 | 403 => Err(ReadError::NeedsUser(refused_key_need(key)?)),
        status => Err(ReadError::Failed(format!("{url} answered {status}"))),
    }
}

/// What the user does about the refused `key`: a key given in OTTY is
/// replaced in OTTY's field, one an agent keeps where the agent keeps it.
fn refused_key_need(key: &Key) -> Result<Need, ReadError> {
    let is_given_in_otty = key.is_given_in_otty().map_err(ReadError::Host)?;

    Ok(match is_given_in_otty {
        true => needs::replace_key(KEYS_PAGE),
        false => needs::new_key(KEYS_PAGE),
    })
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{HostError, NeedAction};

    use super::*;

    const PROVIDER: &str = "openrouter";
    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-or-v1-made-up-key-0000";

    fn host() -> TestHost {
        TestHost::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn account() -> AccountFacts {
        AccountFacts {
            identity: Identity::default(),
            plan: None,
        }
    }

    /// What OTTY found of an API key of an agent, found in no profile.
    fn found() -> LoginFacts {
        LoginFacts {
            kind: otty_plugin_sdk::agents::LoginKind::ApiKey,
            identity: Identity::default(),
            plan: None,
            found_in: Vec::new(),
        }
    }

    /// Issue 29: a key that is not at its place now is nothing, not a
    /// failure, and nothing is sent.
    #[test]
    fn a_key_that_is_not_there_reads_nothing() {
        let host = host();
        let key = host.key_at(&otty_plugin_sdk::KeyPlace::EnvVar(
            "NO_SUCH_KEY".to_string(),
        ));

        let numbers = host.as_provider(PROVIDER, || {
            OpenRouter::numbers(PROVIDER, &found(), &account(), &key, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
    }

    /// The numbers the part reads when the key request answers `status`
    /// and `body`, with a key read from a place of an agent.
    fn numbers_on(
        host: &TestHost,
        status: u16,
        body: &str,
    ) -> Result<AccountNumbers, ReadError> {
        let key = host.key(MADE_UP_KEY);

        numbers_with(host, &key, status, body)
    }

    /// The numbers the part reads with `key` when the key request answers
    /// `status` and `body`.
    fn numbers_with(
        host: &TestHost,
        key: &Key,
        status: u16,
        body: &str,
    ) -> Result<AccountNumbers, ReadError> {
        host.answer("GET", KEY_URL, status, body.as_bytes());

        host.as_provider(PROVIDER, || {
            OpenRouter::numbers(PROVIDER, &found(), &account(), key, &[])
        })
    }

    #[test]
    fn the_request_asks_openrouter_for_the_key() {
        let host = host();

        let _ = numbers_on(&host, 200, r#"{"data": {"usage": 0}}"#);

        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method(), "GET");
        assert_eq!(requests[0].url(), KEY_URL);
        assert_eq!(
            requests[0].header("authorization"),
            Some(format!("Bearer {MADE_UP_KEY}").as_str())
        );
    }

    #[test]
    fn the_key_limit_is_the_usage_of_the_limit() {
        let host = host();

        let numbers = numbers_on(
            &host,
            200,
            r#"{"data": {"label": "sk-or-v1-abc", "usage": 12.3,
            "limit": 50, "limit_remaining": 37.7, "is_free_tier": false}}"#,
        )
        .expect("the numbers");

        assert_eq!(
            numbers.key_limit,
            Some(KeyLimit {
                used: dollars(12.3),
                limit: Some(dollars(50.0)),
            })
        );
        assert_eq!(numbers.left, None);
    }

    #[test]
    fn a_key_with_no_limit_has_only_its_usage() {
        let host = host();

        let numbers = numbers_on(
            &host,
            200,
            r#"{"data": {"usage": 0.5, "limit": null, "limit_remaining": null}}"#,
        )
        .expect("the numbers");

        assert_eq!(
            numbers.key_limit,
            Some(KeyLimit {
                used: dollars(0.5),
                limit: None,
            })
        );
    }

    #[test]
    fn numbers_written_as_decimal_text_are_read_too() {
        let host = host();

        let numbers = numbers_on(
            &host,
            200,
            r#"{"data": {"usage": " 1.25 ", "limit": "10"}}"#,
        )
        .expect("the numbers");

        assert_eq!(
            numbers.key_limit,
            Some(KeyLimit {
                used: dollars(1.25),
                limit: Some(dollars(10.0)),
            })
        );
    }

    #[test]
    fn an_answer_with_no_usage_is_unexpected() {
        for body in [
            r#"{"data": {"limit": 50}}"#,
            r#"{"data": {"usage": "ten"}}"#,
            r#"{"usage": 1}"#,
            "not JSON",
        ] {
            let host = host();

            let numbers = numbers_on(&host, 200, body);

            assert!(
                matches!(numbers, Err(ReadError::Unexpected(_))),
                "{body}: {numbers:?}"
            );
        }
    }

    /// A refused key an agent keeps is a need of the part: its button
    /// opens the page where the user makes a new OpenRouter key, which the
    /// user puts where the agent keeps the old one.
    #[test]
    fn a_refused_key_of_an_agent_asks_for_a_new_one_on_the_keys_page() {
        for status in [401, 403] {
            let host = host();

            let numbers = numbers_on(&host, status, r#"{"error": "invalid"}"#);

            let Err(ReadError::NeedsUser(need)) = numbers else {
                panic!("{status}: a need: {numbers:?}");
            };
            assert_eq!(need.id, "new-key", "{status}");
            assert_eq!(need.title, "The key was refused", "{status}");
            assert!(
                need.text.starts_with("OpenRouter refused the key"),
                "{status}: {}",
                need.text
            );
            assert_eq!(
                need.action,
                NeedAction::OpenPage(KEYS_PAGE.to_string()),
                "{status}"
            );
        }
    }

    /// Issue 26: a refused key the user gave in OTTY is replaced in OTTY:
    /// the need's field takes the new key in place of the old one, with
    /// the keys page at hand.
    #[test]
    fn a_refused_key_given_in_otty_asks_for_a_new_one_in_its_place() {
        for status in [401, 403] {
            let host = host();
            let key = host.given_key(MADE_UP_KEY);

            let numbers = numbers_with(&host, &key, status, "{}");

            let Err(ReadError::NeedsUser(need)) = numbers else {
                panic!("{status}: a need: {numbers:?}");
            };
            assert_eq!(need.id, "replace-key", "{status}");
            assert_eq!(need.title, "The key was refused", "{status}");
            assert!(
                need.text.starts_with("OpenRouter refused the key"),
                "{status}: {}",
                need.text
            );
            let NeedAction::AskKey(ask) = need.action else {
                panic!("{status}: a field for the key");
            };
            assert_eq!(ask.slot, None, "the account's own key");
            assert_eq!(ask.label, "OpenRouter key");
            assert_eq!(ask.prefix.as_deref(), Some("sk-or-"));
            assert_eq!(ask.page.as_deref(), Some(KEYS_PAGE));
        }
    }

    /// The field of a refused key given in OTTY is in the language of
    /// OTTY's interface too.
    #[test]
    fn the_field_of_a_refused_key_given_in_otty_is_in_the_language_of_otty() {
        let host = host().with_language("ru");
        let key = host.given_key(MADE_UP_KEY);

        let numbers = numbers_with(&host, &key, 401, "");

        let Err(ReadError::NeedsUser(need)) = numbers else {
            panic!("a need: {numbers:?}");
        };
        let NeedAction::AskKey(ask) = need.action else {
            panic!("a field for the key");
        };
        assert_eq!(need.title, "Ключ отклонён");
        assert!(
            need.text.starts_with("OpenRouter отклонил ключ"),
            "{}",
            need.text
        );
        assert_eq!(ask.label, "Ключ OpenRouter");
    }

    /// The need is written in the language of OTTY's interface, and in
    /// English for a language the plugin has no texts for.
    #[test]
    fn the_need_of_a_refused_key_is_in_the_language_of_otty() {
        let need_in = |language: &str| {
            let host = host().with_language(language);

            match numbers_on(&host, 401, "") {
                Err(ReadError::NeedsUser(need)) => need,
                other => panic!("{language}: a need: {other:?}"),
            }
        };

        let russian = need_in("ru");
        let unknown = need_in("eo");

        assert_eq!(russian.title, "Ключ отклонён");
        assert!(
            russian.text.starts_with("OpenRouter отклонил ключ"),
            "{}",
            russian.text
        );
        assert_eq!(unknown.title, "The key was refused");
        assert!(
            unknown.text.starts_with("OpenRouter refused the key"),
            "{}",
            unknown.text
        );
    }

    #[test]
    fn another_status_fails_without_what_the_answer_said() {
        let host = host();

        let numbers = numbers_on(&host, 500, "made-up answer text");

        let Err(ReadError::Failed(text)) = numbers else {
            panic!("a failure: {numbers:?}");
        };
        assert!(text.contains("500"), "{text}");
        assert!(!text.contains("made-up answer text"), "{text}");
    }

    #[test]
    fn the_account_of_a_key_is_known_by_its_fingerprint_only() {
        let host = host();
        let key = host.key(MADE_UP_KEY);
        let found = found();

        let facts = host
            .as_provider(PROVIDER, || {
                OpenRouter::account(PROVIDER, &key, &found)
            })
            .expect("the account");

        assert_eq!(facts, account());
        assert!(host.requests().is_empty());
    }

    /// The part may send requests to OpenRouter only.
    #[test]
    fn the_part_may_call_no_other_host() {
        let host = host();

        let sent = host.as_provider(PROVIDER, || {
            http::request(&Request::get(
                "https://api.deepseek.com/user/balance",
            ))
        });

        assert_eq!(sent, Err(HostError::NotAllowed("api.deepseek.com".into())));
    }
}
