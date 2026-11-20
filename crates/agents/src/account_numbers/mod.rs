//! The numbers OTTY reads of an account by itself, with the account's own
//! key, rather than through the limit sources of a profile (spec
//! 2026-11-19, Decisions 5 and 7). The provider part of a plugin that
//! serves the account reads them (spec 2026-11-19-agent-plugins, issue
//! 15): the money of a key, the limits of a subscription login and, with
//! an admin key the user added, the bill of an organization. The money
//! left on a Moonshot account is still read by a reader built into OTTY
//! until issue 17 removes it.
//!
//! OTTY keeps no key: the key is read again right before the request,
//! from where it was found ([`KeyPlace`]), and dropped right after it.
//! Every request has OTTY's `User-Agent`, follows no redirect and gives up
//! after 30 seconds ([`UreqClient`]).

mod moonshot;

use std::time::SystemTime;

use serde_json::Value;

use crate::account::{Account, AccountIdentity, FoundIn};
use crate::account_key::AccountKey;
use crate::admin_key::AdminKey;
use crate::agent::{Agent, agent_of};
use crate::balance::Balance;
use crate::bill::Bill;
use crate::built_in::MOONSHOT;
use crate::error::{SourceError, Unavailable};
use crate::https::{HttpsGet, HttpsRequest, UreqClient, json_answer};
use crate::keyring_entry::{KeyringEntry, key_in_keyring};
use crate::limits::Limits;
use crate::plugin_provider::{PluginProvider, part_source};
use crate::profile::Profile;
use crate::provider::AddAccountKind;
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
    /// What a key told of its money: the money left, the limit of the
    /// key, the money spent this month and the credits.
    Balance(Balance),
    /// The bill of an organization.
    Bill(Bill),
    /// The limits of a subscription login.
    Limits(Answer<Limits>),
}

/// What a read reaches: the network, the system keyring and the agents
/// that read their files again.
struct Reach<'a> {
    client: Box<dyn HttpsGet>,
    keyring: &'a dyn KeyringRead,
    agents: &'a [Box<dyn Agent>],
}

impl Reach<'_> {
    /// The numbers of `account` read by the reader built into OTTY with
    /// its key from `place`: the money left on a Moonshot account.
    fn read(
        self,
        account: &Account,
        place: &KeyPlace,
        env_var: impl Fn(&str) -> Option<String>,
    ) -> Result<AccountNumbers, SourceError> {
        if !has_own_balance(account) {
            return Err(SourceError::NotAvailable(
                Unavailable::NoAccountSource,
            ));
        }

        let key = self.key_of(account, place, env_var)?;

        moonshot::balance(self.client.as_ref(), &key)
            .map(AccountNumbers::Balance)
    }

    /// The numbers of `account` read by the provider part `provider`:
    /// with the key the user added by hand, read now from its file or its
    /// variable (`env_var`), which reads the bill of the organization when
    /// the part offers Add account with an admin key; with the key of the
    /// login an agent part found it by in the profile of `place`; or with
    /// the key found in the keyring.
    fn read_by_part(
        &self,
        provider: &PluginProvider,
        account: &Account,
        place: &KeyPlace,
        now: SystemTime,
        env_var: impl Fn(&str) -> Option<String>,
    ) -> Result<AccountNumbers, SourceError> {
        let gone = || SourceError::NotAvailable(Unavailable::KeyGone);

        match place {
            KeyPlace::Added => {
                let reference = account.added_key().ok_or(
                    SourceError::NotAvailable(Unavailable::NoAccountSource),
                )?;
                let key = reference.read(env_var)?;

                if is_organization_part(provider) {
                    return provider
                        .read_bill(account.provider(), &key, now)
                        .map(AccountNumbers::Bill);
                }
                provider.read_given(account, &key)
            },
            KeyPlace::ProfileFiles(profile) => {
                let login = agent_of(self.agents, profile.agent())
                    .and_then(|agent| agent.found_login(profile, account))
                    .ok_or_else(gone)?;

                provider.read(account, &login)
            },
            KeyPlace::Keyring(entries) => {
                let key =
                    self.keyring_key(account, entries)?.ok_or_else(gone)?;

                provider.read_given(account, &key)
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
            KeyPlace::Keyring(entries) => self
                .keyring_key(account, entries)?
                .map(AccountKey::new)
                .ok_or(gone),
        }
    }

    /// The key of `account` in `entries` of the keyring, read now.
    fn keyring_key(
        &self,
        account: &Account,
        entries: &[KeyringEntry],
    ) -> Result<Option<AdminKey>, SourceError> {
        key_in_keyring(self.keyring, entries, account).map_err(|failure| {
            SourceError::NotAvailable(Unavailable::Keyring(failure))
        })
    }
}

