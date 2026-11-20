//! The numbers OTTY reads of an account by itself, with the account's own
//! key, rather than through the limit sources of a profile (spec
//! 2026-11-19, Decisions 5 and 7): the money left on a DeepSeek, Moonshot
//! or Kilo account, the limit of an OpenRouter key, the bill of an
//! Anthropic or OpenAI organization the user added, and the limits of a
//! ChatGPT login only Kilo or OpenCode keeps.
//!
//! OTTY keeps no key: the key is read again right before the request,
//! from where it was found ([`KeyPlace`]), and dropped right after it.
//! Every request has OTTY's `User-Agent`, follows no redirect and gives up
//! after 30 seconds ([`UreqClient`]).

mod deepseek;
mod kilo;
mod moonshot;
mod openai_costs;
mod openrouter;

use std::time::SystemTime;

use serde_json::Value;

use crate::account::{Account, AccountIdentity, AccountKind, FoundIn};
use crate::account_key::AccountKey;
use crate::agent::{Agent, agent_of};
use crate::balance::Balance;
use crate::bill::{Bill, BillSource};
use crate::built_in::{
    ANTHROPIC, DEEPSEEK, KILO, MOONSHOT, OPENAI, OPENROUTER,
};
use crate::claude_code::CostReport;
use crate::codex::chatgpt_limits;
use crate::error::{SourceError, Unavailable};
use crate::https::{HttpsGet, HttpsRequest, UreqClient, json_answer};
use crate::keyring_entry::{KeyringEntry, key_in_keyring};
use crate::limits::Limits;
use crate::plugin_provider::PluginProvider;
use crate::profile::Profile;
use crate::provider::ProviderId;
use crate::registry::{Reader, Registry};
use crate::source_id::SourceId;
use crate::system_keyring::{KeyringRead, SystemKeyring};
use crate::walk::Answer;

/// Where the key or the login token of an account is read again for a
/// request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPlace {
    /// The file or the variable the user gave for an account they added.
    Added,
    /// The files of this profile, where the account was found; its agent
    /// reads them again.
    ProfileFiles(Profile),
    /// These entries of the system keyring, which the user allowed OTTY to
    /// read; the system may ask to unlock it.
    Keyring(Vec<KeyringEntry>),
}

/// What OTTY read of an account with its own key.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountNumbers {
    /// The money left, or the limit of the key.
    Balance(Balance),
    /// The bill of an organization.
    Bill(Bill),
    /// The limits of a subscription, told by the ChatGPT usage request.
    Limits(Answer<Limits>),
}

/// What OTTY reads of an account with its own key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnSource {
    /// The money left or the limit of a key.
    Balance,
    /// The bill of an organization, read with an admin key.
    Bill,
    /// The limits of a ChatGPT login.
    ChatGptLimits,
}

/// What a read reaches: the network, the system keyring and the agents
/// that read their files again.
struct Reach<'a> {
    client: Box<dyn HttpsGet>,
    keyring: &'a dyn KeyringRead,
    agents: &'a [Box<dyn Agent>],
}

impl Reach<'_> {
    /// The numbers of `account` read with its key from `place`.
    fn read(
        self,
        account: &Account,
        place: &KeyPlace,
        now: SystemTime,
        env_var: impl Fn(&str) -> Option<String>,
    ) -> Result<AccountNumbers, SourceError> {
        let source = own_source(account)
            .ok_or(SourceError::NotAvailable(Unavailable::NoAccountSource))?;

        let key = self.key_of(account, place, env_var)?;
        let client = self.client;

        match source {
            OwnSource::Balance => {
                balance(client.as_ref(), provider_id(account), &key)
                    .map(AccountNumbers::Balance)
            },
            OwnSource::Bill => bill(client, provider_id(account), &key, now)
                .map(AccountNumbers::Bill),
            OwnSource::ChatGptLimits => {
                let AccountIdentity::AccountId(id) = account.identity() else {
                    return Err(SourceError::NotAvailable(
                        Unavailable::NoAccountSource,
                    ));
                };
                chatgpt_limits(client.as_ref(), key.key().secret(), id).map(
                    |limits| {
                        let answer =
                            Answer::first(limits, SourceId::ChatGptUsageApi);
                        AccountNumbers::Limits(answer)
                    },
                )
            },
        }
    }

    /// The key of `account`, read again from `place`.
    fn key_of(
        &self,
        account: &Account,
        place: &KeyPlace,
        env_var: impl Fn(&str) -> Option<String>,
    ) -> Result<AccountKey, SourceError> {
        let gone = SourceError::NotAvailable(Unavailable::KeyGone);

        match place {
            KeyPlace::Added => {
                let reference = account.added_key().ok_or(gone)?;
                reference.read(env_var).map(AccountKey::new)
            },
            KeyPlace::ProfileFiles(profile) => {
                agent_of(self.agents, profile.agent())
                    .and_then(|agent| agent.account_key(profile, account))
                    .ok_or(gone)
            },
            KeyPlace::Keyring(entries) => {
                key_in_keyring(self.keyring, entries, account)
                    .map_err(|failure| {
                        SourceError::NotAvailable(Unavailable::Keyring(failure))
                    })?
                    .map(AccountKey::new)
                    .ok_or(gone)
            },
        }
    }
}

