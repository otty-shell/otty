//! The provider part `example-cloud`: a made-up provider whose API tells
//! whose account a key is and how much money is left on it.

use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::{
    AccountFacts, AccountNumbers, Identity, Key, LoginFacts, Money, Provider,
    ReadError,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Example;

/// The provider's API.
const API: &str = "https://api.example.com/v1";

/// The answer of `/me`.
#[derive(Debug, Deserialize)]
struct Me {
    account_id: String,
    email: Option<String>,
    plan: Option<String>,
}

/// The answer of `/balance`.
#[derive(Debug, Deserialize)]
struct Balance {
    balance: f64,
    currency: String,
}

impl Provider for Example {
    fn account(
        _provider: &str,
        key: &Key,
        _found: &LoginFacts,
    ) -> Result<AccountFacts, ReadError> {
        let me: Me = get_json(&format!("{API}/me"), key)?;

        Ok(AccountFacts {
            identity: Identity {
                account_id: Some(me.account_id),
                email: me.email,
                ..Identity::default()
            },
            plan: me.plan,
        })
    }

    fn numbers(
        _provider: &str,
        _account: &AccountFacts,
        key: &Key,
    ) -> Result<AccountNumbers, ReadError> {
        let balance: Balance = get_json(&format!("{API}/balance"), key)?;

        Ok(AccountNumbers {
            windows: Vec::new(),
            credits: None,
            extra_usage: None,
            left: Some(Money {
                amount: balance.balance,
                currency: balance.currency,
            }),
            key_limit: None,
            spent_this_month: None,
            plan: None,
        })
    }
}

/// The JSON answer of a `GET` of `url` with `key`. The error never holds
/// what the answer said.
fn get_json<T: DeserializeOwned>(url: &str, key: &Key) -> Result<T, ReadError> {
    let request = Request::get(url)
        .header("accept", "application/json")
        .header_with_key("authorization", "Bearer ", key);

    let response = http::request(&request).map_err(ReadError::Host)?;

    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|_| {
            ReadError::Unexpected(format!(
                "the answer of {url} has another shape"
            ))
        }),
        401 | 403 => Err(ReadError::KeyRefused),
        status => Err(ReadError::Failed(format!("{url} answered {status}"))),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{HostError, LoginKind, files};

    use super::*;

    const PROVIDER: &str = "example-cloud";
    const KEY: &str = "ex-made-up-key-0001";

    fn host() -> TestHost {
        TestHost::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn account() -> AccountFacts {
        AccountFacts {
            identity: Identity::default(),
            plan: None,
        }
    }

    #[test]
    fn numbers_read_the_balance_with_the_key_in_the_header() {
        let host = host();
        host.answer(
            "GET",
            "https://api.example.com/v1/balance",
            200,
            br#"{"balance": 12.5, "currency": "USD"}"#,
        );
        let key = host.key(KEY);

        let numbers = host
            .as_provider(PROVIDER, || {
                Example::numbers(PROVIDER, &account(), &key)
            })
            .expect("the numbers");

        assert_eq!(
            numbers.left,
            Some(Money {
                amount: 12.5,
                currency: "USD".to_string(),
            })
        );
        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].header("authorization"),
            Some("Bearer ex-made-up-key-0001")
        );
    }

    #[test]
    fn the_account_of_a_key_is_its_id_and_email() {
        let host = host();
        host.answer(
            "GET",
            "https://api.example.com/v1/me",
            200,
            br#"{"account_id": "acct_1", "email": "dev@example.com", "plan": "Pay as you go"}"#,
        );
        let key = host.key(KEY);
        let found = LoginFacts {
            kind: LoginKind::ApiKey,
            identity: Identity::default(),
            plan: None,
        };

        let facts = host
            .as_provider(PROVIDER, || Example::account(PROVIDER, &key, &found))
            .expect("the account");

        assert_eq!(facts.identity.account_id.as_deref(), Some("acct_1"));
        assert_eq!(facts.identity.email.as_deref(), Some("dev@example.com"));
        assert_eq!(facts.plan.as_deref(), Some("Pay as you go"));
    }

    #[test]
    fn a_refused_key_says_so() {
        let host = host();
        host.answer("GET", "https://api.example.com/v1/balance", 401, b"{}");
        let key = host.key(KEY);

        let numbers = host.as_provider(PROVIDER, || {
            Example::numbers(PROVIDER, &account(), &key)
        });

        assert_eq!(numbers, Err(ReadError::KeyRefused));
    }

    #[test]
    fn the_provider_part_may_not_read_the_agents_files() {
        let host = host();
        let state = host.home().join(".example-agent/state.json");

        let read = host.as_provider(PROVIDER, || {
            files::read(&state.display().to_string(), 1024)
        });

        assert_eq!(
            read,
            Err(HostError::NotAllowed(state.display().to_string()))
        );
    }
}
