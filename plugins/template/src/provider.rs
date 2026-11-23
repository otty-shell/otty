//! The provider part `example-cloud`: a made-up provider whose API tells
//! whose account a key is and how much money is left on it. It reads the
//! key of a login itself in the example agent's profile folders OTTY
//! gives it, as its manifest's `logins` declare.

use otty_plugin_sdk::agents::{
    AccountFacts, AccountNumbers, Bill, DayCost, DayRange, Identity,
    LoginFacts, Money, Provider, ReadError,
};
use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::{GivenKey, HostError, Key, Need, files, given_key};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{Example, needs};

/// The provider's API.
const API: &str = "https://api.example.com/v1";

/// The id of this plugin, as OTTY names it in a found profile.
const PLUGIN: &str = "example";

/// The agent part whose profiles hold the logins the part reads.
const AGENT: &str = "example-agent";

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

/// `auth.json` of a profile of the example agent, its key replaced by a
/// handle.
#[derive(Debug, Deserialize)]
struct AuthFile {
    api_key: Option<KeyMarker>,
}

/// `{"$key": n}`: where the host put a key.
#[derive(Debug, Deserialize)]
struct KeyMarker {
    #[serde(rename = "$key")]
    index: usize,
}

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
    /// Whose account the key is, asked of the provider. With no key at its
    /// place now, the account is what the agent part found of it.
    fn account(
        _provider: &str,
        key: &Key,
        found: &LoginFacts,
    ) -> Result<AccountFacts, ReadError> {
        if !is_there(key)? {
            return Ok(AccountFacts {
                identity: found.identity.clone(),
                plan: found.plan.clone(),
            });
        }

        let me: Me = get_json(&format!("{API}/me"), key, refused_key_need)?;

        Ok(AccountFacts {
            identity: Identity {
                account_id: Some(me.account_id),
                email: me.email,
                ..Identity::default()
            },
            plan: me.plan,
        })
    }

    /// The money left on the key of the login: the one the part reads
    /// itself in a profile folder of the example agent, else the key OTTY
    /// lends. A key that is not at its place now, such as a variable the
    /// user unset, is a normal case: the part has no other way to read the
    /// account, so it answers nothing, and sends nothing.
    fn numbers(
        _provider: &str,
        found: &LoginFacts,
        _account: &AccountFacts,
        key: &Key,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        let own = login_key(found)?;
        let key = own.as_ref().unwrap_or(key);

        if !is_there(key)? {
            return Ok(no_numbers());
        }

        let url = format!("{API}/balance");
        let balance: Balance = get_json(&url, key, refused_key_need)?;

        Ok(AccountNumbers {
            left: Some(Money {
                amount: balance.balance,
                currency: balance.currency,
            }),
            ..no_numbers()
        })
    }

    /// The bill reads with an admin key: the account's own key when it is
    /// one, else the one the user gave under [`ADMIN_SLOT`], else the
    /// part asks for one. A refused key is asked for again where it
    /// sits: the account's own key as any refused key of the account, the
    /// given one under its slot.
    fn read_bill(
        _provider: &str,
        _found: &LoginFacts,
        _account: &AccountFacts,
        key: &Key,
        given: &[GivenKey],
        days: &DayRange,
    ) -> Result<Bill, ReadError> {
        let url = format!("{API}/bill?from={}&to={}", days.first, days.last);
        let is_bill_key = is_bill_key(key)?;

        let answer: BillAnswer = match is_bill_key {
            true => get_json(&url, key, refused_key_need)?,
            false => {
                let bill_key = given_key(given, ADMIN_SLOT)
                    .ok_or_else(|| ReadError::NeedsUser(admin_key_need()))?;

                get_json(&url, bill_key, admin_key_need)?
            },
        };

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

/// The key of `auth.json` in the first profile folder of the example agent
/// where the login was found that holds one, as a handle. Only a profile
/// of this plugin is a login folder of the call; a folder with no
/// `auth.json` is passed over.
fn login_key(found: &LoginFacts) -> Result<Option<Key>, ReadError> {
    let folders = found
        .found_in
        .iter()
        .filter(|profile| profile.plugin == PLUGIN && profile.agent == AGENT)
        .map(|profile| profile.folder.as_str());

    for folder in folders {
        let document =
            match files::json_with_keys(&format!("{folder}/auth.json")) {
                Ok(document) => document,
                Err(HostError::NotFound(_)) => continue,
                Err(error) => return Err(ReadError::Host(error)),
            };
        let (json, keys) = document.into_parts();
        let Ok(auth) = serde_json::from_str::<AuthFile>(&json) else {
            continue;
        };

        let key = auth
            .api_key
            .and_then(|marker| keys.into_iter().nth(marker.index));
        if key.is_some() {
            return Ok(key);
        }
    }

    Ok(None)
}

/// Whether `key` reads bills: an admin key. A key that is not at its
/// place is not one; the part then asks for an admin key.
fn is_bill_key(key: &Key) -> Result<bool, ReadError> {
    match key.has_prefix(ADMIN_PREFIX) {
        Ok(is_bill_key) => Ok(is_bill_key),
        Err(HostError::NotFound(_)) => Ok(false),
        Err(error) => Err(ReadError::Host(error)),
    }
}

/// Whether the key of `key` is at its place now. A key that is not is a
/// normal case, such as a variable the user unset, not a failure.
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

/// The need that asks for the admin key under [`ADMIN_SLOT`].
fn admin_key_need() -> Need {
    needs::bill_key(ADMIN_SLOT, ADMIN_PREFIX, BILL_KEYS_PAGE)
}

/// What the user does about the account's own key, refused: the agent
/// keeps it, so a new one goes where the agent keeps it. A key given under
/// a slot is asked for again under its slot, never with this need.
fn refused_key_need() -> Need {
    needs::new_key(NAME, KEYS_PAGE)
}

/// The JSON answer of a `GET` of `url` with `key`. A refused key is the
/// need `refused` gives. The error never holds what the answer said.
fn get_json<T: DeserializeOwned>(
    url: &str,
    key: &Key,
    refused: impl FnOnce() -> Need,
) -> Result<T, ReadError> {
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
        401 | 403 => Err(ReadError::NeedsUser(refused())),
        status => Err(ReadError::Failed(format!("{url} answered {status}"))),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::{FoundProfile, LoginKind};
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{Command, GivenKey, HostError, NeedAction, files};

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

    /// What OTTY found of an API key of an agent, found in no profile.
    fn found() -> LoginFacts {
        LoginFacts {
            kind: LoginKind::ApiKey,
            identity: Identity::default(),
            plan: None,
            found_in: Vec::new(),
        }
    }

    /// What OTTY found of a login in the profile in `folder` of the home
    /// folder, of the example agent of the plugin `plugin`.
    fn found_in_profile(
        host: &TestHost,
        plugin: &str,
        folder: &str,
    ) -> LoginFacts {
        LoginFacts {
            found_in: vec![FoundProfile {
                plugin: plugin.to_string(),
                agent: "example-agent".to_string(),
                folder: host.home().join(folder).display().to_string(),
                launch: Command {
                    program: "example-agent".to_string(),
                    args: Vec::new(),
                    env: Vec::new(),
                },
            }],
            ..found()
        }
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: the part
    /// reads the key of `auth.json` in a profile folder OTTY gives it
    /// itself, as a handle, and reads the balance with it.
    #[test]
    fn numbers_read_with_the_key_of_the_login_folder() {
        let host = host();
        host.answer(
            "GET",
            "https://api.example.com/v1/balance",
            200,
            br#"{"balance": 12.5, "currency": "USD"}"#,
        );
        let unset = host.key_at(&otty_plugin_sdk::KeyPlace::EnvVar(
            "EXAMPLE_CLOUD_KEY".to_string(),
        ));
        let found = found_in_profile(&host, "example", ".example-agent-work");

        let numbers = host
            .as_provider_in(PROVIDER, &found, || {
                Example::numbers(PROVIDER, &found, &account(), &unset, &[])
            })
            .expect("the numbers");

        assert_eq!(numbers.left.map(|left| left.amount), Some(12.5));
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some("Bearer ex-made-up-key-0002")
        );
    }

    /// A profile of another plugin is no login folder of the call: the
    /// part reads no key there and sends nothing.
    #[test]
    fn a_profile_of_another_plugin_gives_no_key() {
        let host = host();
        let unset = host.key_at(&otty_plugin_sdk::KeyPlace::EnvVar(
            "EXAMPLE_CLOUD_KEY".to_string(),
        ));
        let found = found_in_profile(&host, "another", ".example-agent-work");

        let numbers = host.as_provider_in(PROVIDER, &found, || {
            Example::numbers(PROVIDER, &found, &account(), &unset, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty(), "nothing is sent");
    }

    /// Issue 29: a key that is not at its place now is no failure: the
    /// part answers nothing and sends nothing.
    #[test]
    fn a_key_that_is_not_there_reads_nothing() {
        let host = host();
        let key = host.key_at(&otty_plugin_sdk::KeyPlace::EnvVar(
            "EXAMPLE_CLOUD_KEY".to_string(),
        ));

        let numbers = host.as_provider(PROVIDER, || {
            Example::numbers(PROVIDER, &found(), &account(), &key, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty(), "nothing is sent");
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
                Example::numbers(PROVIDER, &found(), &account(), &key, &[])
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
        let found = found();

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

    /// With the account's key not at its place and no admin key given,
    /// the part asks for an admin key and sends nothing: a key that is
    /// not there is not a failure.
    #[test]
    fn a_bill_with_no_key_there_and_no_admin_key_asks_for_one() {
        let host = host();
        let key = host.key_at(&otty_plugin_sdk::KeyPlace::EnvVar(
            "EXAMPLE_CLOUD_KEY".to_string(),
        ));

        let bill = host.as_provider(PROVIDER, || {
            Example::read_bill(
                PROVIDER,
                &found(),
                &account(),
                &key,
                &[],
                &days(),
            )
        });

        assert!(
            matches!(bill, Err(ReadError::NeedsUser(_))),
            "a need: {bill:?}"
        );
        assert!(host.requests().is_empty(), "nothing is sent");
    }

    /// With no admin key the part asks for one, under its slot.
    #[test]
    fn a_bill_with_no_key_for_the_admin_slot_asks_for_one() {
        let host = host();
        let key = host.key(KEY);

        let bill = host.as_provider(PROVIDER, || {
            Example::read_bill(
                PROVIDER,
                &found(),
                &account(),
                &key,
                &[],
                &days(),
            )
        });

        let Err(ReadError::NeedsUser(need)) = bill else {
            panic!("a need: {bill:?}");
        };
        let NeedAction::AskKey(ask) = need.action else {
            panic!("a key to ask for: {need:?}");
        };
        assert_eq!(ask.slot.as_deref(), Some(ADMIN_SLOT));
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
                Example::read_bill(
                    PROVIDER,
                    &found(),
                    &account(),
                    &key,
                    &given,
                    &days(),
                )
            })
            .expect("the bill");

        assert_eq!(bill.days.len(), 1);
        assert_eq!(bill.days[0].cost.amount, 3.5);
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some("Bearer ex-admin-made-up-01")
        );
    }

    /// The need of a refused bill read with the account's own admin key
    /// `own` and the keys of `given`.
    fn refused_bill_need(
        host: &TestHost,
        own: &Key,
        given: &[GivenKey],
    ) -> otty_plugin_sdk::Need {
        host.answer(
            "GET",
            "https://api.example.com/v1/bill?from=2026-11-01&to=2026-11-02",
            401,
            b"{}",
        );

        let bill = host.as_provider(PROVIDER, || {
            Example::read_bill(
                PROVIDER,
                &found(),
                &account(),
                own,
                given,
                &days(),
            )
        });

        match bill {
            Err(ReadError::NeedsUser(need)) => need,
            other => panic!("a need: {other:?}"),
        }
    }

    /// The account's own admin key reads the bill; refused, it is
    /// replaced where the agent keeps it: the button opens the keys page.
    #[test]
    fn a_refused_own_admin_key_is_replaced_where_the_agent_keeps_it() {
        let host = host();
        let found = host.key("ex-admin-made-up-02");

        let need = refused_bill_need(&host, &found, &[]);

        assert_eq!(need.action, NeedAction::OpenPage(KEYS_PAGE.to_string()));
    }

    /// A refused admin key given under the slot is asked for again under
    /// that slot.
    #[test]
    fn a_refused_admin_key_of_the_slot_is_asked_for_again_under_it() {
        let host = host();
        let key = host.key(KEY);
        let given = [GivenKey::new(
            "admin",
            host.given_key("ex-admin-made-up-01"),
        )];

        let need = refused_bill_need(&host, &key, &given);

        let NeedAction::AskKey(ask) = need.action else {
            panic!("a field for the key: {need:?}");
        };
        assert_eq!(ask.slot.as_deref(), Some(ADMIN_SLOT));
    }

    /// A refused key an agent keeps is a need of the part: its button
    /// opens the page of the keys.
    #[test]
    fn a_refused_key_of_an_agent_asks_for_a_new_one() {
        let host = host();
        host.answer("GET", "https://api.example.com/v1/balance", 401, b"{}");
        let key = host.key(KEY);

        let numbers = host.as_provider(PROVIDER, || {
            Example::numbers(PROVIDER, &found(), &account(), &key, &[])
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