/// The source OTTY reads `account` through with its own key, by itself;
/// `None` for an account whose numbers come only from the profiles that
/// use it, such as a Claude login.
///
/// The account the user added reads the bill of its organization
/// (Anthropic, OpenAI) or the money of its key; a key OTTY found reads the
/// money of a DeepSeek, OpenRouter, Moonshot or Kilo account; a ChatGPT
/// login reads its limits, which the app asks for only when no profile
/// that uses it reads them.
///
/// It is `None` too while no entry of `registry` serves the account's
/// provider (spec 2026-11-19-agent-plugins, "What the UI shows": no Left
/// or Limits rows with no provider part). A provider a plugin part reads
/// reads every account found in a profile, with the key of its login
/// ([`SourceId::Plugin`]).
///
/// ```
/// use otty_agents::{
///     Account, AdminKeyRef, ProviderId, ProviderRef, Registry, SourceId,
///     known_agents, own_source_of,
/// };
///
/// let registry = Registry::built_in(&known_agents());
/// let deepseek = Account::added(
///     ProviderRef::of_id("deepseek"),
///     AdminKeyRef::EnvVar("DEEPSEEK_API_KEY".into()),
/// );
/// let without = registry
///     .clone()
///     .without_provider(&ProviderId::new("deepseek"));
///
/// assert_eq!(
///     own_source_of(&deepseek, &registry),
///     Some(SourceId::DeepSeekBalance)
/// );
/// assert_eq!(own_source_of(&deepseek, &without), None);
/// ```
pub fn own_source_of(
    account: &Account,
    registry: &Registry,
) -> Option<SourceId> {
    let entry = registry.provider_of(account.provider())?;
    if let Reader::Plugin(_) = entry.reader() {
        let is_in_profile = account
            .found_in()
            .iter()
            .any(|place| matches!(place, FoundIn::ProfileFolder(_)));
        return is_in_profile.then_some(SourceId::Plugin);
    }

    let id = match (own_source(account)?, provider_id(account)) {
        (OwnSource::Bill, ANTHROPIC) => SourceId::AnthropicCostReport,
        (OwnSource::Bill, _) => SourceId::OpenAiCosts,
        (OwnSource::ChatGptLimits, _) => SourceId::ChatGptUsageApi,
        (OwnSource::Balance, DEEPSEEK) => SourceId::DeepSeekBalance,
        (OwnSource::Balance, OPENROUTER) => SourceId::OpenRouterKey,
        (OwnSource::Balance, MOONSHOT) => SourceId::MoonshotBalance,
        (OwnSource::Balance, _) => SourceId::KiloBalance,
    };

    Some(id)
}

