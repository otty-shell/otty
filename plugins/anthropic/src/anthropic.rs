//! The provider part `anthropic`: whose account a Claude login or an
//! Anthropic key is, the limits of a Claude login, and the bill of an
//! Anthropic organization read with an admin key the part asks the user
//! for. OTTY gives the part a handle to the login token or the key, never
//! the key, and only of accounts of Anthropic: a key behind another
//! server, such as DeepSeek's, is not one.
//!
//! The limits are read any way the part can (issue 29): from Anthropic's
//! server with the login token; else, with no token at its place or when
//! the server fails, from the `/usage` panel of `claude` in a Claude Code
//! profile the login was found in; else nothing.

use otty_plugin_sdk::agents::{
    AccountFacts, AccountNumbers, Bill, DayRange, LoginFacts, LoginKind,
    Provider, ReadError,
};
use otty_plugin_sdk::{GivenKey, HostError, Key, given_key};

use crate::needs::{self, ADMIN_PREFIX, ADMIN_SLOT};
use crate::{Anthropic, cost_report, oauth_usage, usage_probe};

/// How the token of a Claude login starts; an API key (`sk-ant-api`) or an
/// admin key (`sk-ant-admin`) does not.
const LOGIN_TOKEN_PREFIX: &str = "sk-ant-oat";

/// The agent part whose profiles show the `/usage` panel.
const CLAUDE_CODE: &str = "claude-code";

/// What the key of an account is, as far as the part may tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    /// The token of a Claude login.
    LoginToken,
    /// Another key, such as an API key.
    OtherKey,
    /// No key at its place now, such as a Claude Code profile whose
    /// `.credentials.json` is `{}` while Claude Code still works.
    NotThere,
}

impl Provider for Anthropic {
    /// The account the agent part found: a Claude login names its account
    /// and email in Claude Code's global config; a key names nothing, and
    /// OTTY knows it by its fingerprint. Nothing is asked.
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

    /// The windows and the extra usage of a Claude login: from Anthropic's
    /// server with its token, else from the `/usage` panel in a Claude
    /// Code profile of `found`, else nothing. Anthropic tells nothing of
    /// the money of one API key, so a key has no numbers and nothing is
    /// sent.
    fn numbers(
        _provider: &str,
        found: &LoginFacts,
        _account: &AccountFacts,
        key: &Key,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        match key_kind(key)? {
            KeyKind::OtherKey => Ok(no_numbers()),
            KeyKind::NotThere => {
                usage_panel(found).unwrap_or_else(|| Ok(no_numbers()))
            },
            KeyKind::LoginToken => match oauth_usage::claude_usage(key) {
                Ok(usage) => Ok(AccountNumbers {
                    windows: usage.windows,
                    extra_usage: usage.extra_usage,
                    ..no_numbers()
                }),
                Err(error) => match usage_panel(found) {
                    Some(Ok(numbers)) => Ok(numbers),
                    _ => Err(error),
                },
            },
        }
    }

    /// The cost report of the organization over `days`, read with an
    /// admin key: the one the user gave under [`ADMIN_SLOT`], else the
    /// account's own key when it is one, else the part asks for one and
    /// sends nothing. A refused key is asked for again where it sits: the
    /// account's own key the user gave in OTTY in its place, any other
    /// under [`ADMIN_SLOT`], which the next read takes first.
    fn read_bill(
        _provider: &str,
        _found: &LoginFacts,
        _account: &AccountFacts,
        key: &Key,
        given: &[GivenKey],
        days: &DayRange,
    ) -> Result<Bill, ReadError> {
        if let Some(admin_key) = given_key(given, ADMIN_SLOT) {
            return cost_report::bill(admin_key, Some(ADMIN_SLOT), days);
        }

        let is_bill_key =
            key.has_prefix(ADMIN_PREFIX).map_err(ReadError::Host)?;
        if !is_bill_key {
            return Err(ReadError::NeedsUser(needs::bill_key()));
        }

        let is_given_in_otty =
            key.is_given_in_otty().map_err(ReadError::Host)?;
        let refused_slot = match is_given_in_otty {
            true => None,
            false => Some(ADMIN_SLOT),
        };
        cost_report::bill(key, refused_slot, days)
    }
}

