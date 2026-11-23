//! The provider part `openai`: the OpenAI accounts of the Codex profiles
//! OTTY gives it (`accounts.rs`), the limits and credits of a ChatGPT
//! account, and the bill of an OpenAI organization read with an admin key
//! the part asks the user for. It reads only in the folders of the call,
//! and gets every key as a handle.
//!
//! The limits are read any way the part can (issue 29), only in the
//! account's folders: from `chatgpt.com` with the token of the first of
//! them that holds the account's login; else, when the server fails, from
//! what Codex knows there (`codex_limits.rs`); else, with no login in any
//! of them now, from what Codex wrote in a folder that holds no login;
//! else nothing.

use otty_plugin_sdk::agents::{
    Account, AccountKind, AccountNumbers, Bill, DayRange, Found, LoginFolder,
    Provider, ReadError,
};
use otty_plugin_sdk::{GivenKey, given_key};

use crate::accounts::{
    ChatgptLogin, FolderLogin, KeyLogin, codex_folders, has_no_login,
    login_need,
};
use crate::codex_limits::read_codex_limits;
use crate::limits::Limits;
use crate::needs::{self, ADMIN_SLOT};
use crate::{OpenAi, accounts, chatgpt_usage, organization_costs};

impl Provider for OpenAi {
    /// The ChatGPT logins and API keys of the Codex profile folders: a
    /// ChatGPT login is a subscription known by its account id, a key is
    /// known by its fingerprint, and an admin key is an organization.
    fn accounts(_provider: &str, folders: &[LoginFolder]) -> Found {
        accounts::accounts(folders)
    }

    /// The limits and credits of a ChatGPT account: from `chatgpt.com`
    /// with the token of its login in the first of its folders that
    /// holds it; else what Codex knows in one of its folders; else
    /// nothing. A refused or expired login needs the user to log in again
    /// in that folder. OpenAI tells nothing of the money of one API key,
    /// so a key has no numbers and nothing is sent.
    fn numbers(
        _provider: &str,
        account: &Account,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        if account.kind != AccountKind::Subscription {
            return Ok(no_numbers());
        }

        let folders = codex_folders(account);
        let logins = chatgpt_logins(account, &folders);

        let server = logins.iter().find_map(|(folder, login)| {
            let token = login.token()?;

            Some(chatgpt_usage::chatgpt_limits(
                token,
                login.account_id(),
                &login_need(folder),
            ))
        });

        let limits = match server {
            Some(Ok(limits)) => Ok(limits),
            Some(Err(error)) => match codex_limits(&folders, &logins) {
                Some(Ok(limits)) => Ok(limits),
                _ => Err(error),
            },
            None => match codex_limits(&folders, &logins) {
                Some(Ok(limits)) => Ok(limits),
                _ => return Ok(no_numbers()),
            },
        };

        limits.map(Limits::into_numbers)
    }

    /// The costs of the organization over `days`, read with an admin
    /// key: the one the user gave under [`ADMIN_SLOT`], else the
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
            return organization_costs::bill(admin_key, days);
        }

        let own = own_admin_key(account);
        let Some(admin_key) = own.as_ref().and_then(KeyLogin::key) else {
            return Err(ReadError::NeedsUser(needs::bill_key()));
        };

        organization_costs::bill(admin_key, days)
    }
}

/// The ChatGPT logins of `account` in its Codex profile folders
/// `folders`, each with its folder. A folder that holds another login now
/// is left out.
fn chatgpt_logins<'folder>(
    account: &Account,
    folders: &[&'folder str],
) -> Vec<(&'folder str, ChatgptLogin)> {
    folders
        .iter()
        .filter_map(|folder| match login_of(account, folder)? {
            FolderLogin::Chatgpt(login) => Some((*folder, login)),
            FolderLogin::Key(_) => None,
        })
        .collect()
}

/// The login of `account` in the Codex profile folder `folder`; `None`
/// when the folder holds no login of it now.
fn login_of(account: &Account, folder: &str) -> Option<FolderLogin> {
    FolderLogin::read(folder).filter(|login| login.id() == account.id)
}

