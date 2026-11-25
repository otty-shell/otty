//! The provider part `example-cloud`: a made-up provider whose API tells
//! how much money is left on a key. It finds its accounts itself, in the
//! example agent's profile folders OTTY gives it, as its manifest's
//! `logins` declare: the key of `auth.json` in each folder is one account,
//! known by the key's fingerprint, and the same key in two folders is one
//! account with two places.

use otty_plugin_sdk::agents::{
    Account, AccountKind, AccountNumbers, Bill, DayCost, DayRange, Found,
    LoginFolder, Money, MonthSpend, Provider, ReadError,
};
use otty_plugin_sdk::http::{self, Request};
use otty_plugin_sdk::terminal::HiddenTerminal;
use otty_plugin_sdk::{
    Command, GivenKey, HostError, Key, Need, files, given_key,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::session_file::SessionFile;
use crate::{Example, agent, needs};

/// The provider's API.
const API: &str = "https://api.example.com/v1";

/// The agent's program and its argument that show the limits of its
/// login in a hidden terminal.
const USAGE: [&str; 2] = ["example-agent", "usage"];

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
    /// OTTY's own login folder is read as a profile folder.
    fn accounts(_provider: &str, folders: &[LoginFolder]) -> Found {
        let mut accounts: Vec<Account> = Vec::new();

        for folder in folders {
            let Some(path) = agent_folder(folder) else {
                continue;
            };
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
    /// the first of its folders that still holds one, and what the
    /// sessions of its profile folders spent this month. With no key there
    /// now, the agent is asked: see [`usage_numbers`].
    fn numbers(
        _provider: &str,
        account: &Account,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        let Some(login) = account_login(account) else {
            return Ok(usage_numbers(account));
        };

        let url = format!("{API}/balance");
        let balance: Balance = get_json(&url, &login.key, refused_key_need)?;

        Ok(AccountNumbers {
            left: Some(Money {
                amount: balance.balance,
                currency: balance.currency,
                by_kind: None,
            }),
            spent_this_month: session_spend(account),
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
                    by_kind: None,
                },
            })
            .collect();
        Ok(Bill { days })
    }

    /// Log in with Example Agent (the `log_in` of the manifest's
    /// `logins`): `example-agent login` pointed at OTTY's own folder.
    fn log_in(_provider: &str, folder: &str) -> Option<Command> {
        Some(agent::login_in(folder))
    }
}

/// What the made-up provider tells nothing of: what the example agent's
/// sessions in the profile folders of `account` spent this month, each
/// answer at the API price OTTY's price list gives during `numbers`,
/// kind by kind, marked at API prices by its kinds (issue 22 of spec
/// 2026-11-23-accounts-apart-from-profiles). A provider that tells the
/// spend of a key gives that instead. OTTY's own login folder holds no
/// session to count. `None` when no answer had a price.
fn session_spend(account: &Account) -> Option<Money> {
    let mut month = MonthSpend::this_month();

    for folder in &account.found_in {
        let LoginFolder::Profile((agent, path)) = folder else {
            continue;
        };
        if agent != AGENT {
            continue;
        }

        let Ok(files) = files::list(&format!("{path}/sessions/*.jsonl")) else {
            continue;
        };
        for file in files {
            if file.changed_ms < month.since_ms() {
                continue;
            }
            let Ok(session) = SessionFile::read(&file.path) else {
                continue;
            };
            for answer in session.into_answers("").answers {
                month.add(&answer);
            }
        }
    }

    month.money()
}

/// The numbers of an account none of whose folders holds a key: in the
/// first folder whose `auth.json` names no key, such as a login the agent
/// keeps elsewhere, `example-agent usage` runs in a hidden terminal
/// pointed at that folder, and its screen is the plan. OTTY starts the
/// terminal only for a read the user sees, and only when the manifest
/// lists the program under `[[agents.provider.permissions.terminal]]`,
/// which the tests that need it add. With no such folder, or no
/// terminal, the part answers nothing and sends nothing.
fn usage_numbers(account: &Account) -> AccountNumbers {
    let Some(folder) = account
        .found_in
        .iter()
        .filter_map(agent_folder)
        .find(|folder| is_keyless(folder))
    else {
        return no_numbers();
    };

    match usage_screen(folder) {
        Ok(screen) => AccountNumbers {
            plan: Some(screen.trim().to_string()),
            ..no_numbers()
        },
        Err(_) => no_numbers(),
    }
}

/// Whether `auth.json` of `folder` is there and names no key.
fn is_keyless(folder: &str) -> bool {
    let Ok(document) = files::json_with_keys(&format!("{folder}/auth.json"))
    else {
        return false;
    };

    serde_json::from_str::<AuthFile>(document.json())
        .is_ok_and(|auth| auth.api_key.is_none())
}

/// The screen of `example-agent usage` in a hidden terminal pointed at
/// `folder`.
fn usage_screen(folder: &str) -> Result<String, HostError> {
    let env = vec![(agent::HOME_VARIABLE.to_string(), folder.to_string())];
    let terminal =
        HiddenTerminal::start(USAGE[0], &[USAGE[1].to_string()], &env, 80, 24)?;

    terminal.wait_for_change(2000)?;
    terminal.screen()
}

/// The login of the first folder of `account` that holds one.
fn account_login(account: &Account) -> Option<FolderLogin> {
    account
        .found_in
        .iter()
        .filter_map(agent_folder)
        .find_map(folder_login)
}

/// The path of `folder` when it holds a login of the example agent: a
/// profile folder of it, or OTTY's own login folder, where the example
/// agent logged in.
fn agent_folder(folder: &LoginFolder) -> Option<&str> {
    match folder {
        LoginFolder::Profile((agent, path)) if agent == AGENT => Some(path),
        LoginFolder::OwnLogin((_, path)) => Some(path),
        _ => None,
    }
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
