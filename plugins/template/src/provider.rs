//! The provider part `example-cloud`: a made-up provider whose API tells
//! how much money is left on a key. It finds its accounts itself, in the
//! example agent's profile folders OTTY gives it, as its manifest's
//! `logins` declare: the key of `auth.json` in each folder is one account,
//! known by the key's fingerprint, and the same key in two folders is one
//! account with two places.

use otty_plugin_sdk::agents::{
    Account, AccountKind, AccountNumbers, Bill, DayCost, DayRange, Found,
    LoginFolder, Money, Provider, ReadError,
};
use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::{GivenKey, HostError, Key, Need, files, given_key};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{Example, needs};

/// The provider's API.
const API: &str = "https://api.example.com/v1";

/// The agent part whose profiles hold the logins the part reads.
const AGENT: &str = "example-agent";

/// The provider's name, as the texts of its needs say it.
const NAME: &str = "Example Cloud";

/// The page where the user makes a new key.
const KEYS_PAGE: &str = "https://example.com/keys";

/// The slot the admin key is given under: the part's own name for it.
const ADMIN_SLOT: &str = "admin";

/// What an admin key starts with: the key of an organization, whose bill
/// it reads.
const ADMIN_PREFIX: &str = "ex-admin-";

/// The page where the user makes an admin key.
const BILL_KEYS_PAGE: &str = "https://example.com/keys/admin";

/// `auth.json` of a profile of the example agent, its key replaced by a
/// handle.
#[derive(Debug, Deserialize)]
struct AuthFile {
    api_key: Option<KeyMarker>,
    email: Option<String>,
}

/// `{"$key": n}`: where the host put a key.
#[derive(Debug, Deserialize)]
struct KeyMarker {
    #[serde(rename = "$key")]
    index: usize,
}

/// The login of one folder: its key, and the email it names.
struct FolderLogin {
    key: Key,
    email: Option<String>,
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
    /// One account per key of `auth.json` in the folders, known by the
    /// key's fingerprint, which stays the same while the key does. It is
    /// named by the email of the file, else by the end of the
    /// fingerprint. An admin key is an organization. A folder with no
    /// login gives nothing: the agent part says it is not logged in.
    fn accounts(_provider: &str, folders: &[LoginFolder]) -> Found {
        let mut accounts: Vec<Account> = Vec::new();

        for folder in folders {
            let LoginFolder::Profile((agent, path)) = folder;
            if agent != AGENT {
                continue;
            }
            let Some(login) = folder_login(path) else {
                continue;
            };
            let Ok(fingerprint) = login.key.fingerprint() else {
                continue;
            };

            let id = format!("key:{fingerprint}");
            if let Some(known) =
                accounts.iter_mut().find(|known| known.id == id)
            {
                known.found_in.push(folder.clone());
                continue;
            }
            let is_admin = login.key.has_prefix(ADMIN_PREFIX).unwrap_or(false);
            accounts.push(Account {
                id,
                name: login.email.unwrap_or_else(|| short_key(&fingerprint)),
                kind: match is_admin {
                    true => AccountKind::Organization,
                    false => AccountKind::Key,
                },
                plan: None,
                found_in: vec![folder.clone()],
                needs: Vec::new(),
            });
        }

        Found {
            accounts,
            needs: Vec::new(),
        }
    }