/// What `key` is: a login token, another key, or no key at its place.
fn key_kind(key: &Key) -> Result<KeyKind, ReadError> {
    match key.has_prefix(LOGIN_TOKEN_PREFIX) {
        Ok(true) => Ok(KeyKind::LoginToken),
        Ok(false) => Ok(KeyKind::OtherKey),
        Err(HostError::NotFound(_)) => Ok(KeyKind::NotThere),
        Err(error) => Err(ReadError::Host(error)),
    }
}

/// The windows of the `/usage` panel of `claude` in the first Claude Code
/// profile of `found`, for a subscription login; `None` when the login is
/// none or was found in no Claude Code profile.
fn usage_panel(
    found: &LoginFacts,
) -> Option<Result<AccountNumbers, ReadError>> {
    if found.kind != LoginKind::Subscription {
        return None;
    }
    let profile = found
        .found_in
        .iter()
        .find(|profile| profile.agent == CLAUDE_CODE)?;

    let windows = usage_probe::read_windows(profile);
    Some(windows.map(|windows| AccountNumbers {
        windows,
        ..no_numbers()
    }))
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
    use otty_plugin_sdk::agents::Identity;
    use otty_plugin_sdk::http::{self, Request};
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{
        GivenKey, JsonField, KeyPlace, NeedAction, files, keys,
    };

    use super::*;
    use crate::testing::{found_profile, host, path};

    const PROVIDER: &str = "anthropic";
    /// A made-up login token, never a real one.
    const TOKEN: &str = "sk-ant-oat01-made-up-login-token";
    const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
    /// The address of the cost report of 2026-11-16, the one day of
    /// [`one_day`].
    const REPORT_URL: &str = "https://api.anthropic.com/v1/organizations/cost_report?starting_at=2026-11-16T00:00:00Z&ending_at=2026-11-17T00:00:00Z&bucket_width=1d&limit=31";
    /// A made-up admin key, never a real one.
    const MADE_UP_ADMIN: &str = "sk-ant-admin01-made-up-for-tests";

    fn one_day() -> DayRange {
        DayRange {
            first: "2026-11-16".to_string(),
            last: "2026-11-16".to_string(),
        }
    }

    fn organization() -> AccountFacts {
        AccountFacts {
            identity: Identity {
                organization_id: Some("made-up-organization".to_string()),
                ..Identity::default()
            },
            plan: None,
        }
    }

    fn claude_login() -> LoginFacts {
        LoginFacts {
            kind: LoginKind::Subscription,
            identity: Identity {
                account_id: Some("made-up-account".to_string()),
                organization_id: None,
                email: Some("dev@example.com".to_string()),
            },
            plan: Some("Max 20x".to_string()),
            found_in: Vec::new(),
        }
    }

    /// The Claude login found in the Claude Code profile `.claude-no-token`
    /// of `host`, whose `.credentials.json` is `{}`.
    fn login_in_profile(host: &TestHost) -> LoginFacts {
        LoginFacts {
            found_in: vec![found_profile(host, ".claude-no-token", false)],
            ..claude_login()
        }
    }

    /// A handle to the login token of `.claude-no-token`, which is not
    /// there.
    fn missing_token(host: &TestHost) -> Key {
        host.key_at(&KeyPlace::JsonField(JsonField {
            file: path(host, ".claude-no-token/.credentials.json"),
            pointer: "/claudeAiOauth/accessToken".to_string(),
        }))
    }

    /// The screens of `claude` in a hidden terminal: its prompt, then the
    /// `/usage` panel with a five-hour window 29 % used.
    fn usage_screens(host: &TestHost) {
        let prompt = "> Try \"how does <filepath> work?\"\n  ? for shortcuts\n";
        let panel = "> /usage\n Settings:  Status   Config   Usage\n\n Current session\n \u{2588}\u{2588}\u{2588}                29% used\n Resets 5pm (Europe/Berlin)\n\n Esc to exit\n";
        host.answer_terminal("claude", &PROBE_ARGS, &[prompt, panel]);
    }

    /// The arguments the probe starts `claude` with.
    const PROBE_ARGS: [&str; 5] = [
        "--allowed-tools",
        "",
        "--strict-mcp-config",
        "--settings",
        r#"{"remoteControlAtStartup":false,"disableAllHooks":true}"#,
    ];

    /// Issue 29, the owner's case: Claude Code keeps no token in the
    /// profile's `.credentials.json`, yet works; the part reads the limits
    /// from the `/usage` panel of `claude` in that profile, and nothing
    /// fails.
    #[test]
    fn a_login_with_no_token_reads_the_usage_panel_of_its_profile() {
        let host =
            host().with_terminal_folder("/home/dev/.cache/otty/agent-probe");
        usage_screens(&host);
        let key = missing_token(&host);
        let found = login_in_profile(&host);

        let numbers = host.as_provider(PROVIDER, || {
            let facts = Anthropic::account(PROVIDER, &key, &found)?;
            Anthropic::numbers(PROVIDER, &found, &facts, &key, &[])
        });

        let numbers = numbers.expect("the numbers");
        assert_eq!(numbers.windows.len(), 1);
        assert_eq!(numbers.windows[0].percent_used, 29.0);
        assert!(host.requests().is_empty(), "nothing is sent");
        assert_eq!(host.started().len(), 1);
    }

    /// Issue 29: no token and no Claude Code profile to ask is nothing,
    /// not a failure, and nothing is sent or started.
    #[test]
    fn a_login_with_no_token_and_no_profile_reads_nothing() {
        let host = host();
        let key = missing_token(&host);

        let numbers = host.as_provider(PROVIDER, || {
            let facts = Anthropic::account(PROVIDER, &key, &claude_login())?;
            Anthropic::numbers(PROVIDER, &claude_login(), &facts, &key, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
        assert!(host.started().is_empty());
    }

    /// Issue 29: the server failed, so the part asks Claude Code in the
    /// profile the login was found in.
    #[test]
    fn a_failed_server_falls_back_on_the_usage_panel() {
        let host =
            host().with_terminal_folder("/home/dev/.cache/otty/agent-probe");
        host.answer("GET", USAGE_URL, 500, b"{}");
        usage_screens(&host);
        let key = host.key(TOKEN);
        let found = login_in_profile(&host);

        let numbers = host.as_provider(PROVIDER, || {
            let facts = Anthropic::account(PROVIDER, &key, &found)?;
            Anthropic::numbers(PROVIDER, &found, &facts, &key, &[])
        });

        assert_eq!(numbers.map(|numbers| numbers.windows.len()), Ok(1));
        assert_eq!(host.requests().len(), 1);
    }

    /// Issue 29: with no login token at its place, an admin key the user
    /// gave under `admin` still reads the bill.
    #[test]
    fn an_admin_key_given_reads_the_bill_without_the_token() {
        let host = host();
        host.answer(
            "GET",
            REPORT_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let key = missing_token(&host);
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        let bill = host.as_provider(PROVIDER, || {
            Anthropic::read_bill(
                PROVIDER,
                &claude_login(),
                &organization(),
                &key,
                &given,
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests()[0].header("x-api-key"), Some(MADE_UP_ADMIN));
    }

    #[test]
    fn a_claude_login_reads_its_windows_with_the_plan_it_was_found_with() {
        let host = host();
        host.answer(
            "GET",
            USAGE_URL,
            200,
            br#"{"five_hour": {"utilization": 42.0, "resets_at": null},
                 "extra_usage": {"is_enabled": true, "used_credits": 830}}"#,
        );
        let key = host.key(TOKEN);

        let (facts, numbers) = host
            .as_provider(PROVIDER, || {
                let facts =
                    Anthropic::account(PROVIDER, &key, &claude_login())?;
                let numbers = Anthropic::numbers(
                    PROVIDER,
                    &claude_login(),
                    &facts,
                    &key,
                    &[],
                )?;
                Ok::<_, ReadError>((facts, numbers))
            })
            .expect("the numbers");

        assert_eq!(facts.plan.as_deref(), Some("Max 20x"));
        assert_eq!(facts.identity, claude_login().identity);
        assert_eq!(numbers.windows.len(), 1);
        assert_eq!(numbers.extra_usage.and_then(|extra| extra.used), Some(8.3));
        assert_eq!(host.requests().len(), 1);
    }

    #[test]
    fn an_api_key_has_no_numbers_and_sends_nothing() {
        let host = host();
        let key = host.key("sk-ant-api03-made-up-key");
        let found = LoginFacts {
            kind: LoginKind::ApiKey,
            identity: Identity::default(),
            plan: None,
            found_in: Vec::new(),
        };

        let numbers = host.as_provider(PROVIDER, || {
            let facts = Anthropic::account(PROVIDER, &key, &found)?;
            Anthropic::numbers(PROVIDER, &found, &facts, &key, &[])
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
    }

    /// The bill reads only with an admin key: an API key of the account
    /// is not one, and with no admin key given the part asks for one under
    /// the slot `admin` and sends nothing.
    #[test]
    fn a_bill_with_no_bill_key_asks_for_one() {
        let host = host();
        let key = host.key("sk-ant-api03-made-up-key");

        let bill = host.as_provider(PROVIDER, || {
            Anthropic::read_bill(
                PROVIDER,
                &claude_login(),
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
        let key = host.key("sk-ant-api03-made-up-key");
        let given = [GivenKey::new("admin", host.key(MADE_UP_ADMIN))];

        let bill = host.as_provider(PROVIDER, || {
            Anthropic::read_bill(
                PROVIDER,
                &claude_login(),
                &organization(),
                &key,
                &given,
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests()[0].header("x-api-key"), Some(MADE_UP_ADMIN));
    }

    /// An account whose own key is an admin key reads its bill with it.
    #[test]
    fn an_account_key_that_reads_bills_reads_its_own() {
        let host = host();
        host.answer(
            "GET",
            REPORT_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let key = host.key(MADE_UP_ADMIN);

        let bill = host.as_provider(PROVIDER, || {
            Anthropic::read_bill(
                PROVIDER,
                &claude_login(),
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
    /// keys of `given` that Anthropic refuses.
    fn refused_bill_need(
        host: &otty_plugin_sdk::test_host::TestHost,
        key: &Key,
        given: &[GivenKey],
    ) -> otty_plugin_sdk::Need {
        host.answer("GET", REPORT_URL, 401, b"{}");

        let bill = host.as_provider(PROVIDER, || {
            Anthropic::read_bill(
                PROVIDER,
                &claude_login(),
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

    /// Issue 26: the account's own admin key, which the user added by
    /// hand, is refused: the new key takes its place, with no slot.
    #[test]
    fn a_refused_own_admin_key_given_in_otty_is_asked_for_in_its_place() {
        let host = host();
        let key = host.given_key(MADE_UP_ADMIN);

        let need = refused_bill_need(&host, &key, &[]);

        assert_eq!(need.id, "new-bill-key");
        assert_eq!(asked_slot(need), None);
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
    /// it, never in place of the account's own key.
    #[test]
    fn a_refused_admin_key_of_the_slot_is_asked_for_again_under_it() {
        let host = host();
        let key = host.given_key("sk-ant-api03-made-up-key");
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
            REPORT_URL,
            200,
            br#"{"data": [], "has_more": false}"#,
        );
        let key = host.key("sk-ant-admin01-made-up-refused");
        let given = [GivenKey::new("admin", host.given_key(MADE_UP_ADMIN))];

        let bill = host.as_provider(PROVIDER, || {
            Anthropic::read_bill(
                PROVIDER,
                &claude_login(),
                &organization(),
                &key,
                &given,
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests()[0].header("x-api-key"), Some(MADE_UP_ADMIN));
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
    /// runs, the files and the key places of the plugin's `claude-code`
    /// part are refused.
    #[test]
    fn the_part_may_not_read_the_files_of_the_claude_code_part() {
        let host = host();
        let credentials = path(&host, ".claude/.credentials.json");

        let (read, opened) = host.as_provider(PROVIDER, || {
            let read = files::read(&credentials, 4096);
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
