//! The provider part `anthropic`: the accounts of Anthropic found in the
//! Claude Code profiles OTTY gives it (`logins.rs`), the limits of a
//! Claude login, and the bill of an Anthropic organization read with an
//! admin key. The part reads the login files itself, only in the folders
//! of the call, and gets every key as a handle, never the key.
//!
//! The limits are read any way the part can (issue 29), only in the
//! folders the account was found in: from Anthropic's server with the
//! login token of one of them; else, with no token there or when the
//! server fails, from the `/usage` panel of `claude` in that folder; else
//! nothing. A token Anthropic refuses is the need that runs `claude` in
//! its folder, which renews the login or asks to log in again.
//!
//! OTTY's own login is read first (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 7): with its token
//! past its time nothing is sent, and the `/usage` panel of `claude` in
//! that folder makes Claude Code renew its own login. The part never
//! renews a token. A need there is "Log in again" (Decision 9).

use otty_plugin_sdk::agents::{
    Account, AccountKind, AccountNumbers, Bill, DayRange, Found, LoginFolder,
    Provider, ReadError,
};
use otty_plugin_sdk::{Command, GivenKey, HostError, Key, given_key};

use crate::claude_code::{claude_in_folder, claude_login_in};
use crate::logins::{Access, claude_folder, logins};
use crate::needs::{self, ADMIN_PREFIX, ADMIN_SLOT};
use crate::session_spend::session_spend;
use crate::{Anthropic, cost_report, oauth_usage, usage_probe};

impl Provider for Anthropic {
    /// The accounts of the logins in `folders`. One account found in two
    /// folders is one account with two places and the needs of both. A
    /// folder with no login gives nothing: the agent part says it is not
    /// logged in.
    fn accounts(_provider: &str, folders: &[LoginFolder]) -> Found {
        let mut accounts: Vec<Account> = Vec::new();

        for folder in folders {
            for login in logins(folder) {
                let (account, _) = login.into_parts();
                add_account(&mut accounts, account);
            }
        }

        Found {
            accounts,
            needs: Vec::new(),
        }
    }

    /// The windows and the extra usage of a Claude login, read in one of
    /// the account's folders: from Anthropic's server with its token, else
    /// from the `/usage` panel there, else nothing. Anthropic tells
    /// nothing of the money of one API key, so for a key or an
    /// organization the part counts what the Claude Code sessions of its
    /// folders spent this month at API prices (issue 22), and sends
    /// nothing for a key. A failed read of the server for an account paid
    /// per token gives that spend all the same; a need comes first.
    fn numbers(
        _provider: &str,
        account: &Account,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        let spent = session_spend(account);

        match login_numbers(account) {
            Ok(numbers) => Ok(AccountNumbers {
                spent_this_month: spent,
                ..numbers
            }),
            Err(ReadError::NeedsUser(need)) => Err(ReadError::NeedsUser(need)),
            Err(error) => match spent {
                Some(spent) => Ok(AccountNumbers {
                    spent_this_month: Some(spent),
                    ..no_numbers()
                }),
                None => Err(error),
            },
        }
    }

    /// The cost report of the organization over `days`, read with an
    /// admin key: the one the user gave under [`ADMIN_SLOT`], else the
    /// account's own key when it is one, else the part asks for one and
    /// sends nothing. A refused key is asked for again under
    /// [`ADMIN_SLOT`], which the next read takes first.
    fn read_bill(
        _provider: &str,
        account: &Account,
        given: &[GivenKey],
        days: &DayRange,
    ) -> Result<Bill, ReadError> {
        if let Some(admin_key) = given_key(given, ADMIN_SLOT) {
            return cost_report::bill(admin_key, days);
        }

        let own_key = match account_access(account) {
            Some((_, Access::Key(key))) => Some(key),
            _ => None,
        };
        let Some(own_key) = own_key else {
            return Err(ReadError::NeedsUser(needs::bill_key()));
        };
        if !is_bill_key(&own_key)? {
            return Err(ReadError::NeedsUser(needs::bill_key()));
        }

        cost_report::bill(&own_key, days)
    }