    /// The money left on the key of the account, read with the key of
    /// the first of its folders that still holds one. With no key there
    /// now, such as a key taken out of the file, the part has no other way
    /// to read the account, so it answers nothing and sends nothing.
    fn numbers(
        _provider: &str,
        account: &Account,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        let Some(login) = account_login(account) else {
            return Ok(no_numbers());
        };

        let url = format!("{API}/balance");
        let balance: Balance = get_json(&url, &login.key, refused_key_need)?;

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
        account: &Account,
        given: &[GivenKey],
        days: &DayRange,
    ) -> Result<Bill, ReadError> {
        let url = format!("{API}/bill?from={}&to={}", days.first, days.last);
        let own = account_login(account).map(|login| login.key);
        let is_bill_key = match &own {
            Some(key) => is_bill_key(key)?,
            None => false,
        };

        let answer: BillAnswer = match (own, is_bill_key) {
            (Some(key), true) => get_json(&url, &key, refused_key_need)?,
            _ => {
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

/// The login of the first folder of `account` that holds one.
fn account_login(account: &Account) -> Option<FolderLogin> {
    account.found_in.iter().find_map(|folder| {
        let LoginFolder::Profile((agent, path)) = folder;

        (agent == AGENT).then(|| folder_login(path)).flatten()
    })
}

/// The key of `auth.json` in the profile folder `folder`, as a handle,
/// with the email the file names; `None` when the file is not there, not
/// in the shape the part reads, or holds no key.
fn folder_login(folder: &str) -> Option<FolderLogin> {
    let document =
        files::json_with_keys(&format!("{folder}/auth.json")).ok()?;
    let (json, keys) = document.into_parts();
    let auth = serde_json::from_str::<AuthFile>(&json).ok()?;

    let key = keys.into_iter().nth(auth.api_key?.index)?;
    Some(FolderLogin {
        key,
        email: auth.email,
    })
}

/// A key shown by the end of its fingerprint, such as `key ••••a1f3`:
/// the key itself is never read.
fn short_key(fingerprint: &str) -> String {
    let start = fingerprint.len().saturating_sub(4);

    format!("key ••••{}", &fingerprint[start..])
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
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{GivenKey, HostError, NeedAction, files};

    use super::*;

    const PROVIDER: &str = "example-cloud";

    fn host() -> TestHost {
        TestHost::new(env!("CARGO_MANIFEST_DIR"))
    }

    /// The profile folder `folder` of the home folder, of the agent part
    /// `agent`, as OTTY gives it.
    fn profile(host: &TestHost, agent: &str, folder: &str) -> LoginFolder {
        let path = host.home().join(folder).display().to_string();

        LoginFolder::Profile((agent.to_string(), path))
    }

    /// What the part finds in `folders`, in a call given them.
    fn accounts_in(host: &TestHost, folders: &[LoginFolder]) -> Found {
        host.as_provider_in(PROVIDER, folders, || {
            Example::accounts(PROVIDER, folders)
        })
    }

    /// The account the part finds in the profile folder `folder` of the
    /// example agent.
    fn account_in(host: &TestHost, folder: &str) -> Account {
        let folders = [profile(host, AGENT, folder)];

        accounts_in(host, &folders)
            .accounts
            .pop()
            .expect("an account")
    }

    /// An account found in no folder.
    fn account_with_no_folder() -> Account {
        Account {
            id: "key:made-up".to_string(),
            name: "key ••••made".to_string(),
            kind: AccountKind::Key,
            plan: None,
            found_in: Vec::new(),
            needs: Vec::new(),
        }
    }

    /// The numbers of `account`, read in a call given its folders.
    fn numbers_of(
        host: &TestHost,
        account: &Account,
    ) -> Result<AccountNumbers, ReadError> {
        host.as_provider_in(PROVIDER, &account.found_in, || {
            Example::numbers(PROVIDER, account, &[])
        })
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 8: the key
    /// of a profile folder is a key account, named by the email of its
    /// file and known by a stable id.
    #[test]
    fn the_key_of_a_folder_is_an_account_named_by_its_email() {
        let host = host();

        let account = account_in(&host, ".example-agent");

        assert_eq!(account.name, "dev@example.com");
        assert_eq!(account.kind, AccountKind::Key);
        assert!(account.id.starts_with("key:"), "{}", account.id);
        assert_eq!(account.found_in, [profile(&host, AGENT, ".example-agent")]);
        assert_eq!(account, account_in(&host, ".example-agent"));
    }

    /// Two folders with two keys are two accounts; a folder with no key
    /// gives none, and no need: that need is the agent part's.
    #[test]
    fn each_key_is_its_own_account() {
        let host = host();
        let folders = [
            profile(&host, AGENT, ".example-agent"),
            profile(&host, AGENT, ".example-agent-work"),
            profile(&host, AGENT, ".example-agent-nobody"),
        ];

        let found = accounts_in(&host, &folders);

        let names: Vec<&str> = found
            .accounts
            .iter()
            .map(|account| account.name.as_str())
            .collect();
        assert_eq!(names, ["dev@example.com", "dev@work.example.com"]);
        assert_ne!(found.accounts[0].id, found.accounts[1].id);
        assert!(found.needs.is_empty());
    }

    /// One key in two folders is one account with two places.
    #[test]
    fn one_key_in_two_folders_is_one_account() {
        let host = host();
        let folders = [
            profile(&host, AGENT, ".example-agent"),
            profile(&host, AGENT, ".example-agent"),
        ];

        let found = accounts_in(&host, &folders);

        assert_eq!(found.accounts.len(), 1);
        assert_eq!(found.accounts[0].found_in.len(), 2);
    }

    /// A folder of another agent part is not read.
    #[test]
    fn a_folder_of_another_agent_part_gives_no_account() {
        let host = host();
        let folders = [profile(&host, "other-agent", ".example-agent")];

        let found = accounts_in(&host, &folders);

        assert!(found.accounts.is_empty());
    }

    #[test]
    fn a_key_is_named_by_its_fingerprint_when_no_email_names_it() {
        assert_eq!(short_key("27c8d30e"), "key ••••d30e");
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: the part
    /// reads the key of `auth.json` in the account's folder itself, as a
    /// handle, and reads the balance with it.
    #[test]
    fn numbers_read_with_the_key_of_the_accounts_folder() {
        let host = host();
        host.answer(
            "GET",
            "https://api.example.com/v1/balance",
            200,
            br#"{"balance": 12.5, "currency": "USD"}"#,
        );
        let account = account_in(&host, ".example-agent-work");

        let numbers = numbers_of(&host, &account).expect("the numbers");

        assert_eq!(numbers.left.map(|left| left.amount), Some(12.5));
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some("Bearer ex-made-up-key-0002")
        );
    }

    /// Issue 29: an account whose key is not there now is no failure: the
    /// part answers nothing and sends nothing.
    #[test]
    fn an_account_with_no_key_there_reads_nothing() {
        let host = host();

        let numbers = numbers_of(&host, &account_with_no_folder());

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty(), "nothing is sent");
    }

    fn days() -> DayRange {
        DayRange {
            first: "2026-11-01".to_string(),
            last: "2026-11-02".to_string(),
        }
    }

    /// The bill of `account` with the keys of `given`, read in a call
    /// given its folders.
    fn bill_of(
        host: &TestHost,
        account: &Account,
        given: &[GivenKey],
    ) -> Result<Bill, ReadError> {
        host.as_provider_in(PROVIDER, &account.found_in, || {
            Example::read_bill(PROVIDER, account, given, &days())
        })
    }

    /// With no admin key the part asks for one, under its slot, and sends
    /// nothing.
    #[test]
    fn a_bill_with_no_key_for_the_admin_slot_asks_for_one() {
        let host = host();
        let account = account_in(&host, ".example-agent");

        let bill = bill_of(&host, &account, &[]);

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
        let account = account_in(&host, ".example-agent");
        let given = [GivenKey::new("admin", host.key("ex-admin-made-up-01"))];

        let bill = bill_of(&host, &account, &given).expect("the bill");

        assert_eq!(bill.days.len(), 1);
        assert_eq!(bill.days[0].cost.amount, 3.5);
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some("Bearer ex-admin-made-up-01")
        );
    }

    /// A refused admin key given under the slot is asked for again under
    /// that slot.
    #[test]
    fn a_refused_admin_key_of_the_slot_is_asked_for_again_under_it() {
        let host = host();
        host.answer(
            "GET",
            "https://api.example.com/v1/bill?from=2026-11-01&to=2026-11-02",
            401,
            b"{}",
        );
        let account = account_in(&host, ".example-agent");
        let given = [GivenKey::new(
            "admin",
            host.given_key("ex-admin-made-up-01"),
        )];

        let bill = bill_of(&host, &account, &given);

        let Err(ReadError::NeedsUser(need)) = bill else {
            panic!("a need: {bill:?}");
        };
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
        let account = account_in(&host, ".example-agent");

        let numbers = numbers_of(&host, &account);

        let Err(ReadError::NeedsUser(need)) = numbers else {
            panic!("a need: {numbers:?}");
        };
        assert_eq!(need.id, "new-key");
        assert!(need.text.starts_with("Example Cloud refused the key"));
        assert_eq!(need.action, NeedAction::OpenPage(KEYS_PAGE.to_string()));
    }

    #[test]
    fn the_provider_part_may_not_read_the_agents_files_outside_a_call() {
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
