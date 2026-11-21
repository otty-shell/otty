//! The provider part `anthropic`: whose account a Claude login or an
//! Anthropic key is, the limits of a Claude login read from Anthropic's
//! server, and the bill of an Anthropic organization read with an admin
//! key the part asks the user for. OTTY gives the part a handle to the login token or the key, never
//! the key, and only of accounts of Anthropic: a key behind another
//! server, such as DeepSeek's, is not one.

use otty_plugin_sdk::agents::{
    AccountFacts, AccountNumbers, Bill, DayRange, LoginFacts, Provider,
    ReadError,
};
use otty_plugin_sdk::{GivenKey, Key, given_key};

use crate::needs::{self, ADMIN_PREFIX, ADMIN_SLOT};
use crate::{Anthropic, cost_report, oauth_usage};

/// How the token of a Claude login starts; an API key (`sk-ant-api`) or an
/// admin key (`sk-ant-admin`) does not.
const LOGIN_TOKEN_PREFIX: &str = "sk-ant-oat";

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

    /// The windows and the extra usage of a Claude login. Anthropic tells
    /// nothing of the money of one API key, so a key has no numbers and
    /// nothing is sent.
    fn numbers(
        _provider: &str,
        _account: &AccountFacts,
        key: &Key,
        _given: &[GivenKey],
    ) -> Result<AccountNumbers, ReadError> {
        let is_login = key
            .has_prefix(LOGIN_TOKEN_PREFIX)
            .map_err(ReadError::Host)?;
        if !is_login {
            return Ok(no_numbers());
        }

        let usage = oauth_usage::claude_usage(key)?;

        Ok(AccountNumbers {
            windows: usage.windows,
            extra_usage: usage.extra_usage,
            ..no_numbers()
        })
    }

    /// The cost report of the organization over `days`, read with an
    /// admin key: the account's own key when it is one, else the one the
    /// user gave under [`ADMIN_SLOT`], else the part asks for one and
    /// sends nothing.
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
            false => given_key(given, ADMIN_SLOT)
                .ok_or_else(|| ReadError::NeedsUser(needs::bill_key()))?,
        };

        cost_report::bill(bill_key, days)
    }
}

/// Numbers with nothing in them.
fn no_numbers() -> AccountNumbers {
    AccountNumbers {
        windows: Vec::new(),
        credits: None,
        extra_usage: None,
        left: None,
        key_limit: None,
        spent_this_month: None,
        plan: None,
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::{Identity, LoginKind};
    use otty_plugin_sdk::http::{self, Request};
    use otty_plugin_sdk::{
        GivenKey, HostError, JsonField, KeyPlace, NeedAction, files, keys,
    };

    use super::*;
    use crate::testing::{host, path};

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
        }
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
                let numbers = Anthropic::numbers(PROVIDER, &facts, &key, &[])?;
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
        };

        let numbers = host.as_provider(PROVIDER, || {
            let facts = Anthropic::account(PROVIDER, &key, &found)?;
            Anthropic::numbers(PROVIDER, &facts, &key, &[])
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
        assert_eq!(ask.slot, "admin");
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
                &organization(),
                &key,
                &[],
                &one_day(),
            )
        });

        assert!(bill.is_ok(), "{bill:?}");
        assert_eq!(host.requests().len(), 1);
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
