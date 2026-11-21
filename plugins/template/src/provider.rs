//! The provider part `example-cloud`: a made-up provider whose API tells
//! whose account a key is and how much money is left on it.

use otty_plugin_sdk::agents::{
    AccountFacts, AccountNumbers, Bill, DayCost, DayRange, Identity,
    LoginFacts, Money, Provider, ReadError,
};
use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::{GivenKey, Key, given_key};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{Example, needs};

/// The provider's API.
const API: &str = "https://api.example.com/v1";

/// The provider's name, as the texts of its needs say it.
const NAME: &str = "Example Cloud";

/// The page where the user makes a new key.
const KEYS_PAGE: &str = "https://example.com/keys";

/// The slot the admin key is given under: the part's own name for it.
const ADMIN_SLOT: &str = "admin";

/// What an admin key starts with.
const ADMIN_PREFIX: &str = "ex-admin-";

/// The page where the user makes an admin key.
const BILL_KEYS_PAGE: &str = "https://example.com/keys/admin";

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

/// The answer of `/bill`: the money of each day, in US dollars.
#[derive(Debug, Deserialize)]
struct BillAnswer {
    days: Vec<BillDay>,
}

/// One day of `/bill`.
#[derive(Debug, Deserialize)]
struct BillDay {
    day: String,
    usd: f64,
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
        _given: &[GivenKey],
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

    /// The bill reads with an admin key: the account's own key when it is
    /// one, else the one the user gave under [`ADMIN_SLOT`], else the
    /// part asks for one.
    fn read_bill(
        _provider: &str,
        _account: &AccountFacts,
        key: &Key,
        given: &[GivenKey],
        days: &DayRange,
    ) -> Result<Bill, ReadError> {
        let is_bill_key =
            key.has_prefix(ADMIN_PREFIX).map_err(ReadError::Host)?;
        let bill_key = match is_bill_key {
            true => key,
            false => given_key(given, ADMIN_SLOT).ok_or_else(|| {
                ReadError::NeedsUser(needs::bill_key(
                    ADMIN_SLOT,
                    ADMIN_PREFIX,
                    BILL_KEYS_PAGE,
                ))
            })?,
        };

        let url = format!("{API}/bill?from={}&to={}", days.first, days.last);
        let answer: BillAnswer = get_json(&url, bill_key)?;

        let days = answer
            .days
            .into_iter()
            .map(|day| DayCost {
                day: day.day,
                cost: Money {
                    amount: day.usd,
                    currency: "USD".to_string(),
                },
            })
            .collect();
        Ok(Bill { days })
    }
}

/// The JSON answer of a `GET` of `url` with `key`. A refused key asks the
/// user for a new one. The error never holds what the answer said.
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
        401 | 403 => Err(ReadError::NeedsUser(needs::new_key(NAME, KEYS_PAGE))),
        status => Err(ReadError::Failed(format!("{url} answered {status}"))),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::LoginKind;
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{GivenKey, HostError, NeedAction, files};

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
                Example::numbers(PROVIDER, &account(), &key, &[])
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

    fn days() -> DayRange {
        DayRange {
            first: "2026-11-01".to_string(),
            last: "2026-11-02".to_string(),
        }
    }

    /// With no admin key the part asks for one, under its slot.
    #[test]
    fn a_bill_with_no_key_for_the_admin_slot_asks_for_one() {
        let host = host();
        let key = host.key(KEY);

        let bill = host.as_provider(PROVIDER, || {
            Example::read_bill(PROVIDER, &account(), &key, &[], &days())
        });

        let Err(ReadError::NeedsUser(need)) = bill else {
            panic!("a need: {bill:?}");
        };
        let NeedAction::AskKey(ask) = need.action else {
            panic!("a key to ask for: {need:?}");
        };
        assert_eq!(ask.slot, ADMIN_SLOT);
        assert_eq!(ask.prefix.as_deref(), Some(ADMIN_PREFIX));
        assert_eq!(ask.page.as_deref(), Some(BILL_KEYS_PAGE));
        assert!(host.requests().is_empty(), "nothing is sent");
    }

    /// The admin key the user gave reads the bill: it goes into the
    /// request, never the account's own key.
    #[test]
    fn the_key_given_for_the_admin_slot_reads_the_bill() {
        let host = host();
        host.answer(
            "GET",
            "https://api.example.com/v1/bill?from=2026-11-01&to=2026-11-02",
            200,
            br#"{"days": [{"day": "2026-11-02", "usd": 3.5}]}"#,
        );
        let key = host.key(KEY);
        let given = [GivenKey::new("admin", host.key("ex-admin-made-up-01"))];

        let bill = host
            .as_provider(PROVIDER, || {
                Example::read_bill(PROVIDER, &account(), &key, &given, &days())
            })
            .expect("the bill");

        assert_eq!(bill.days.len(), 1);
        assert_eq!(bill.days[0].cost.amount, 3.5);
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some("Bearer ex-admin-made-up-01")
        );
    }

    /// A refused key is a need of the part: its button opens the page of
    /// the keys.
    #[test]
    fn a_refused_key_asks_for_a_new_one() {
        let host = host();
        host.answer("GET", "https://api.example.com/v1/balance", 401, b"{}");
        let key = host.key(KEY);

        let numbers = host.as_provider(PROVIDER, || {
            Example::numbers(PROVIDER, &account(), &key, &[])
        });

        let Err(ReadError::NeedsUser(need)) = numbers else {
            panic!("a need: {numbers:?}");
        };
        assert_eq!(need.id, "new-key");
        assert!(need.text.starts_with("Example Cloud refused the key"));
        assert_eq!(need.action, NeedAction::OpenPage(KEYS_PAGE.to_string()));
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