    /// Log in with Claude Code (spec 2026-11-23-accounts-apart-from-profiles,
    /// Decision 6): `claude auth login --claudeai` with `CLAUDE_CONFIG_DIR`
    /// set to OTTY's own folder, which the user runs in a terminal tab.
    fn log_in(_provider: &str, folder: &str) -> Option<Command> {
        Some(claude_login_in(folder))
    }
}

/// What the login of `account` tells in one of its folders: the windows
/// and the extra usage of a Claude login, read from Anthropic's server
/// with its token, else from the `/usage` panel there, else nothing. A key
/// tells nothing and sends nothing.
fn login_numbers(account: &Account) -> Result<AccountNumbers, ReadError> {
    let Some((folder, access)) = account_access(account) else {
        return Ok(no_numbers());
    };
    let Some(path) = claude_folder(&folder) else {
        return Ok(no_numbers());
    };

    match (access, &folder) {
        (Access::Claude { token, is_past }, LoginFolder::OwnLogin(_)) => {
            let token = token.filter(|_| !is_past);
            own_login_numbers(account.kind, path, token.as_ref())
        },
        (Access::Claude { token, .. }, _) => {
            claude_numbers(account.kind, path, token.as_ref())
        },
        (Access::Key(_), _) => Ok(no_numbers()),
    }
}

/// Adds `account`, found in one folder, to `accounts`: an account with
/// the same id gets its places and needs, each once.
fn add_account(accounts: &mut Vec<Account>, account: Account) {
    let Some(known) = accounts.iter_mut().find(|known| known.id == account.id)
    else {
        accounts.push(account);
        return;
    };

    for folder in account.found_in {
        if !known.found_in.contains(&folder) {
            known.found_in.push(folder);
        }
    }
    for need in account.needs {
        if !known.needs.contains(&need) {
            known.needs.push(need);
        }
    }
}

/// The folder to read `account` in and how: the first of its folders
/// whose login holds a key or a token, else the first that still holds
/// its login, OTTY's own logins before the profile folders, so reading
/// the numbers keeps OTTY's own login fresh (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 7). `None` when none
/// of its folders holds it now.
fn account_access(account: &Account) -> Option<(LoginFolder, Access)> {
    let is_own =
        |folder: &&LoginFolder| matches!(folder, LoginFolder::OwnLogin(_));
    let own = account.found_in.iter().filter(is_own);
    let profiles = account.found_in.iter().filter(|folder| !is_own(folder));
    let mut without_token: Option<(LoginFolder, Access)> = None;

    for folder in own.chain(profiles) {
        let Some(login) = logins(folder)
            .into_iter()
            .find(|login| login.account().id == account.id)
        else {
            continue;
        };

        let (_, access) = login.into_parts();
        match access {
            Access::Claude { token: None, .. } => {
                without_token.get_or_insert((folder.clone(), access));
            },
            _ => return Some((folder.clone(), access)),
        }
    }

    without_token
}

/// The numbers of OTTY's own Claude login in its folder `folder`, as
/// [`claude_numbers`] reads them with `token`, the login token while it
/// is not past its time. With no such token nothing is sent: the
/// `/usage` probe runs Claude Code there, which renews its own login, as
/// it does after the server refused the token. A need is "Log in again",
/// Claude Code's login in that folder (Decision 9).
fn own_login_numbers(
    kind: AccountKind,
    folder: &str,
    token: Option<&Key>,
) -> Result<AccountNumbers, ReadError> {
    claude_numbers(kind, folder, token).map_err(|error| match error {
        ReadError::NeedsUser(_) => {
            ReadError::NeedsUser(needs::log_in_again(claude_login_in(folder)))
        },
        other => other,
    })
}

