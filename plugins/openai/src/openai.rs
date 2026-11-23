//! The provider part `openai`: whose account a ChatGPT login or an OpenAI
//! key is, the limits and credits of a ChatGPT login, and the bill of an
//! OpenAI organization read with an admin key the part asks the user for.
//! OTTY gives the part a handle to the login token or the key, never the
//! key, and only of accounts of OpenAI: a login behind a gateway is not
//! one.
//!
//! The limits are read any way the part can (issue 29): from
//! `chatgpt.com` with the login token; else, with no token at its place or
//! when the server fails, from what Codex knows in a Codex profile the
//! login was found in (`codex_limits.rs`); else nothing.

use otty_plugin_sdk::agents::{
    AccountFacts, AccountNumbers, Bill, DayRange, LoginFacts, LoginKind,
    Provider, ReadError,
};
use otty_plugin_sdk::{GivenKey, HostError, Key, given_key};

use crate::codex_limits::read_codex_limits;
use crate::limits::Limits;
use crate::needs::{self, ADMIN_PREFIX, ADMIN_SLOT};
use crate::{OpenAi, chatgpt_usage, organization_costs};

/// The agent part whose profiles know the limits of a ChatGPT login.
const CODEX: &str = "codex";

impl Provider for OpenAi {
    /// The account the agent part found: a ChatGPT login names its account
    /// id, email and plan in its tokens; a key names nothing, and OTTY
    /// knows it by its fingerprint. Nothing is asked.
    fn account(
        _provider: &str,
        _key: &Key,
        found: &LoginFacts,
    ) -> Result<AccountFacts, ReadError> {
        Ok(AccountFacts {
            identity: found.identity.clone(),
            plan: found.plan.clone(),
        })
    }