/// What Codex knows of the limits: in the first folder of `logins`, with
/// its token; else, with no login of the account in any folder now, what
/// Codex wrote in the first of `folders` that holds no login. `None` when
/// there is no such folder.
fn codex_limits(
    folders: &[&str],
    logins: &[(&str, ChatgptLogin)],
) -> Option<Result<Limits, ReadError>> {
    if let Some((folder, login)) = logins.first() {
        return Some(read_codex_limits(folder, login.token()));
    }

    let folder = folders.iter().find(|folder| has_no_login(folder))?;
    Some(read_codex_limits(folder, None))
}

/// The admin key of `account` in the first of its folders that holds it.
fn own_admin_key(account: &Account) -> Option<KeyLogin> {
    codex_folders(account).into_iter().find_map(|folder| {
        match login_of(account, folder)? {
            FolderLogin::Key(login) if login.is_admin() => Some(login),
            _ => None,
        }
    })
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
        GivenKey, HostError, JsonField, KeyPlace, Need, NeedAction, files, keys,
    };

    use super::*;
    use crate::app_server::APP_SERVER_ARGS;
    use crate::testing::{ACCOUNT, PROVIDER, TEN, codex_folder, host, path};

    /// The access token of `.codex-plus`, made up.
    const PLUS_TOKEN: &str =
        "e30.eyJleHAiOjQxMDI0NDQ4MDAsInN1YiI6Im1hZGUtdXAtYWNjZXNzIn0.c2ln";
    /// The access token of `.codex-renewed`, made up.
    const RENEWED_TOKEN: &str = "e30.eyJleHAiOjQxMDI0NDQ4MDAsInN1YiI6Im1hZGUtdXAtYWNjZXNzLXJlbmV3ZWQifQ.c2ln";
    const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
    const USAGE_ANSWER: &[u8] = br#"{"plan_type": "pro", "rate_limit": {"primary_window": {"used_percent": 42, "limit_window_seconds": 18000}}}"#;
    /// The address of the costs of 2026-11-16, the one day of [`one_day`].
    const COSTS_URL: &str = "https://api.openai.com/v1/organization/costs?start_time=1794787200&end_time=1794873600&bucket_width=1d&limit=31";
    /// A made-up admin key, never a real one: the key of `.codex-admin`.
    const MADE_UP_ADMIN: &str = "sk-admin-made-up-for-tests";
    /// Another made-up admin key, given by the user.
    const GIVEN_ADMIN: &str = "sk-admin-made-up-given";

    fn one_day() -> DayRange {
        DayRange {
            first: "2026-11-16".to_string(),
            last: "2026-11-16".to_string(),
        }
    }

    /// The accounts the part finds in the Codex profile folders `names`.
    fn accounts_in(host: &TestHost, names: &[&str]) -> Vec<Account> {
        let folders: Vec<LoginFolder> =
            names.iter().map(|name| codex_folder(host, name)).collect();

        host.as_provider_in(PROVIDER, &folders, || {
            OpenAi::accounts(PROVIDER, &folders)
        })
        .accounts
    }

    /// The one account the part finds in the Codex profile folder `name`.
    fn account_in(host: &TestHost, name: &str) -> Account {
        accounts_in(host, &[name]).remove(0)
    }

    /// The ChatGPT account [`ACCOUNT`] as OTTY knows it from an earlier
    /// read, found in the Codex profile folders `names`.
    fn known_account(host: &TestHost, names: &[&str]) -> Account {
        Account {
            id: format!("chatgpt:{ACCOUNT}"),
            name: "dev@example.com".to_string(),
            kind: AccountKind::Subscription,
            plan: Some("Pro 200".to_string()),
            found_in: names
                .iter()
                .map(|name| codex_folder(host, name))
                .collect(),
            needs: Vec::new(),
        }
    }

    /// The numbers of `account`, read in a call given its folders.
    fn numbers_of(
        host: &TestHost,
        account: &Account,
    ) -> Result<AccountNumbers, ReadError> {
        host.as_provider_in(PROVIDER, &account.found_in, || {
            OpenAi::numbers(PROVIDER, account, &[])
        })
    }

    /// The bill of `account` with the keys of `given`, read in a call
    /// given its folders.
    fn bill_of(
        host: &TestHost,
        account: &Account,
        given: &[GivenKey],
    ) -> Result<Bill, ReadError> {
        host.as_provider_in(PROVIDER, &account.found_in, || {
            OpenAi::read_bill(PROVIDER, account, given, &one_day())
        })
    }

    /// The need of `result`, which must be one.
    fn need_of<T: std::fmt::Debug>(result: Result<T, ReadError>) -> Need {
        match result {
            Err(ReadError::NeedsUser(need)) => need,
            other => panic!("a need: {other:?}"),
        }
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: the part
    /// reads the token of `auth.json` in the account's folder itself and
    /// asks ChatGPT with it, naming the account.
    #[test]
    fn numbers_of_an_account_read_with_the_token_of_its_folder() {
        let host = host().with_now_ms(TEN);
        host.answer("GET", USAGE_URL, 200, USAGE_ANSWER);
        let account = account_in(&host, ".codex-renewed");

        let numbers = numbers_of(&host, &account).expect("the numbers");

        assert_eq!(numbers.windows.len(), 1);
        assert_eq!(numbers.windows[0].percent_used, 42.0);
        assert_eq!(numbers.plan.as_deref(), Some("Pro 200"));
        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].header("chatgpt-account-id"), Some(ACCOUNT));
        assert_eq!(
            requests[0].header("authorization"),
            Some(format!("Bearer {RENEWED_TOKEN}").as_str())
        );
    }

    /// An account in two folders reads with the token of the first.
    #[test]
    fn an_account_in_two_folders_reads_with_the_first() {
        let host = host().with_now_ms(TEN);
        host.answer("GET", USAGE_URL, 200, USAGE_ANSWER);
        let accounts = accounts_in(&host, &[".codex-plus", ".codex-renewed"]);

        numbers_of(&host, &accounts[0]).expect("the numbers");

        assert_eq!(
            host.requests()[0].header("authorization"),
            Some(format!("Bearer {PLUS_TOKEN}").as_str())
        );
    }

    /// Decision 9: ChatGPT refused the token and Codex knows nothing
    /// there, so the read needs the user to run `codex login` in that
    /// folder.
    #[test]
    fn a_refused_token_gives_the_login_need_of_its_folder() {
        let host = host().with_now_ms(TEN);
        host.answer("GET", USAGE_URL, 401, b"{}");
        let account = account_in(&host, ".codex-renewed");
        let folder = path(&host, ".codex-renewed");

        let need = need_of(numbers_of(&host, &account));

        assert_eq!(need.id, "login-expired");
        let NeedAction::Run(command) = need.action else {
            panic!("a command: {need:?}");
        };
        assert_eq!(command.program, "codex");
        assert_eq!(command.args, ["login"]);
        assert_eq!(command.env, [("CODEX_HOME".to_string(), folder)]);
    }

    /// Issue 29: the server failed, so the part reads what Codex knows in
    /// the account's folder: `codex app-server` there, else its session
    /// files.
    #[test]
    fn a_failed_server_falls_back_on_what_codex_knows_in_the_folder() {
        let host = host().with_now_ms(TEN);
        host.answer("GET", USAGE_URL, 500, b"{}");
        let account = account_in(&host, ".codex-limits");

        let numbers = numbers_of(&host, &account).expect("the numbers");

        assert_eq!(numbers.plan.as_deref(), Some("Plus"));
        assert!(numbers.recorded_at_ms.is_some());
    }

    /// Issue 29: the login is gone from the account's folders since it
    /// was found, and Codex wrote nothing there: the part reads nothing,
    /// sends nothing and starts nothing, which is no failure.
    #[test]
    fn an_account_with_no_login_now_reads_nothing() {
        let host = host();
        host.answer_talk("codex", &APP_SERVER_ARGS, &[]);
        let account = known_account(&host, &[".codex-empty"]);

        let numbers = numbers_of(&host, &account);

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
        assert!(host.started().is_empty());
    }

    /// A folder that holds another account's login now is not read for
    /// this account.
    #[test]
    fn a_folder_with_another_login_now_is_not_read() {
        let host = host();
        let account = known_account(&host, &[".codex-team"]);

        let numbers = numbers_of(&host, &account);

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
        assert!(host.started().is_empty());
    }

    #[test]
    fn an_api_key_has_no_numbers_and_sends_nothing() {
        let host = host();
        let account = account_in(&host, ".codex-work");

        let numbers = numbers_of(&host, &account);

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
    }

    /// The bill reads only with an admin key: with no admin key given the
    /// part asks for one under the slot `admin` and sends nothing.
    #[test]
    fn a_bill_with_no_admin_key_asks_for_one() {
        let host = host();
        let account = account_in(&host, ".codex-work");

        let need = need_of(bill_of(&host, &account, &[]));

        assert_eq!(need.id, "bill-key");
        assert_eq!(need.title, "The real bill needs an admin key");
        let NeedAction::AskKey(ask) = need.action else {
            panic!("a key to ask for: {need:?}");
        };
        assert_eq!(ask.slot.as_deref(), Some("admin"));
        assert_eq!(ask.label, "Admin key");
        assert_eq!(ask.prefix.as_deref(), Some("sk-admin-"));
        assert_eq!(
            ask.page.as_deref(),
            Some(
                "https://platform.openai.com/settings/organization/admin-keys"
            )
        );
        assert!(host.requests().is_empty());
    }

    /// Issue 29: an account with no login in its folders now still reads
    /// its bill with the admin key the user gave under `admin`.
    #[test]
    fn an_admin_key_given_reads_the_bill() {
        let host = host();
        host.answer(
            "GET",
            COSTS_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let account = known_account(&host, &[".codex-empty"]);
        let given = [GivenKey::new("admin", host.given_key(GIVEN_ADMIN))];

        let bill = bill_of(&host, &account, &given);

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some(format!("Bearer {GIVEN_ADMIN}").as_str())
        );
    }

    /// An organization whose own key is an admin key reads its bill with
    /// it, read in its folder.
    #[test]
    fn an_organizations_own_admin_key_reads_its_bill() {
        let host = host();
        host.answer(
            "GET",
            COSTS_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let account = account_in(&host, ".codex-admin");

        let bill = bill_of(&host, &account, &[]);

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some(format!("Bearer {MADE_UP_ADMIN}").as_str())
        );
    }

    /// The admin key given under `admin` reads the bill before the
    /// account's own admin key.
    #[test]
    fn a_key_given_under_admin_reads_the_bill_before_the_accounts_own() {
        let host = host();
        host.answer(
            "GET",
            COSTS_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let account = account_in(&host, ".codex-admin");
        let given = [GivenKey::new("admin", host.given_key(GIVEN_ADMIN))];

        let bill = bill_of(&host, &account, &given);

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some(format!("Bearer {GIVEN_ADMIN}").as_str())
        );
    }

    /// Issue 26: an admin key an agent keeps is not OTTY's to replace: a
    /// refused one is asked for under `admin`, as a refused given one is
    /// asked for again, and the next bill reads with it.
    #[test]
    fn a_refused_admin_key_is_asked_for_under_admin() {
        let host = host();
        host.answer("GET", COSTS_URL, 401, b"{}");
        let account = account_in(&host, ".codex-admin");
        let given = [GivenKey::new("admin", host.given_key(GIVEN_ADMIN))];

        let own = need_of(bill_of(&host, &account, &[]));
        let of_slot = need_of(bill_of(&host, &account, &given));

        for need in [own, of_slot] {
            let NeedAction::AskKey(ask) = need.action else {
                panic!("a key to ask for: {need:?}");
            };
            assert_eq!(ask.slot.as_deref(), Some(ADMIN_SLOT));
        }
    }

    /// Spec Decision 7: a key handle goes only into a request to a host
    /// the part declares.
    #[test]
    fn the_part_may_call_no_other_host() {
        let host = host();
        let key = host.key(PLUS_TOKEN);

        let sent = host.as_provider(PROVIDER, || {
            let request = Request::get("https://api.deepseek.com/user/balance")
                .header_with_key("authorization", "Bearer ", &key);
            http::request(&request)
        });

        assert_eq!(sent, Err(HostError::NotAllowed("api.deepseek.com".into())));
        assert!(host.requests().is_empty());
    }

    /// Spec Decision 7, "Applied by context": in a call given other
    /// folders, the files and the key places of a Codex profile are
    /// refused to the provider part.
    #[test]
    fn the_part_may_not_read_codex_files_outside_its_folders() {
        let host = host();
        let auth = path(&host, ".codex-plus/auth.json");
        let folders = [codex_folder(&host, ".codex-team")];

        let (read, opened) = host.as_provider_in(PROVIDER, &folders, || {
            let read = files::read(&auth, 4096);
            let opened = keys::open(&KeyPlace::JsonField(JsonField {
                file: auth.clone(),
                pointer: "/tokens/access_token".to_string(),
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