/// The source OTTY reads `account` through with its own key, by itself;
/// `None` for an account it does not read that way.
///
/// It is `None` while no entry of `registry` serves the account's
/// provider (spec 2026-11-19-agent-plugins, "What the UI shows": no Left
/// or Limits rows with no provider part). A provider a plugin part reads
/// reads every account found in a profile, with the key of its login;
/// every key found in the system keyring; and every key the user added by
/// hand when its part offers Add account, a key for its numbers or an
/// admin key for the bill of its organization, named after the provider
/// ([`SourceId::Named`]). The reader of Moonshot built into OTTY reads the
/// money of its keys.
///
/// ```
/// use otty_agents::{
///     Account, AdminKeyRef, ProviderId, ProviderRef, Registry, SourceId,
///     known_agents, own_source_of,
/// };
///
/// let registry = Registry::built_in(&known_agents());
/// let moonshot = Account::added(
///     ProviderRef::of_id("moonshot"),
///     AdminKeyRef::EnvVar("MOONSHOT_API_KEY".into()),
/// );
/// let without = registry
///     .clone()
///     .without_provider(&ProviderId::new("moonshot"));
///
/// assert_eq!(
///     own_source_of(&moonshot, &registry),
///     Some(SourceId::MoonshotBalance)
/// );
/// assert_eq!(own_source_of(&moonshot, &without), None);
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
        let is_in_keyring =
            account.found_in().contains(&FoundIn::SystemKeyring);
        let is_added =
            account.added_key().is_some() && entry.add_account().is_some();
        return (is_in_profile || is_in_keyring || is_added)
            .then(|| part_source(entry));
    }

    has_own_balance(account).then_some(SourceId::MoonshotBalance)
}