/// The numbers of a Claude login of the kind `kind` in the profile folder
/// `folder`: from Anthropic's server with its `token`, else, for a
/// subscription, from the `/usage` panel of `claude` there. A need of the
/// panel, such as a login it asks for, comes first; else the server's
/// error, which for a refused token is the need that runs `claude` in
/// the folder.
fn claude_numbers(
    kind: AccountKind,
    folder: &str,
    token: Option<&Key>,
) -> Result<AccountNumbers, ReadError> {
    let claude = claude_in_folder(folder);

    let server_error =
        match token.map(|token| oauth_usage::claude_usage(token, &claude)) {
            Some(Ok(usage)) => {
                return Ok(AccountNumbers {
                    windows: usage.windows,
                    extra_usage: usage.extra_usage,
                    ..no_numbers()
                });
            },
            Some(Err(error)) => Some(error),
            None => None,
        };

    // Only a subscription has limits in the `/usage` panel.
    if kind != AccountKind::Subscription {
        return match server_error {
            Some(error) => Err(error),
            None => Ok(no_numbers()),
        };
    }

    match usage_probe::read_windows(folder, &claude) {
        Ok(windows) => Ok(AccountNumbers {
            windows,
            ..no_numbers()
        }),
        Err(ReadError::NeedsUser(need)) => Err(ReadError::NeedsUser(need)),
        Err(probe_error) => Err(server_error.unwrap_or(probe_error)),
    }
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

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::http::{self, Request};
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{
        GivenKey, JsonField, KeyPlace, NeedAction, files, keys,
    };

    use super::*;
    use crate::profiles::CONFIG_VARIABLE;
    use crate::testing::{
        OWN_FOLDER, PROVIDER, as_provider_in, host, login_folder,
        own_login_folder, path,
    };

    /// A time after the token of OTTY's own login of the fixtures, which
    /// expired on 2025-11-16, and before the other made-up tokens expire.
    const AFTER_OWN_EXPIRY_MS: i64 = 1_800_000_000_000;
    /// The session file a `/usage` probe left in OTTY's own login.
    const OWN_PROBE_SESSION: &str =
        "projects/-home-dev--cache-otty-agent-probe/p1.jsonl";

    const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
    /// The address of the cost report of 2026-11-16, the one day of
    /// [`one_day`].
    const REPORT_URL: &str = "https://api.anthropic.com/v1/organizations/cost_report?starting_at=2026-11-16T00:00:00Z&ending_at=2026-11-17T00:00:00Z&bucket_width=1d&limit=31";
    /// A made-up admin key, never a real one.
    const MADE_UP_ADMIN: &str = "sk-ant-admin01-made-up-for-tests";
    /// The made-up admin key of the settings of `.claude-admin`.
    const OWN_ADMIN: &str = "sk-ant-admin01-made-up-own-key";
    /// The working folder of the hidden terminals of the tests.
    const PROBE_FOLDER: &str = "/home/dev/.cache/otty/agent-probe";
    /// The arguments the probe starts `claude` with.
    const PROBE_ARGS: [&str; 5] = [
        "--allowed-tools",
        "",
        "--strict-mcp-config",
        "--settings",
        r#"{"remoteControlAtStartup":false,"disableAllHooks":true}"#,
    ];

    fn one_day() -> DayRange {
        DayRange {
            first: "2026-11-16".to_string(),
            last: "2026-11-16".to_string(),
        }
    }

    /// What the part finds in the folders `names` of `host`, in a call
    /// given them.
    fn found_in(host: &TestHost, names: &[&str]) -> Found {
        let folders: Vec<LoginFolder> =
            names.iter().map(|name| login_folder(host, name)).collect();

        as_provider_in(host, &folders, || {
            Anthropic::accounts(PROVIDER, &folders)
        })
    }

    /// The first account the part finds in the folders `names` of `host`.
    fn account_in(host: &TestHost, names: &[&str]) -> Account {
        found_in(host, names)
            .accounts
            .into_iter()
            .next()
            .expect("an account")
    }

    /// The numbers of `account`, read in a call given its folders.
    fn numbers_of(
        host: &TestHost,
        account: &Account,
    ) -> Result<AccountNumbers, ReadError> {
        as_provider_in(host, &account.found_in, || {
            Anthropic::numbers(PROVIDER, account, &[])
        })
    }

    /// The bill of `account` with the keys of `given`, read in a call
    /// given its folders.
    fn bill_of(
        host: &TestHost,
        account: &Account,
        given: &[GivenKey],
    ) -> Result<Bill, ReadError> {
        as_provider_in(host, &account.found_in, || {
            Anthropic::read_bill(PROVIDER, account, given, &one_day())
        })
    }

    /// The screens of `claude` in a hidden terminal: its prompt, then the
    /// `/usage` panel with a five-hour window 29 % used.
    fn usage_screens(host: &TestHost) {
        let prompt = "> Try \"how does <filepath> work?\"\n  ? for shortcuts\n";
        let panel = "> /usage\n Settings:  Status   Config   Usage\n\n Current session\n \u{2588}\u{2588}\u{2588}                29% used\n Resets 5pm (Europe/Berlin)\n\n Esc to exit\n";
        host.answer_terminal("claude", &PROBE_ARGS, &[prompt, panel]);
    }

    /// The variable and its value a started program got for the config
    /// folder, if any.
    fn config_folder_of(host: &TestHost) -> Option<String> {
        host.started()[0]
            .env()
            .iter()
            .find(|(name, _)| name == CONFIG_VARIABLE)
            .map(|(_, value)| value.clone())
    }

    /// An account found in no folder.
    fn account_with_no_folder() -> Account {
        Account {
            id: "claude:made-up".to_string(),
            name: "dev@example.com".to_string(),
            kind: AccountKind::Subscription,
            plan: None,
            found_in: Vec::new(),
            needs: Vec::new(),
        }
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 8: one Claude
    /// account logged in to two folders is one account with two places.
    #[test]
    fn one_login_in_two_folders_is_one_account_with_two_places() {
        let host = host();

        let found = found_in(&host, &[".claude-helper", ".claude-account"]);

        assert_eq!(found.accounts.len(), 1);
        let account = &found.accounts[0];
        assert_eq!(account.id, "claude:00000000-0000-4000-8000-00000000000a");
        assert_eq!(account.name, "dev@example.com, Example Org");
        assert_eq!(
            account.found_in,
            [
                login_folder(&host, ".claude-helper"),
                login_folder(&host, ".claude-account")
            ]
        );
    }

    /// Two accounts in two folders stay two; a key of the settings is an
    /// account of its own.
    #[test]
    fn each_account_and_key_is_its_own_account() {
        let host = host();

        let found = found_in(&host, &[".claude-mixed", ".claude-work"]);

        let names: Vec<&str> = found
            .accounts
            .iter()
            .map(|account| account.name.as_str())
            .collect();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "dev@example.com, Example Org");
        assert!(names[1].starts_with("key ••••"), "{}", names[1]);
        assert_eq!(names[2], "own@example.com, Example Org");
    }

    /// One key in two places is one account.
    #[test]
    fn one_key_in_two_folders_is_one_account() {
        let host = host();

        let found = found_in(&host, &[".claude-console", ".claude-api"]);

        assert_eq!(found.accounts.len(), 1);
        assert_eq!(found.accounts[0].found_in.len(), 2);
    }

    /// Decision 9: a folder with no login gives neither an account nor a
    /// need; that need is the agent part's. A DeepSeek key is no
    /// Anthropic account.
    #[test]
    fn folders_with_no_anthropic_login_give_nothing() {
        let host = host();

        let found = found_in(&host, &[".claude-nothing", ".claude-deepseek"]);

        assert_eq!(found.accounts, []);
        assert_eq!(found.needs, []);
    }

    /// Decision 5: the part reads the login only in the folders of the
    /// call; with none it finds nothing.
    #[test]
    fn a_call_with_no_folder_finds_nothing() {
        let host = host();

        let found = found_in(&host, &[]);

        assert_eq!(found.accounts, []);
    }

    /// The token of the account's folder reads the limits, from the folder
    /// that holds one.
    #[test]
    fn a_claude_login_reads_its_limits_with_the_token_of_its_folder() {
        let host = host();
        host.answer(
            "GET",
            USAGE_URL,
            200,
            br#"{"five_hour": {"utilization": 42.0, "resets_at": null},
                 "extra_usage": {"is_enabled": true, "used_credits": 830}}"#,
        );
        let account = account_in(&host, &[".claude-renewed"]);

        let numbers = numbers_of(&host, &account).expect("the numbers");

        assert_eq!(numbers.windows.len(), 1);
        assert_eq!(numbers.extra_usage.and_then(|extra| extra.used), Some(8.3));
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some("Bearer made-up-access-token-0003")
        );
        assert!(host.started().is_empty());
    }

    /// Issue 29, the owner's case: Claude Code keeps no token in the
    /// folder's `.credentials.json`, yet works; the part reads the limits
    /// from the `/usage` panel of `claude` pointed at that folder, and
    /// nothing fails.
    #[test]
    fn a_login_with_no_token_reads_the_usage_panel_in_its_folder() {
        let host = host().with_terminal_folder(PROBE_FOLDER);
        usage_screens(&host);
        let account = account_in(&host, &[".claude-account"]);

        let numbers = numbers_of(&host, &account).expect("the numbers");

        assert_eq!(numbers.windows.len(), 1);
        assert_eq!(numbers.windows[0].percent_used, 29.0);
        assert!(host.requests().is_empty(), "nothing is sent");
        assert_eq!(host.started().len(), 1);
        assert_eq!(
            config_folder_of(&host),
            Some(path(&host, ".claude-account"))
        );
    }

    /// Issue 29: the server failed, so the part asks Claude Code in the
    /// folder of the token. Here it is the default folder, which
    /// `CLAUDE_CONFIG_DIR` names, so `claude` gets no variable.
    #[test]
    fn a_failed_server_falls_back_on_the_usage_panel_of_the_folder() {
        let host = host();
        let work = path(&host, ".claude-work");
        let host = host
            .with_var(CONFIG_VARIABLE, &work)
            .with_terminal_folder(PROBE_FOLDER);
        host.answer("GET", USAGE_URL, 500, b"{}");
        usage_screens(&host);
        let account = account_in(&host, &[".claude-work"]);

        let numbers = numbers_of(&host, &account);

        assert_eq!(numbers.map(|numbers| numbers.windows.len()), Ok(1));
        assert_eq!(host.requests().len(), 1);
        assert_eq!(config_folder_of(&host), None);
    }

    /// Decision 9: a token Anthropic refuses, which the `/usage` panel
    /// could not get past either, is the need that runs `claude` in the
    /// folder the token was read in.
    #[test]
    fn a_refused_token_needs_claude_in_its_folder() {
        let host = host();
        host.answer("GET", USAGE_URL, 401, b"{}");
        let account = account_in(&host, &[".claude-work"]);

        let numbers = numbers_of(&host, &account);

        let Err(ReadError::NeedsUser(need)) = numbers else {
            panic!("a need: {numbers:?}");
        };
        assert_eq!(need.id, "login-expired");
        let NeedAction::Run(command) = need.action else {
            panic!("a command: {need:?}");
        };
        assert_eq!(command.program, "claude");
        assert_eq!(
            command.env,
            [(CONFIG_VARIABLE.to_string(), path(&host, ".claude-work"))]
        );
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles, Decision
    /// 7: OTTY's own login is read first, and with its token past its
    /// time the part sends nothing and never renews it: the `/usage` probe
    /// runs Claude Code in the own folder, which renews its own login, and
    /// the session files the probe left there are removed.
    #[test]
    fn an_own_login_past_its_time_is_kept_fresh_by_claude_code_itself() {
        let host = host()
            .with_now_ms(AFTER_OWN_EXPIRY_MS)
            .with_terminal_folder(PROBE_FOLDER);
        usage_screens(&host);
        let folders = [
            login_folder(&host, ".claude-renewed"),
            own_login_folder(&host),
        ];
        let account = as_provider_in(&host, &folders, || {
            Anthropic::accounts(PROVIDER, &folders)
        })
        .accounts
        .remove(0);

        let numbers = numbers_of(&host, &account).expect("the numbers");

        assert_eq!(account.found_in, folders);
        assert_eq!(numbers.windows[0].percent_used, 29.0);
        assert!(host.requests().is_empty(), "nothing is sent");
        assert_eq!(config_folder_of(&host), Some(path(&host, OWN_FOLDER)));
        assert_eq!(
            host.removed(),
            [std::path::PathBuf::from(path(
                &host,
                &format!("{OWN_FOLDER}/{OWN_PROBE_SESSION}")
            ))]
        );
    }

    /// Issue 09, Decision 9: Anthropic refused the token of OTTY's own
    /// login and the probe could not get past it either: the account needs
    /// "Log in again", which runs Claude Code's login in that folder.
    #[test]
    fn a_refused_own_login_needs_log_in_again() {
        let host = host();
        host.answer("GET", USAGE_URL, 401, b"{}");
        let folder = path(&host, ".claude-renewed");
        let own = LoginFolder::OwnLogin(("a1".to_string(), folder.clone()));
        let account = as_provider_in(&host, std::slice::from_ref(&own), || {
            Anthropic::accounts(PROVIDER, std::slice::from_ref(&own))
        })
        .accounts
        .remove(0);

        let numbers = numbers_of(&host, &account);

        let Err(ReadError::NeedsUser(need)) = numbers else {
            panic!("a need: {numbers:?}");
        };
        assert_eq!(need.id, "log-in-again");
        let NeedAction::Run(command) = need.action else {
            panic!("a command: {need:?}");
        };
        assert_eq!(command.args, ["auth", "login", "--claudeai"]);
        assert_eq!(command.env, [(CONFIG_VARIABLE.to_string(), folder)]);
    }

    /// Issue 29: an account none of whose folders is given reads nothing,
    /// and nothing is sent or started.
    #[test]
    fn an_account_found_in_no_folder_reads_nothing() {
        let host = host();

        let numbers = numbers_of(&host, &account_with_no_folder());

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
        assert!(host.started().is_empty());
    }

    /// Issue 22: an API key has no numbers of Anthropic's but its spend
    /// this month by its sessions, nothing in a folder with none, and
    /// nothing is sent.
    #[test]
    fn an_api_key_has_only_its_spend_and_sends_nothing() {
        let host = host();
        let account = account_in(&host, &[".claude-api"]);

        let numbers = numbers_of(&host, &account).expect("the numbers");

        let spent = numbers.spent_this_month.clone().expect("the spend");
        assert_eq!(spent.by_kind.map(|cost| cost.total), Some(0));
        assert_eq!(
            numbers,
            AccountNumbers {
                spent_this_month: Some(spent),
                ..no_numbers()
            }
        );
        assert!(host.requests().is_empty());
    }

    /// The bill reads only with an admin key: a Console login is no admin
    /// key, and with no admin key given the part asks for one under the
    /// slot `admin` and sends nothing.
    #[test]
    fn a_bill_with_no_admin_key_asks_for_one() {
        let host = host();
        let account = account_in(&host, &[".claude-prepaid"]);

        let bill = bill_of(&host, &account, &[]);

        let Err(ReadError::NeedsUser(need)) = bill else {
            panic!("a need: {bill:?}");
        };
        assert_eq!(need.id, "bill-key");
        assert_eq!(need.title, "The real bill needs an admin key");
        let NeedAction::AskKey(ask) = need.action else {
            panic!("a key to ask for: {need:?}");
        };
        assert_eq!(ask.slot.as_deref(), Some("admin"));
        assert_eq!(ask.label, "Admin key");
        assert_eq!(ask.prefix.as_deref(), Some("sk-ant-admin"));
        assert_eq!(
            ask.page.as_deref(),
            Some("https://console.anthropic.com/settings/admin-keys")
        );
        assert!(host.requests().is_empty());
    }

    /// The admin key the user gave under `admin` reads the bill.
    #[test]
    fn a_key_given_under_admin_reads_the_bill() {
        let host = host();
        host.answer(
            "GET",
            REPORT_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let account = account_in(&host, &[".claude-prepaid"]);
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        let bill = bill_of(&host, &account, &given);

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests()[0].header("x-api-key"), Some(MADE_UP_ADMIN));
    }

    /// An account whose own key is an admin key reads its bill with it.
    #[test]
    fn an_admin_key_of_the_settings_reads_its_own_bill() {
        let host = host();
        host.answer(
            "GET",
            REPORT_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let account = account_in(&host, &[".claude-admin"]);

        let bill = bill_of(&host, &account, &[]);

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests()[0].header("x-api-key"), Some(OWN_ADMIN));
    }

    /// The admin key given under `admin` reads the bill before the
    /// account's own admin key, which was refused before it was given.
    #[test]
    fn a_key_given_under_admin_reads_the_bill_before_the_accounts_own() {
        let host = host();
        host.answer(
            "GET",
            REPORT_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let account = account_in(&host, &[".claude-admin"]);
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        let bill = bill_of(&host, &account, &given);

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests()[0].header("x-api-key"), Some(MADE_UP_ADMIN));
    }

    /// Issue 26: an admin key an agent keeps is not OTTY's to replace: a
    /// refused one is asked for again under `admin`, and so is a refused
    /// key given there.
    #[test]
    fn a_refused_admin_key_is_asked_for_under_admin() {
        let host = host();
        host.answer("GET", REPORT_URL, 401, b"{}");
        let account = account_in(&host, &[".claude-admin"]);
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        for given in [&[][..], &given[..]] {
            let bill = bill_of(&host, &account, given);

            let Err(ReadError::NeedsUser(need)) = bill else {
                panic!("a need: {bill:?}");
            };
            let NeedAction::AskKey(ask) = need.action else {
                panic!("a key to ask for: {need:?}");
            };
            assert_eq!(ask.slot.as_deref(), Some(ADMIN_SLOT));
        }
    }

    /// Spec 2026-11-19-agent-plugins, Decision 7: a key handle goes only
    /// into a request to a host the part declares.
    #[test]
    fn the_part_may_call_no_other_host() {
        let host = host();
        let key = host.key("made-up-access-token");

        let sent = host.as_provider(PROVIDER, || {
            let request = Request::get("https://api.deepseek.com/user/balance")
                .header_with_key("authorization", "Bearer ", &key);
            http::request(&request)
        });

        assert_eq!(sent, Err(HostError::NotAllowed("api.deepseek.com".into())));
        assert!(host.requests().is_empty());
    }

    /// Decision 5: outside a call given the folder, the files and the key
    /// places of the plugin's `claude-code` part are refused.
    #[test]
    fn the_part_may_not_read_a_login_outside_the_folders_of_its_call() {
        let host = host();
        let credentials = path(&host, ".claude-work/.credentials.json");

        let (read, opened) = host.as_provider(PROVIDER, || {
            let read = files::json_with_keys(&credentials).map(|_| ());
            let opened = keys::open(&KeyPlace::JsonField(JsonField {
                file: credentials.clone(),
                pointer: "/claudeAiOauth/accessToken".to_string(),
            }));
            (read, opened.map(|_| ()))
        });

        assert!(matches!(read, Err(HostError::NotAllowed(_))), "{read:?}");
        assert!(
            matches!(opened, Err(HostError::NotAllowed(_))),
            "{opened:?}"
        );
    }
}
