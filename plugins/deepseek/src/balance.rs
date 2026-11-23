//! The money left on a DeepSeek account, `GET /user/balance` with its key,
//! as DeepSeek documents it:
//! <https://api-docs.deepseek.com/api/get-user-balance>. The answer lists
//! the balance in each currency of the account, `total_balance` as decimal
//! text.

use otty_plugin_sdk::agents::{
    AccountFacts, AccountNumbers, Identity, LoginFacts, Money, Provider,
    ReadError,
};
use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::{GivenKey, HostError, Key};
use serde_json::Value;

use crate::{DeepSeek, needs};

/// The address of the balance.
const BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

/// The page where the user makes a new key.
const KEYS_PAGE: &str = "https://platform.deepseek.com/api_keys";

impl Provider for DeepSeek {
    /// DeepSeek does not tell whose account a key is: OTTY knows the
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

    /// The money left on the account of `key`: the total balance of the
    /// first currency the answer lists. The part asks for no other key.
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

        let answer = get_json(BALANCE_URL, key)?;

        let left = answer
            .get("balance_infos")
            .and_then(Value::as_array)
            .and_then(|infos| infos.iter().find_map(money_of))
            .ok_or_else(|| {
                ReadError::Unexpected(format!(
                    "the answer of {BALANCE_URL} has no balance_infos with \
                     currency and total_balance"
                ))
            })?;

        Ok(AccountNumbers {
            windows: Vec::new(),
            credits: None,
            extra_usage: None,
            left: Some(left),
            key_limit: None,
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

/// The total balance of one currency of the answer.
fn money_of(info: &Value) -> Option<Money> {
    let amount = number_in(info.get("total_balance")?)?;
    let currency = info.get("currency")?.as_str()?;

    Some(Money {
        amount,
        currency: currency.to_string(),
    })
}

/// A number of an answer, written as a JSON number or as decimal text,
/// such as `"110.00"`; `None` for anything else, infinity included.
fn number_in(value: &Value) -> Option<f64> {
    let number = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }?;

    number.is_finite().then_some(number)
}

/// The JSON answer of a `GET` of `url` with `key` as its bearer token. A
/// refused key (401 or 403) asks the user for a new one where the agent
/// keeps it. The error never holds what the answer said.
fn get_json(url: &str, key: &Key) -> Result<Value, ReadError> {
    let request = Request::get(url)
        .header("accept", "application/json")
        .header_with_key("authorization", "Bearer ", key);

    let response = http::request(&request).map_err(ReadError::Host)?;

    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|_| {
            ReadError::Unexpected(format!("the answer of {url} is not JSON"))
        }),
        401 | 403 => Err(ReadError::NeedsUser(needs::new_key(KEYS_PAGE))),
        status => Err(ReadError::Failed(format!("{url} answered {status}"))),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{HostError, NeedAction};

    use super::*;

    const PROVIDER: &str = "deepseek";
    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-made-up-deepseek-key-0000";

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
            DeepSeek::numbers(PROVIDER, &found(), &account(), &key, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
    }

    /// The numbers the part reads when the balance request answers
    /// `status` and `body`, with a key read from a place of an agent.
    fn numbers_on(
        host: &TestHost,
        status: u16,
        body: &str,
    ) -> Result<AccountNumbers, ReadError> {
        let key = host.key(MADE_UP_KEY);

        numbers_with(host, &key, status, body)
    }

    /// The numbers the part reads with `key` when the balance request
    /// answers `status` and `body`.
    fn numbers_with(
        host: &TestHost,
        key: &Key,
        status: u16,
        body: &str,
    ) -> Result<AccountNumbers, ReadError> {
        host.answer("GET", BALANCE_URL, status, body.as_bytes());

        host.as_provider(PROVIDER, || {
            DeepSeek::numbers(PROVIDER, &found(), &account(), key, &[])
        })
    }

    #[test]
    fn the_request_sends_the_key_to_deepseek_only() {
        let host = host();

        let _ = numbers_on(
            &host,
            200,
            r#"{"is_available": true, "balance_infos": []}"#,
        );

        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method(), "GET");
        assert_eq!(requests[0].url(), BALANCE_URL);
        assert_eq!(
            requests[0].header("authorization"),
            Some(format!("Bearer {MADE_UP_KEY}").as_str())
        );
    }

    #[test]
    fn left_is_the_total_balance_in_its_currency() {
        let host = host();

        let numbers = numbers_on(
            &host,
            200,
            r#"{"is_available": true, "balance_infos": [
                {"currency": "CNY", "total_balance": "110.00",
                 "granted_balance": "10.00", "topped_up_balance": "100.00"}
            ]}"#,
        )
        .expect("the numbers");

        assert_eq!(
            numbers.left,
            Some(Money {
                amount: 110.0,
                currency: "CNY".to_string(),
            })
        );
        assert_eq!(numbers.key_limit, None);
        assert!(numbers.windows.is_empty());
    }

    #[test]
    fn a_balance_written_as_a_number_is_read_too() {
        let host = host();

        let numbers = numbers_on(
            &host,
            200,
            r#"{"balance_infos": [
                {"currency": "USD", "total_balance": "not a number"},
                {"currency": "USD", "total_balance": 4.2}
            ]}"#,
        )
        .expect("the numbers");

        assert_eq!(
            numbers.left,
            Some(Money {
                amount: 4.2,
                currency: "USD".to_string(),
            })
        );
    }

    #[test]
    fn an_answer_with_no_balance_is_unexpected() {
        for body in [
            r#"{"balance_infos": []}"#,
            r#"{"is_available": false}"#,
            r#"{"balance_infos": [{"total_balance": "1.00"}]}"#,
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
    /// opens the page where the user makes a new DeepSeek key, which the
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
                need.text.starts_with("DeepSeek refused the key"),
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
            russian.text.starts_with("DeepSeek отклонил ключ"),
            "{}",
            russian.text
        );
        assert_eq!(unknown.title, "The key was refused");
        assert!(
            unknown.text.starts_with("DeepSeek refused the key"),
            "{}",
            unknown.text
        );
    }

    #[test]
    fn another_status_fails_without_what_the_answer_said() {
        let host = host();

        let numbers = numbers_on(&host, 503, "made-up answer text");

        let Err(ReadError::Failed(text)) = numbers else {
            panic!("a failure: {numbers:?}");
        };
        assert!(text.contains("503"), "{text}");
        assert!(!text.contains("made-up answer text"), "{text}");
    }

    #[test]
    fn the_account_of_a_key_is_known_by_its_fingerprint_only() {
        let host = host();
        let key = host.key(MADE_UP_KEY);
        let found = found();

        let facts = host
            .as_provider(PROVIDER, || DeepSeek::account(PROVIDER, &key, &found))
            .expect("the account");

        assert_eq!(facts, account());
        assert!(host.requests().is_empty());
    }

    /// The part may send requests to DeepSeek's API host only.
    #[test]
    fn the_part_may_call_no_other_host() {
        let host = host();

        let sent = host.as_provider(PROVIDER, || {
            http::request(&Request::get("https://openrouter.ai/api/v1/key"))
        });

        assert_eq!(sent, Err(HostError::NotAllowed("openrouter.ai".into())));
    }
}