/// The numbers of `account` up to `now`, read with its key from `place`
/// by the request of its provider: the one of `providers`, the provider
/// parts of plugins, that serves it, else the reader built into OTTY.
/// `agents` read the files of a profile again, and `env_var` gives the
/// value of an environment variable, for a key the user keeps in one. It
/// blocks; a key in the keyring may make the system ask the user to
/// unlock it.
///
/// # Errors
///
/// An account with no source of its own, or whose key is no longer where
/// it was found, is "not available" and nothing is sent; a refused key is
/// [`SourceError::KeyRefused`], and an expired ChatGPT login is "not
/// logged in" (it is never renewed).
pub fn read_account_numbers(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    account: &Account,
    place: &KeyPlace,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<AccountNumbers, SourceError> {
    if let Some(provider) = providers.iter().find(|part| part.serves(account)) {
        return plugin_numbers(provider, agents, account, place);
    }

    let reach = Reach {
        client: Box::new(UreqClient::new()),
        keyring: &SystemKeyring,
        agents,
    };

    reach.read(account, place, now, env_var)
}

/// The numbers of `account` read by the provider part `provider` with the
/// key of the login an agent part found it by in the profile of `place`.
/// An account the user added or found in the keyring has no such login.
fn plugin_numbers(
    provider: &PluginProvider,
    agents: &[Box<dyn Agent>],
    account: &Account,
    place: &KeyPlace,
) -> Result<AccountNumbers, SourceError> {
    let KeyPlace::ProfileFiles(profile) = place else {
        return Err(SourceError::NotAvailable(Unavailable::NoAccountSource));
    };

    let login = agent_of(agents, profile.agent())
        .and_then(|agent| agent.found_login(profile, account))
        .ok_or(SourceError::NotAvailable(Unavailable::KeyGone))?;

    provider.read(account, &login)
}

/// The JSON answer of a `GET` of `url` with `key` as its bearer token and
/// the `extra` headers. A refused key is "key refused".
pub(super) fn ask_with_key(
    client: &dyn HttpsGet,
    url: &str,
    key: &AccountKey,
    extra: Vec<(&'static str, String)>,
) -> Result<Value, SourceError> {
    let mut headers = vec![
        ("Authorization", format!("Bearer {}", key.key().secret())),
        ("Accept", String::from("application/json")),
    ];
    headers.extend(extra);
    let request = HttpsRequest {
        url: url.to_string(),
        headers,
    };

    json_answer(client.get(&request)).map_err(|error| match error {
        SourceError::NotLoggedIn => SourceError::KeyRefused,
        other => other,
    })
}

/// A number of an answer, written as a JSON number or as decimal text,
/// such as `"110.00"`.
pub(super) fn number_in(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// The id of the provider of `account`; empty for one named by a host,
/// which no reader of this crate reads.
fn provider_id(account: &Account) -> &str {
    account.provider().as_id().map_or("", ProviderId::as_str)
}

/// What OTTY reads of `account` with its own key, if anything.
fn own_source(account: &Account) -> Option<OwnSource> {
    let provider = provider_id(account);

    match account.identity() {
        AccountIdentity::AddedKey(_) if is_added_as_organization(provider) => {
            Some(OwnSource::Bill)
        },
        AccountIdentity::AddedKey(_) => Some(OwnSource::Balance),
        AccountIdentity::KeyFingerprint(_) => {
            has_balance(provider).then_some(OwnSource::Balance)
        },
        AccountIdentity::AccountId(_)
            if provider == OPENAI && account.kind() != AccountKind::Money =>
        {
            Some(OwnSource::ChatGptLimits)
        },
        _ => None,
    }
}

/// Whether an account added by hand at the provider `provider` is an
/// organization, whose bill its admin key reads (Anthropic, OpenAI),
/// rather than a key with money left on it.
fn is_added_as_organization(provider: &str) -> bool {
    matches!(provider, ANTHROPIC | OPENAI)
}

/// Whether a key at the provider `provider` tells its money: DeepSeek,
/// OpenRouter, Moonshot and Kilo do; an Anthropic or OpenAI key does not.
fn has_balance(provider: &str) -> bool {
    matches!(provider, DEEPSEEK | OPENROUTER | MOONSHOT | KILO)
}

/// The money of the key `key` at the provider `provider`.
fn balance(
    client: &dyn HttpsGet,
    provider: &str,
    key: &AccountKey,
) -> Result<Balance, SourceError> {
    match provider {
        DEEPSEEK => deepseek::balance(client, key),
        OPENROUTER => openrouter::balance(client, key),
        MOONSHOT => moonshot::balance(client, key),
        KILO => kilo::balance(client, key),
        _ => Err(SourceError::NotAvailable(Unavailable::NoAccountSource)),
    }
}

/// The bill of the organization of the admin key `key` at the provider
/// `provider`.
fn bill(
    client: Box<dyn HttpsGet>,
    provider: &str,
    key: &AccountKey,
    now: SystemTime,
) -> Result<Bill, SourceError> {
    match provider {
        ANTHROPIC => CostReport::new(client).bill(key.key(), now),
        OPENAI => openai_costs::bill(client.as_ref(), key.key(), now),
        _ => Err(SourceError::NotAvailable(Unavailable::NoAccountSource)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::admin_key::AdminKeyRef;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::known_agents;
    use crate::provider::ProviderRef;
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{
        NOW, TempDir, answering, at, chatgpt_login, key_account, organization,
        write_file,
    };

    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-made-up-deepseek-key-0000-9b1d";

    fn no_variables(_: &str) -> Option<String> {
        None
    }

    fn deepseek_in(variable: &str) -> Account {
        Account::added(
            ProviderRef::of_id("deepseek"),
            AdminKeyRef::EnvVar(variable.to_string()),
        )
    }

    /// A read that reaches `client`, no keyring and the known agents.
    fn read(
        client: MockHttpsGet,
        account: &Account,
        place: &KeyPlace,
        env_var: impl Fn(&str) -> Option<String>,
    ) -> Result<AccountNumbers, SourceError> {
        let mut keyring = MockKeyringRead::new();
        keyring.expect_password().never();
        let agents = known_agents();
        let reach = Reach {
            client: Box::new(client),
            keyring: &keyring,
            agents: &agents,
        };

        reach.read(account, place, at(NOW), env_var)
    }

    fn never_asked() -> MockHttpsGet {
        let mut client = MockHttpsGet::new();
        client.expect_get().never();
        client
    }

    #[test]
    fn each_account_has_the_source_of_its_provider() {
        let added = |provider| {
            Account::added(provider, AdminKeyRef::EnvVar("KEY".into()))
        };
        let cases = [
            (
                added(ProviderRef::of_id("deepseek")),
                Some(SourceId::DeepSeekBalance),
            ),
            (
                added(ProviderRef::of_id("openrouter")),
                Some(SourceId::OpenRouterKey),
            ),
            (
                added(ProviderRef::of_id("moonshot")),
                Some(SourceId::MoonshotBalance),
            ),
            (
                added(ProviderRef::of_id("anthropic")),
                Some(SourceId::AnthropicCostReport),
            ),
            (
                added(ProviderRef::of_id("openai")),
                Some(SourceId::OpenAiCosts),
            ),
            (
                key_account(ProviderRef::of_id("kilo"), "made-up-kilo-token"),
                Some(SourceId::KiloBalance),
            ),
            (
                key_account(ProviderRef::of_id("openai"), "sk-made-up"),
                None,
            ),
            (
                key_account(ProviderRef::of_id("anthropic"), "sk-ant-made-up"),
                None,
            ),
            (chatgpt_login("made-up"), Some(SourceId::ChatGptUsageApi)),
            (organization("made-up-org"), None),
        ];

        let registry = Registry::built_in(&known_agents());
        for (account, source) in cases {
            assert_eq!(
                own_source_of(&account, &registry),
                source,
                "{account:?}"
            );
        }
    }

    /// Spec 2026-11-19-agent-plugins, "What the UI shows": with no entry
    /// for its provider, an account has no Left or Limits rows, so nothing
    /// is read for it; a key for a host is read once an entry serves it.
    #[test]
    fn an_account_whose_provider_no_entry_serves_has_no_source() {
        let registry = Registry::built_in(&known_agents());
        let without = registry
            .clone()
            .without_provider(&ProviderId::new("deepseek"));
        let host = ProviderRef::Host(String::from("api.deepseek.com"));
        let by_host = key_account(host.clone(), "sk-made-up");
        let resolved = key_account(registry.resolve(&host), "sk-made-up");

        assert_eq!(own_source_of(&by_host, &without), None);
        assert_eq!(
            own_source_of(&resolved, &registry),
            Some(SourceId::DeepSeekBalance)
        );
    }

    /// The owner's check: a DeepSeek key added by hand shows Left.
    #[test]
    fn an_added_key_is_read_from_its_variable_and_sent_once() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == "https://api.deepseek.com/user/balance"
                    && request.headers.contains(&(
                        "Authorization",
                        format!("Bearer {MADE_UP_KEY}"),
                    ))
            })
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 200,
                    body: br#"{"balance_infos": [{"currency": "USD", "total_balance": "4.20"}]}"#
                        .to_vec(),
                })
            });

        let numbers = read(
            client,
            &deepseek_in("DEEPSEEK_API_KEY"),
            &KeyPlace::Added,
            |name| {
                (name == "DEEPSEEK_API_KEY").then(|| MADE_UP_KEY.to_string())
            },
        );

        let Ok(AccountNumbers::Balance(balance)) = numbers else {
            panic!("a balance: {numbers:?}");
        };
        assert_eq!(balance.left().map(|left| left.value()), Some(4.2));
    }

    #[test]
    fn a_missing_key_sends_nothing() {
        let account = deepseek_in("DEEPSEEK_API_KEY");

        let numbers =
            read(never_asked(), &account, &KeyPlace::Added, no_variables);

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::AdminKeyMissing(
                AdminKeyRef::EnvVar(String::from("DEEPSEEK_API_KEY"))
            )))
        );
    }

    #[test]
    fn an_account_with_no_source_of_its_own_sends_nothing() {
        let numbers = read(
            never_asked(),
            &organization("made-up-org"),
            &KeyPlace::Added,
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
    }

    #[test]
    fn an_added_openai_organization_reads_its_bill() {
        let account = Account::added(
            ProviderRef::of_id("openai"),
            AdminKeyRef::EnvVar(String::from("OPENAI_ADMIN_KEY")),
        );
        let client = answering(200, r#"{"data": [], "has_more": false}"#);

        let numbers = read(client, &account, &KeyPlace::Added, |_| {
            Some(String::from("sk-admin-made-up"))
        });

        assert!(
            matches!(numbers, Ok(AccountNumbers::Bill(_))),
            "{numbers:?}"
        );
    }

    /// Spec 2026-11-19 (issue 12): the Kilo login is read again from
    /// `auth.json` of the profile, with its team, and its balance asked
    /// for.
    #[test]
    fn a_kilo_login_is_read_again_from_the_files_of_its_profile() {
        let temp = TempDir::new();
        let dir = temp.path().join("kilo");
        let token = "made-up-kilo-token-0000-4c2e";
        write_file(
            &dir.join("auth.json"),
            &format!(
                r#"{{"kilo": {{"type": "oauth", "refresh": "made-up-refresh",
                "access": "{token}", "expires": 1, "accountId": "made-up-team"}}}}"#
            ),
        );
        let profile = crate::Profile::default_for(
            crate::AgentId::new("kilo"),
            "Kilo",
            dir.clone(),
        );
        let account = key_account(ProviderRef::of_id("kilo"), token);
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(move |request| {
                request.url == "https://api.kilo.ai/api/profile/balance"
                    && request
                        .headers
                        .contains(&("Authorization", format!("Bearer {token}")))
                    && request.headers.contains(&(
                        "x-kilocode-organizationid",
                        String::from("made-up-team"),
                    ))
            })
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 200,
                    body: br#"{"balance": 3}"#.to_vec(),
                })
            });

        let numbers = read(
            client,
            &account,
            &KeyPlace::ProfileFiles(profile),
            no_variables,
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    #[test]
    fn a_key_no_longer_in_its_files_sends_nothing() {
        let profile = crate::Profile::default_for(
            crate::AgentId::new("kilo"),
            "Kilo",
            PathBuf::from("/no/such/kilo"),
        );
        let account =
            key_account(ProviderRef::of_id("kilo"), "made-up-kilo-token");

        let numbers = read(
            never_asked(),
            &account,
            &KeyPlace::ProfileFiles(profile),
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::KeyGone))
        );
    }

    /// Spec 2026-11-19 (issue 10): a key of the keyring is read again from
    /// the keyring at the request.
    #[test]
    fn a_key_of_the_keyring_is_read_again_from_the_keyring() {
        let mut keyring = MockKeyringRead::new();
        keyring.expect_password().times(1).returning(|_, _| {
            Ok(Some(format!(r#"{{"DEEPSEEK_API_KEY": "{MADE_UP_KEY}"}}"#)))
        });
        let client = answering(
            200,
            r#"{"balance_infos": [{"currency": "CNY", "total_balance": "1"}]}"#,
        );
        let agents = known_agents();
        let reach = Reach {
            client: Box::new(client),
            keyring: &keyring,
            agents: &agents,
        };
        let account = Account::of_key(
            ProviderRef::of_id("deepseek"),
            &crate::key_fingerprint::SeenKey::of(MADE_UP_KEY).expect("a key"),
            crate::account::FoundIn::SystemKeyring,
        );

        let numbers = reach.read(
            &account,
            &KeyPlace::Keyring(vec![KeyringEntry::goose()]),
            at(NOW),
            no_variables,
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    #[test]
    fn a_number_is_read_from_a_number_or_from_decimal_text() {
        assert_eq!(number_in(&serde_json::json!(1.5)), Some(1.5));
        assert_eq!(number_in(&serde_json::json!(" 110.00 ")), Some(110.0));
        assert_eq!(number_in(&serde_json::json!("ten")), None);
        assert_eq!(number_in(&serde_json::json!(null)), None);
    }
    fn example_login() -> otty_plugin_host::agent::Login {
        use otty_plugin_host::agent::{
            JsonField, KeyPlace as PartPlace, LoginKind,
            ProviderRef as PartProvider,
        };

        use crate::testing::no_identity;

        otty_plugin_host::agent::Login {
            kind: LoginKind::ApiKey,
            for_whom: PartProvider::Id("example-cloud".to_string()),
            place: PartPlace::JsonField(JsonField {
                file: "/home/user/.config-Example/auth.json".to_string(),
                pointer: "/api_key".to_string(),
            }),
            identity: no_identity(),
            plan: None,
        }
    }

    /// Spec "How plugins talk to OTTY": an account a provider part serves
    /// is read by that part, once, with the login the agent part of its
    /// profile found it by; the built-in readers are not asked.
    #[test]
    fn an_account_a_provider_part_serves_is_read_by_it_once() {
        use otty_plugin_host::provider::{
            AccountFacts, AccountNumbers as PartNumbers, Money,
        };

        use crate::plugin_provider::MockProviderPartCalls;
        use crate::testing::{MockAgent, mock_entry, no_identity, profile};

        let twelve_dollars = || {
            let facts = AccountFacts {
                identity: no_identity(),
                plan: None,
            };
            let numbers = PartNumbers {
                windows: Vec::new(),
                credits: None,
                left: Some(Money {
                    amount: 12.0,
                    currency: "USD".to_string(),
                }),
                key_limit: None,
                spent_this_month: None,
                plan: None,
            };
            (facts, numbers)
        };

        let example = profile("Example");
        let account =
            key_account(ProviderRef::of_id("example-cloud"), "made-up");
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("test-agent", "test-agent"));
        agent
            .expect_found_login()
            .times(1)
            .returning(|_, _| Some(example_login()));
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .times(1)
            .returning(move |_, _| Ok(twelve_dollars()));
        let providers = [PluginProvider::made_up(
            "example-cloud",
            &["example.com"],
            calls,
        )];
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent)];

        let numbers = read_account_numbers(
            &agents,
            &providers,
            &account,
            &KeyPlace::ProfileFiles(example),
            at(NOW),
            |_| None,
        );
        let added = read_account_numbers(
            &agents,
            &providers,
            &account,
            &KeyPlace::Added,
            at(NOW),
            |_| None,
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
        assert_eq!(
            added,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
    }

    #[test]
    fn a_provider_a_plugin_reads_reads_the_accounts_found_in_profiles() {
        use crate::provider::ProviderEntry;
        use crate::registry::PluginPart;

        let part = PluginPart::new("example", "Example", "0.1.0", "", "cloud");
        let registry = Registry::new(
            Vec::new(),
            vec![
                ProviderEntry::new(
                    ProviderId::new("example-cloud"),
                    "Example Cloud",
                    Reader::Plugin(part),
                )
                .with_hosts(["example.com"]),
            ],
        );
        let found = key_account(ProviderRef::of_id("example-cloud"), "made-up");
        let added = Account::added(
            ProviderRef::of_id("example-cloud"),
            AdminKeyRef::EnvVar("EXAMPLE_KEY".into()),
        );

        assert_eq!(own_source_of(&found, &registry), Some(SourceId::Plugin));
        assert_eq!(own_source_of(&added, &registry), None);
    }
}
