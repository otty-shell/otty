//! The provider part `openai`: whose account a ChatGPT login or an OpenAI
//! key is, the limits and credits of a ChatGPT login, and the bill of an
//! OpenAI organization read with an admin key. OTTY gives the part a
//! handle to the login token or the key, never the key, and only of
//! accounts of OpenAI: a login behind a gateway is not one.

use otty_plugin_sdk::{
    AccountFacts, AccountNumbers, Bill, DayRange, Key, LoginFacts, Provider,
    ReadError,
};

use crate::{OpenAi, chatgpt_usage, organization_costs};

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

    /// The limits and credits of a ChatGPT login, told by its account id.
    /// OpenAI tells nothing of the money of one API key, so a key has no
    /// numbers and nothing is sent.
    fn numbers(
        _provider: &str,
        account: &AccountFacts,
        key: &Key,
    ) -> Result<AccountNumbers, ReadError> {
        let Some(account_id) = account.identity.account_id.as_deref() else {
            return Ok(no_numbers());
        };

        let limits = chatgpt_usage::chatgpt_limits(key, account_id)?;

        Ok(AccountNumbers {
            windows: limits.windows,
            credits: limits.credits,
            plan: limits.plan,
            ..no_numbers()
        })
    }

    /// The costs of the organization of the admin key over `days`.
    fn read_bill(
        _provider: &str,
        _account: &AccountFacts,
        admin_key: &Key,
        days: &DayRange,
    ) -> Result<Bill, ReadError> {
        organization_costs::bill(admin_key, days)
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
    use otty_plugin_sdk::http::{self, Request};
    use otty_plugin_sdk::{
        HostError, Identity, JsonField, KeyPlace, LoginKind, files, keys,
    };

    use super::*;
    use crate::testing::{ACCOUNT, TEN, host, path};

    const PROVIDER: &str = "openai";
    /// A made-up token that expires in 2100.
    const TOKEN: &str =
        "e30.eyJleHAiOjQxMDI0NDQ4MDAsInN1YiI6Im1hZGUtdXAtYWNjZXNzIn0.c2ln";
    const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

    fn chatgpt_login() -> LoginFacts {
        LoginFacts {
            kind: LoginKind::Subscription,
            identity: Identity {
                account_id: Some(ACCOUNT.to_string()),
                organization_id: None,
                email: Some("dev@example.com".to_string()),
            },
            plan: Some("Plus".to_string()),
        }
    }

    fn api_key() -> LoginFacts {
        LoginFacts {
            kind: LoginKind::ApiKey,
            identity: Identity::default(),
            plan: None,
        }
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
            OpenAi::numbers(PROVIDER, &facts, &key)
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
            OpenAi::numbers(PROVIDER, &facts, &key)
        });

        assert_eq!(numbers, Ok(no_numbers()));
        assert!(host.requests().is_empty());
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