/// The numbers of `account` up to `now`, read with its key from `place`
/// by the provider part of `providers` that serves it, else by the reader
/// built into OTTY. `agents` read the files of a profile again, and
/// `env_var` gives the value of an environment variable, for a key the
/// user keeps in one. It blocks; a key in the keyring may make the system
/// ask the user to unlock it.
///
/// # Errors
///
/// An account with no source of its own, or whose key is no longer where
/// it was found, is "not available" and nothing is sent; a refused key is
/// [`SourceError::KeyRefused`], and a refused subscription login is "not
/// logged in" (it is never renewed).
pub fn read_account_numbers(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    account: &Account,
    place: &KeyPlace,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<AccountNumbers, SourceError> {
    let reach = Reach {
        client: Box::new(UreqClient::new()),
        keyring: &SystemKeyring,
        agents,
    };

    match providers.iter().find(|part| part.serves(account)) {
        Some(provider) => {
            reach.read_by_part(provider, account, place, now, env_var)
        },
        None => reach.read(account, place, env_var),
    }
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

/// Whether the reader built into OTTY reads the money left on `account`:
/// a Moonshot key, added by hand or found.
fn has_own_balance(account: &Account) -> bool {
    let is_moonshot = account
        .provider()
        .as_id()
        .is_some_and(|id| id.as_str() == MOONSHOT);

    is_moonshot
        && matches!(
            account.identity(),
            AccountIdentity::AddedKey(_) | AccountIdentity::KeyFingerprint(_)
        )
}

/// Whether a key added by hand for the provider `provider` is the admin
/// key of an organization, whose bill it reads, rather than a key with
/// numbers of its own: the part offers Add account with an admin key.
fn is_organization_part(provider: &PluginProvider) -> bool {
    provider.entry().add_account() == Some(AddAccountKind::AdminKey)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_plugin_host::KeySource;
    use otty_plugin_host::provider::{
        AccountFacts, AccountNumbers as PartNumbers, Bill as PartBill, Money,
    };

    use super::*;
    use crate::admin_key::AdminKeyRef;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::known_agents;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::{ProviderEntry, ProviderId, ProviderRef};
    use crate::registry::PluginPart;
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{
        NOW, TempDir, answering, at, chatgpt_login, key_account, no_identity,
        organization, write_file,
    };

    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-made-up-deepseek-key-0000-9b1d";

    /// A made-up Moonshot key, never a real one.
    const MOONSHOT_KEY: &str = "sk-made-up-moonshot-key-0000-7c3a";

    fn no_variables(_: &str) -> Option<String> {
        None
    }

    fn added(provider: &str, variable: &str) -> Account {
        Account::added(
            ProviderRef::of_id(provider),
            AdminKeyRef::EnvVar(variable.to_string()),
        )
    }

    /// A read by the built-in reader that reaches `client`, no keyring
    /// and the known agents.
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

        reach.read(account, place, env_var)
    }

    /// The source of the provider part of the entry named `name`.
    fn named(name: &str) -> SourceId {
        SourceId::Named(crate::SourceName::new(name))
    }

    fn never_asked() -> MockHttpsGet {
        let mut client = MockHttpsGet::new();
        client.expect_get().never();
        client
    }

    /// The registry entry of a provider a plugin part reads, offering Add
    /// account with `add_account`.
    fn plugin_entry(
        id: &str,
        host: &str,
        add_account: Option<AddAccountKind>,
    ) -> ProviderEntry {
        let part = PluginPart::new(id, id, "0.1.0", "", id);
        let entry =
            ProviderEntry::new(ProviderId::new(id), id, Reader::Plugin(part))
                .with_hosts([host]);

        match add_account {
            Some(kind) => entry.with_add_account(kind),
            None => entry,
        }
    }

    /// What a provider part reads of a key with 4.20 dollars left.
    fn four_dollars_twenty() -> (AccountFacts, PartNumbers) {
        let facts = AccountFacts {
            identity: no_identity(),
            plan: None,
        };
        let numbers = PartNumbers {
            windows: Vec::new(),
            credits: None,
            extra_usage: None,
            left: Some(Money {
                amount: 4.2,
                currency: "USD".to_string(),
            }),
            key_limit: None,
            spent_this_month: None,
            plan: None,
        };

        (facts, numbers)
    }

    #[test]
    fn each_account_has_the_source_of_its_provider() {
        let mut providers = vec![
            plugin_entry(
                "anthropic",
                "anthropic.com",
                Some(AddAccountKind::AdminKey),
            ),
            plugin_entry(
                "openai",
                "openai.com",
                Some(AddAccountKind::AdminKey),
            ),
        ];
        let built_in = Registry::built_in(&known_agents());
        providers.extend(built_in.providers().iter().cloned());
        let registry = Registry::new(Vec::new(), providers);
        let cases = [
            (added("moonshot", "KEY"), Some(SourceId::MoonshotBalance)),
            (
                key_account(ProviderRef::of_id("moonshot"), "sk-made-up"),
                Some(SourceId::MoonshotBalance),
            ),
            (added("anthropic", "KEY"), Some(named("anthropic"))),
            (added("openai", "KEY"), Some(named("openai"))),
            (
                key_account(ProviderRef::of_id("openai"), "sk-made-up"),
                Some(named("openai")),
            ),
            (chatgpt_login("made-up"), Some(named("openai"))),
            (organization("made-up-org"), Some(named("anthropic"))),
            (
                key_account(ProviderRef::of_id("deepseek"), "sk-made-up"),
                None,
            ),
        ];

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
            .without_provider(&ProviderId::new("moonshot"));
        let host = ProviderRef::Host(String::from("api.moonshot.ai"));
        let by_host = key_account(host.clone(), "sk-made-up");
        let resolved = key_account(registry.resolve(&host), "sk-made-up");

        assert_eq!(own_source_of(&by_host, &without), None);
        assert_eq!(
            own_source_of(&resolved, &registry),
            Some(SourceId::MoonshotBalance)
        );
    }

    /// A key added by hand for a provider a reader built into OTTY reads
    /// shows Left.
    #[test]
    fn an_added_key_is_read_from_its_variable_and_sent_once() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == "https://api.moonshot.ai/v1/users/me/balance"
                    && request.headers.contains(&(
                        "Authorization",
                        format!("Bearer {MOONSHOT_KEY}"),
                    ))
            })
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 200,
                    body: br#"{"data": {"available_balance": 4.2}}"#.to_vec(),
                })
            });

        let numbers = read(
            client,
            &added("moonshot", "MOONSHOT_API_KEY"),
            &KeyPlace::Added,
            |name| {
                (name == "MOONSHOT_API_KEY").then(|| MOONSHOT_KEY.to_string())
            },
        );

        let Ok(AccountNumbers::Balance(balance)) = numbers else {
            panic!("a balance: {numbers:?}");
        };
        assert_eq!(balance.left().map(|left| left.value()), Some(4.2));
    }

    #[test]
    fn a_missing_key_sends_nothing() {
        let account = added("moonshot", "MOONSHOT_API_KEY");

        let numbers =
            read(never_asked(), &account, &KeyPlace::Added, no_variables);

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::AdminKeyMissing(
                AdminKeyRef::EnvVar(String::from("MOONSHOT_API_KEY"))
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
    fn a_key_no_longer_in_its_files_sends_nothing() {
        let profile = crate::Profile::default_for(
            crate::AgentId::new("test-agent"),
            "Test agent",
            PathBuf::from("/no/such/test-agent"),
        );
        let account =
            key_account(ProviderRef::of_id("moonshot"), "sk-made-up-moonshot");

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
            Ok(Some(format!(r#"{{"MOONSHOT_API_KEY": "{MOONSHOT_KEY}"}}"#)))
        });
        let client = answering(200, r#"{"data": {"available_balance": 1}}"#);
        let agents = known_agents();
        let reach = Reach {
            client: Box::new(client),
            keyring: &keyring,
            agents: &agents,
        };
        let account = Account::of_key(
            ProviderRef::of_id("moonshot"),
            &crate::key_fingerprint::SeenKey::of(MOONSHOT_KEY).expect("a key"),
            FoundIn::SystemKeyring,
        );

        let numbers = reach.read(
            &account,
            &KeyPlace::Keyring(vec![KeyringEntry::goose()]),
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
        use crate::testing::{MockAgent, mock_entry, profile};

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
            .returning(move |_, _| Ok(four_dollars_twenty()));
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
    fn a_provider_a_plugin_reads_reads_found_and_added_accounts() {
        let found = key_account(ProviderRef::of_id("example-cloud"), "made-up");
        let in_keyring = Account::of_key(
            ProviderRef::of_id("example-cloud"),
            &crate::key_fingerprint::SeenKey::of("made-up-key").expect("a key"),
            FoundIn::SystemKeyring,
        );
        let added = added("example-cloud", "EXAMPLE_KEY");

        let part = named("example-cloud");
        for (add_account, added_source) in [
            (Some(AddAccountKind::Key), Some(part)),
            (Some(AddAccountKind::AdminKey), Some(part)),
            (None, None),
        ] {
            let registry = Registry::new(
                Vec::new(),
                vec![plugin_entry("example-cloud", "example.com", add_account)],
            );

            assert_eq!(own_source_of(&found, &registry), Some(part));
            assert_eq!(own_source_of(&in_keyring, &registry), Some(part));
            assert_eq!(
                own_source_of(&added, &registry),
                added_source,
                "{add_account:?}"
            );
        }
    }

    /// Issue 16: a key added by hand, in a file or a variable, is read by
    /// OTTY and reaches the provider part that serves it as a handle,
    /// once; the built-in reader is not asked.
    #[test]
    fn an_added_key_reaches_the_provider_part_as_a_handle() {
        let temp = TempDir::new();
        let file = temp.path().join("deepseek.key");
        write_file(&file, &format!("{MADE_UP_KEY}\n"));
        let accounts = [
            added("deepseek", "DEEPSEEK_API_KEY"),
            Account::added(
                ProviderRef::of_id("deepseek"),
                AdminKeyRef::File(file),
            ),
        ];

        for account in accounts {
            let mut calls = MockProviderPartCalls::new();
            calls
                .expect_read_numbers()
                .withf(|key, _| {
                    matches!(key, KeySource::Given(given)
                        if given.as_str() == MADE_UP_KEY)
                })
                .times(1)
                .returning(|_, _| Ok(four_dollars_twenty()));
            let providers = [PluginProvider::made_up(
                "deepseek",
                &["deepseek.com"],
                calls,
            )];

            let numbers = read_account_numbers(
                &[],
                &providers,
                &account,
                &KeyPlace::Added,
                at(NOW),
                |name| {
                    (name == "DEEPSEEK_API_KEY")
                        .then(|| MADE_UP_KEY.to_string())
                },
            );

            let Ok(AccountNumbers::Balance(balance)) = numbers else {
                panic!("a balance: {numbers:?}");
            };
            assert_eq!(balance.left().map(|left| left.value()), Some(4.2));
        }
    }

    #[test]
    fn a_missing_added_key_gives_the_part_nothing() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let providers = [PluginProvider::made_up(
            "deepseek",
            &["deepseek.com"],
            calls,
        )];

        let numbers = read_account_numbers(
            &[],
            &providers,
            &added("deepseek", "DEEPSEEK_API_KEY"),
            &KeyPlace::Added,
            at(NOW),
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::AdminKeyMissing(
                AdminKeyRef::EnvVar(String::from("DEEPSEEK_API_KEY"))
            )))
        );
    }

    /// Issue 15: an organization added by hand with an admin key, at a
    /// provider whose part offers Add account with an admin key, reads its
    /// bill through the part; the key reaches it only as a handle.
    #[test]
    fn an_added_admin_key_reads_the_bill_through_the_part() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        calls
            .expect_read_bill()
            .withf(|key, _| {
                matches!(key, KeySource::Given(given)
                    if given.as_str() == "sk-admin-made-up")
            })
            .times(1)
            .returning(|_, _| Ok(PartBill { days: Vec::new() }));
        let provider =
            PluginProvider::made_up("openai", &["openai.com"], calls)
                .with_add_account(AddAccountKind::AdminKey);

        let numbers = read_account_numbers(
            &[],
            &[provider],
            &added("openai", "OPENAI_ADMIN_KEY"),
            &KeyPlace::Added,
            at(NOW),
            |_| Some(String::from("sk-admin-made-up")),
        );

        let Ok(AccountNumbers::Bill(bill)) = numbers else {
            panic!("a bill: {numbers:?}");
        };
        assert_eq!(bill.days().len(), 31);
    }

    /// Issue 15: a key found only in the keyring, which the user allowed
    /// OTTY to read, reaches the part that serves it as a handle.
    #[test]
    fn a_key_of_the_keyring_reaches_the_provider_part_as_a_handle() {
        let mut keyring = MockKeyringRead::new();
        keyring.expect_password().times(1).returning(|_, _| {
            Ok(Some(format!(r#"{{"DEEPSEEK_API_KEY": "{MADE_UP_KEY}"}}"#)))
        });
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|key, _| {
                matches!(key, KeySource::Given(given)
                    if given.as_str() == MADE_UP_KEY)
            })
            .times(1)
            .returning(|_, _| Ok(four_dollars_twenty()));
        let provider =
            PluginProvider::made_up("deepseek", &["deepseek.com"], calls);
        let reach = Reach {
            client: Box::new(never_asked()),
            keyring: &keyring,
            agents: &[],
        };
        let account = Account::of_key(
            ProviderRef::of_id("deepseek"),
            &crate::key_fingerprint::SeenKey::of(MADE_UP_KEY).expect("a key"),
            FoundIn::SystemKeyring,
        );

        let numbers = reach.read_by_part(
            &provider,
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
}
