//! The limit of an OpenRouter key, `GET /api/v1/key` with the key itself,
//! as OpenRouter documents it:
//! <https://openrouter.ai/docs/api/reference/limits.md>. `data.usage` is
//! what the key used and `data.limit` its limit, `null` for none, both in
//! US dollars.

use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::{
    AccountFacts, AccountNumbers, Identity, Key, KeyLimit, LoginFacts, Money,
    Provider, ReadError,
};
use serde_json::Value;

use crate::OpenRouter;

/// The address of the key's own numbers.
const KEY_URL: &str = "https://openrouter.ai/api/v1/key";

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

    /// The limit of `key` and what of it was used.
    fn numbers(
        _provider: &str,
        _account: &AccountFacts,
        key: &Key,
    ) -> Result<AccountNumbers, ReadError> {
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
            left: None,
            key_limit: Some(KeyLimit {
                used: dollars(used),
                limit: limit.map(dollars),
            }),
            spent_this_month: None,
            plan: None,
        })
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
/// refused key (401 or 403) is [`ReadError::KeyRefused`]. The error never
/// holds what the answer said.
fn get_json(url: &str, key: &Key) -> Result<Value, ReadError> {
    let request = Request::get(url)
        .header("accept", "application/json")
        .header_with_key("authorization", "Bearer ", key);

    let response = http::request(&request).map_err(ReadError::Host)?;

    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|_| {
            ReadError::Unexpected(format!("the answer of {url} is not JSON"))
        }),
        401 | 403 => Err(ReadError::KeyRefused),
        status => Err(ReadError::Failed(format!("{url} answered {status}"))),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::HostError;
    use otty_plugin_sdk::test_host::TestHost;

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

    /// The numbers the part reads when the key request answers `status`
    /// and `body`.
    fn numbers_on(
        host: &TestHost,
        status: u16,
        body: &str,
    ) -> Result<AccountNumbers, ReadError> {
        host.answer("GET", KEY_URL, status, body.as_bytes());
        let key = host.key(MADE_UP_KEY);

        host.as_provider(PROVIDER, || {
            OpenRouter::numbers(PROVIDER, &account(), &key)
        })
    }

    #[test]
    fn the_request_asks_openrouter_for_the_key() {
        let host = host();

        let numbers = numbers_on(&host, 403, "");

        assert_eq!(numbers, Err(ReadError::KeyRefused));
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
        let found = LoginFacts {
            kind: otty_plugin_sdk::LoginKind::ApiKey,
            identity: Identity::default(),
            plan: None,
        };

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