    /// The limits and credits of a ChatGPT login: from `chatgpt.com`,
    /// told by its account id, with its token; else what Codex knows in a
    /// Codex profile of `found`; else nothing. OpenAI tells nothing of the
    /// money of one API key, so a key has no numbers and nothing is sent.
    fn numbers(
        _provider: &str,
        found: &LoginFacts,
        account: &AccountFacts,
        key: &Key,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        if found.kind != LoginKind::Subscription {
            return Ok(no_numbers());
        }

        let account_id = account.identity.account_id.as_deref();
        let server = match account_id {
            Some(account_id) if is_there(key)? => {
                Some(chatgpt_usage::chatgpt_limits(key, account_id))
            },
            _ => None,
        };

        let limits = match server {
            Some(Ok(limits)) => Ok(limits),
            Some(Err(error)) => match codex_limits(found, key) {
                Some(Ok(limits)) => Ok(limits),
                _ => Err(error),
            },
            None => match codex_limits(found, key) {
                Some(read) => read,
                None => return Ok(no_numbers()),
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
        _found: &LoginFacts,
        _account: &AccountFacts,
        key: &Key,
        given: &[GivenKey],
        days: &DayRange,
    ) -> Result<Bill, ReadError> {
        if let Some(admin_key) = given_key(given, ADMIN_SLOT) {
            return organization_costs::bill(admin_key, days);
        }

        let is_bill_key = is_bill_key(key)?;
        if !is_bill_key {
            return Err(ReadError::NeedsUser(needs::bill_key()));
        }

        organization_costs::bill(key, days)
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

/// Whether the key of `key` is at its place now. A key that is not is a
/// normal case, such as a login another agent keeps elsewhere.
fn is_there(key: &Key) -> Result<bool, ReadError> {
    match key.fingerprint() {
        Ok(_) => Ok(true),
        Err(HostError::NotFound(_)) => Ok(false),
        Err(error) => Err(ReadError::Host(error)),
    }
}

/// The limits Codex knows of the login of `key` in the first Codex
/// profile of `found`; `None` when it was found in none.
fn codex_limits(
    found: &LoginFacts,
    key: &Key,
) -> Option<Result<Limits, ReadError>> {
    let profile = found
        .found_in
        .iter()
        .find(|profile| profile.agent == CODEX)?;

    Some(read_codex_limits(profile, key))
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
    use otty_plugin_sdk::agents::{Identity, LoginKind};
    use otty_plugin_sdk::http::{self, Request};
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{
        GivenKey, HostError, JsonField, KeyPlace, NeedAction, files, keys,
    };

    use super::*;
    use crate::testing::{ACCOUNT, PROVIDER, TEN, found_profile, host, path};

    /// A made-up token that expires in 2100.
    const TOKEN: &str =
        "e30.eyJleHAiOjQxMDI0NDQ4MDAsInN1YiI6Im1hZGUtdXAtYWNjZXNzIn0.c2ln";
    const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
    /// The address of the costs of 2026-11-16, the one day of [`one_day`].
    const COSTS_URL: &str = "https://api.openai.com/v1/organization/costs?start_time=1794787200&end_time=1794873600&bucket_width=1d&limit=31";
    /// A made-up admin key, never a real one.
    const MADE_UP_ADMIN: &str = "sk-admin-made-up-for-tests";

    fn one_day() -> DayRange {
        DayRange {
            first: "2026-11-16".to_string(),
            last: "2026-11-16".to_string(),
        }
    }

    fn organization() -> AccountFacts {
        AccountFacts {
            identity: Identity {
                organization_id: Some("org-made-up".to_string()),
                ..Identity::default()
            },
            plan: None,
        }
    }

    fn chatgpt_login() -> LoginFacts {
        LoginFacts {
            kind: LoginKind::Subscription,
            identity: Identity {
                account_id: Some(ACCOUNT.to_string()),
                organization_id: None,
                email: Some("dev@example.com".to_string()),
            },
            plan: Some("Plus".to_string()),
            found_in: Vec::new(),
        }
    }

    fn api_key() -> LoginFacts {
        LoginFacts {
            kind: LoginKind::ApiKey,
            identity: Identity::default(),
            plan: None,
            found_in: Vec::new(),
        }
    }

    /// A handle to the access token of the login of the config folder
    /// `name`, read at each use.
    fn token_at(host: &TestHost, name: &str) -> Key {
        host.key_at(&KeyPlace::JsonField(JsonField {
            file: path(host, &format!("{name}/auth.json")),
            pointer: "/tokens/access_token".to_string(),
        }))
    }

    /// The ChatGPT login found in the Codex profile `name`.
    fn login_in(host: &TestHost, name: &str) -> LoginFacts {
        LoginFacts {
            found_in: vec![found_profile(host, name, false)],
            ..chatgpt_login()
        }
    }

    /// Issue 29: no token at its place, so the part reads the limits Codex
    /// wrote in the session files of the profile the login was found in;
    /// nothing is sent and nothing fails.
    #[test]
    fn a_login_with_no_token_reads_the_limits_codex_wrote() {
        let host = host();
        let key = token_at(&host, ".codex-empty");
        let found = login_in(&host, ".codex-limits");

        let numbers = host.as_provider(PROVIDER, || {
            let facts = OpenAi::account(PROVIDER, &key, &found)?;
            OpenAi::numbers(PROVIDER, &found, &facts, &key, &[])
        });

        let numbers = numbers.expect("the numbers");
        assert_eq!(numbers.plan.as_deref(), Some("Plus"));
        assert!(numbers.recorded_at_ms.is_some());
        assert!(host.requests().is_empty());
    }

    /// Issue 29: no token and no Codex profile is nothing, not a failure.
    #[test]
    fn a_login_with_no_token_and_no_profile_reads_nothing() {
        let host = host();
        let key = token_at(&host, ".codex-empty");

        let numbers = host.as_provider(PROVIDER, || {
            let facts = OpenAi::account(PROVIDER, &key, &chatgpt_login())?;
            OpenAi::numbers(PROVIDER, &chatgpt_login(), &facts, &key, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
        assert!(host.started().is_empty());
    }

    /// Issue 29: the server failed, so the part reads what Codex knows in
    /// the profile the login was found in.
    #[test]
    fn a_failed_server_falls_back_on_what_codex_knows() {
        let host = host().with_now_ms(TEN);
        host.answer("GET", USAGE_URL, 500, b"{}");
        let key = host.key(TOKEN);
        let found = login_in(&host, ".codex-limits");

        let numbers = host.as_provider(PROVIDER, || {
            let facts = OpenAi::account(PROVIDER, &key, &found)?;
            OpenAi::numbers(PROVIDER, &found, &facts, &key, &[])
        });

        assert_eq!(
            numbers.map(|numbers| numbers.plan),
            Ok(Some("Plus".to_string()))
        );
    }

    /// Issue 29: with no login token at its place, an admin key the user
    /// gave under `admin` still reads the bill.
    #[test]
    fn an_admin_key_given_reads_the_bill_without_the_token() {
        let host = host();
        host.answer(
            "GET",
            COSTS_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let key = token_at(&host, ".codex-empty");
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        let bill = host.as_provider(PROVIDER, || {
            OpenAi::read_bill(
                PROVIDER,
                &chatgpt_login(),
                &organization(),
                &key,
                &given,
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some(format!("Bearer {MADE_UP_ADMIN}").as_str())
        );
    }

    #[test]
    fn the_account_of_a_login_is_what_its_agent_part_found() {
        let host = host();
        let key = host.key(TOKEN);

        let facts = host.as_provider(PROVIDER, || {
            OpenAi::account(PROVIDER, &key, &chatgpt_login())
        });

        assert_eq!(
            facts,
            Ok(AccountFacts {
                identity: chatgpt_login().identity,
                plan: Some("Plus".to_string()),
            })
        );
        assert!(host.requests().is_empty());
    }

    #[test]
    fn a_chatgpt_login_reads_its_limits_with_its_account() {
        let host = host().with_now_ms(TEN);
        host.answer(
            "GET",
            USAGE_URL,
            200,
            br#"{"plan_type": "pro", "rate_limit": {"primary_window": {"used_percent": 42, "limit_window_seconds": 18000}}}"#,
        );
        let key = host.key(TOKEN);

        let numbers = host.as_provider(PROVIDER, || {
            let facts = OpenAi::account(PROVIDER, &key, &chatgpt_login())?;
            OpenAi::numbers(PROVIDER, &chatgpt_login(), &facts, &key, &[])
        });

        let numbers = numbers.expect("the numbers");
        assert_eq!(numbers.windows.len(), 1);
        assert_eq!(numbers.windows[0].percent_used, 42.0);
        assert_eq!(numbers.plan.as_deref(), Some("Pro 200"));
        assert_eq!(numbers.left, None);
        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].header("chatgpt-account-id"), Some(ACCOUNT));
    }

    #[test]
    fn an_api_key_has_no_numbers_and_sends_nothing() {
        let host = host();
        let key = host.key("sk-made-up-openai-key-0000");

        let numbers = host.as_provider(PROVIDER, || {
            let facts = OpenAi::account(PROVIDER, &key, &api_key())?;
            OpenAi::numbers(PROVIDER, &api_key(), &facts, &key, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
    }

    /// With no login token at its place and no admin key given, the bill
    /// asks for an admin key, as for any key that does not read bills,
    /// and sends nothing: a key that is not there is not a failure.
    #[test]
    fn a_bill_with_no_token_and_no_admin_key_asks_for_one() {
        let host = host();
        let key = token_at(&host, ".codex-empty");

        let bill = host.as_provider(PROVIDER, || {
            OpenAi::read_bill(
                PROVIDER,
                &login_in(&host, ".codex-empty"),
                &organization(),
                &key,
                &[],
                &one_day(),
            )
        });

        let Err(ReadError::NeedsUser(need)) = bill else {
            panic!("a need: {bill:?}");
        };
        assert_eq!(need.id, "bill-key");
        assert!(host.requests().is_empty());
    }

    /// The bill reads only with an admin key: an API key of the account
    /// is not one, and with no admin key given the part asks for one under
    /// the slot `admin` and sends nothing.
    #[test]
    fn a_bill_with_no_bill_key_asks_for_one() {
        let host = host();
        let key = host.key("sk-proj-made-up-openai-key");

        let bill = host.as_provider(PROVIDER, || {
            OpenAi::read_bill(
                PROVIDER,
                &chatgpt_login(),
                &organization(),
                &key,
                &[],
                &one_day(),
            )
        });

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
        assert_eq!(ask.prefix.as_deref(), Some("sk-admin-"));
        assert_eq!(
            ask.page.as_deref(),
            Some(
                "https://platform.openai.com/settings/organization/admin-keys"
            )
        );
        assert!(host.requests().is_empty());
    }

    /// The admin key the user gave under `admin` reads the bill.
    #[test]
    fn a_key_given_under_admin_reads_the_bill() {
        let host = host();
        host.answer(
            "GET",
            COSTS_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let key = host.key("sk-proj-made-up-openai-key");
        let given = [GivenKey::new("admin", host.key(MADE_UP_ADMIN))];

        let bill = host.as_provider(PROVIDER, || {
            OpenAi::read_bill(
                PROVIDER,
                &chatgpt_login(),
                &organization(),
                &key,
                &given,
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some(format!("Bearer {MADE_UP_ADMIN}").as_str())
        );
    }

    /// An account whose own key is an admin key reads its bill with it.
    #[test]
    fn an_account_key_that_reads_bills_reads_its_own() {
        let host = host();
        host.answer(
            "GET",
            COSTS_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let key = host.key(MADE_UP_ADMIN);

        let bill = host.as_provider(PROVIDER, || {
            OpenAi::read_bill(
                PROVIDER,
                &chatgpt_login(),
                &organization(),
                &key,
                &[],
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests().len(), 1);
    }

    /// The need of a bill read with the account's own key `key` and the
    /// keys of `given` that OpenAI refuses.
    fn refused_bill_need(
        host: &otty_plugin_sdk::test_host::TestHost,
        key: &Key,
        given: &[GivenKey],
    ) -> otty_plugin_sdk::Need {
        host.answer("GET", COSTS_URL, 401, b"{}");

        let bill = host.as_provider(PROVIDER, || {
            OpenAi::read_bill(
                PROVIDER,
                &chatgpt_login(),
                &organization(),
                key,
                given,
                &one_day(),
            )
        });

        match bill {
            Err(ReadError::NeedsUser(need)) => need,
            other => panic!("a need: {other:?}"),
        }
    }

    /// The slot the need asks for its key under.
    fn asked_slot(need: otty_plugin_sdk::Need) -> Option<String> {
        match need.action {
            NeedAction::AskKey(ask) => ask.slot,
            other => panic!("a key to ask for: {other:?}"),
        }
    }

    /// Issue 26: an admin key an agent keeps is not OTTY's to replace: the
    /// new one is asked for under `admin`, and the next bill reads with it.
    #[test]
    fn a_refused_own_admin_key_of_an_agent_is_asked_for_under_admin() {
        let host = host();
        let key = host.key(MADE_UP_ADMIN);

        let need = refused_bill_need(&host, &key, &[]);

        assert_eq!(asked_slot(need).as_deref(), Some(ADMIN_SLOT));
    }

    /// A refused admin key given under `admin` is asked for again under
    /// it.
    #[test]
    fn a_refused_admin_key_of_the_slot_is_asked_for_again_under_it() {
        let host = host();
        let key = host.given_key("sk-proj-made-up-openai-key");
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        let need = refused_bill_need(&host, &key, &given);

        assert_eq!(asked_slot(need).as_deref(), Some(ADMIN_SLOT));
    }

    /// The admin key given under `admin` reads the bill before the
    /// account's own admin key, which was refused before it was given.
    #[test]
    fn a_key_given_under_admin_reads_the_bill_before_the_accounts_own() {
        let host = host();
        host.answer(
            "GET",
            COSTS_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let key = host.key("sk-admin-made-up-refused");
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        let bill = host.as_provider(PROVIDER, || {
            OpenAi::read_bill(
                PROVIDER,
                &chatgpt_login(),
                &organization(),
                &key,
                &given,
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(
            host.requests()[0].header("authorization"),
            Some(format!("Bearer {MADE_UP_ADMIN}").as_str())
        );
    }

    /// Spec Decision 7: a key handle goes only into a request to a host
    /// the part declares.
    #[test]
    fn the_part_may_call_no_other_host() {
        let host = host();
        let key = host.key(TOKEN);

        let sent = host.as_provider(PROVIDER, || {
            let request = Request::get("https://api.deepseek.com/user/balance")
                .header_with_key("authorization", "Bearer ", &key);
            http::request(&request)
        });

        assert_eq!(sent, Err(HostError::NotAllowed("api.deepseek.com".into())));
        assert!(host.requests().is_empty());
    }

    /// Spec Decision 7, "Applied by context": while the provider part
    /// runs, the files and the key places of the plugin's `codex` part are
    /// refused.
    #[test]
    fn the_part_may_not_read_the_files_of_the_codex_part() {
        let host = host();
        let auth = path(&host, ".codex-plus/auth.json");

        let (read, opened) = host.as_provider(PROVIDER, || {
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
