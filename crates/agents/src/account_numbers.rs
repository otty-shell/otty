//! The numbers OTTY reads of an account by itself, with the account's own
//! key, rather than through the limit sources of a profile (spec
//! 2026-11-19, Decisions 5 and 7). The provider part of a plugin that
//! serves the account reads them (spec 2026-11-19-agent-plugins, issue
//! 15): the money of a key, the limits of a subscription login and, with
//! an admin key the user added, the bill of an organization. An account
//! whose provider no part serves is not read.
//!
//! OTTY keeps no key: a key the user added is read again right before the
//! request, from where they keep it ([`KeyPlace`]), and reaches the part
//! only as a handle; a key an agent part found stays where it is.

use std::time::SystemTime;

use crate::account::{Account, FoundIn};
use crate::agent::{Agent, agent_of};
use crate::balance::Balance;
use crate::bill::Bill;
use crate::error::{SourceError, Unavailable};
use crate::limits::Limits;
use crate::plugin_provider::{PluginProvider, part_source};
use crate::profile::Profile;
use crate::provider::AddAccountKind;
use crate::registry::Registry;
use crate::source_id::SourceId;
use crate::walk::Answer;

/// Where the key or the login token of an account is read again for a
/// request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPlace {
    /// The file or the variable the user gave for an account they added.
    Added,
    /// The files of this profile, where the account was found; the agent
    /// part that found it names the place of its key again.
    ProfileFiles(Profile),
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

/// The source OTTY reads `account` through with its own key, by itself;
/// `None` for an account it does not read that way.
///
/// It is `None` while no entry of `registry` serves the account's
/// provider (spec 2026-11-19-agent-plugins, "What the UI shows": no Left
/// or Limits rows with no provider part). The provider part that serves
/// it reads every account found in a profile, with the key of its login,
/// and every key the user added by hand when the part offers Add account,
/// a key for its numbers or an admin key for the bill of its
/// organization, named after the provider ([`SourceId::Named`]).
///
/// ```
/// use otty_agents::{
///     Account, AdminKeyRef, ProviderRef, Registry, own_source_of,
/// };
///
/// let added = Account::added(
///     ProviderRef::of_id("example-cloud"),
///     AdminKeyRef::EnvVar("EXAMPLE_API_KEY".into()),
/// );
///
/// assert_eq!(own_source_of(&added, &Registry::default()), None);
/// ```
pub fn own_source_of(
    account: &Account,
    registry: &Registry,
) -> Option<SourceId> {
    let entry = registry.provider_of(account.provider())?;

    let is_in_profile = account
        .found_in()
        .iter()
        .any(|place| matches!(place, FoundIn::ProfileFolder(_)));
    let is_added =
        account.added_key().is_some() && entry.add_account().is_some();

    (is_in_profile || is_added).then(|| part_source(entry))
}

/// The numbers of `account` up to `now`, read by the provider part of
/// `providers` that serves it: with the key the user added by hand, read
/// now from its file or its variable (`env_var`), which reads the bill of
/// the organization when the part offers Add account with an admin key;
/// or with the key of the login the agent part of `agents` found it by in
/// the profile of `place`. It blocks.
///
/// # Errors
///
/// An account no part serves, or whose key is no longer where it was
/// found, is "not available" and nothing is sent; a refused key is
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
    let no_source = || SourceError::NotAvailable(Unavailable::NoAccountSource);
    let provider = providers
        .iter()
        .find(|part| part.serves(account))
        .ok_or_else(no_source)?;

    match place {
        KeyPlace::Added => {
            let reference = account.added_key().ok_or_else(no_source)?;
            let key = reference.read(env_var)?;

            if is_organization_part(provider) {
                return provider
                    .read_bill(account.provider(), &key, now)
                    .map(AccountNumbers::Bill);
            }
            provider.read_given(account, &key)
        },
        KeyPlace::ProfileFiles(profile) => {
            let login = agent_of(agents, profile.agent())
                .and_then(|agent| agent.found_login(profile, account))
                .ok_or(SourceError::NotAvailable(Unavailable::KeyGone))?;

            provider.read(account, &login)
        },
    }
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
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::{ProviderEntry, ProviderId, ProviderRef};
    use crate::registry::PluginPart;
    use crate::testing::{
        NOW, TempDir, at, chatgpt_login, key_account, no_identity,
        organization, write_file,
    };

    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-made-up-example-key-0000-9b1d";

    fn no_variables(_: &str) -> Option<String> {
        None
    }

    fn added(provider: &str, variable: &str) -> Account {
        Account::added(
            ProviderRef::of_id(provider),
            AdminKeyRef::EnvVar(variable.to_string()),
        )
    }

    /// The source of the provider part of the entry named `name`.
    fn named(name: &str) -> SourceId {
        SourceId::Named(crate::SourceName::new(name))
    }

    /// The registry entry of a provider a plugin part reads, offering Add
    /// account with `add_account`.
    fn plugin_entry(
        id: &str,
        host: &str,
        add_account: Option<AddAccountKind>,
    ) -> ProviderEntry {
        let part = PluginPart::new(id, id, "0.1.0", "", id);
        let entry = ProviderEntry::new(ProviderId::new(id), id, part)
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
        let registry = Registry::new(
            Vec::new(),
            vec![
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
            ],
        );
        let cases = [
            (added("anthropic", "KEY"), Some(named("anthropic"))),
            (added("openai", "KEY"), Some(named("openai"))),
            (
                key_account(ProviderRef::of_id("openai"), "sk-made-up"),
                Some(named("openai")),
            ),
            (chatgpt_login("made-up"), Some(named("openai"))),
            (organization("made-up-org"), Some(named("anthropic"))),
            (
                key_account(ProviderRef::of_id("example-cloud"), "sk-made-up"),
                None,
            ),
            (added("example-cloud", "KEY"), None),
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
        let registry = Registry::new(
            Vec::new(),
            vec![plugin_entry(
                "example-cloud",
                "example.com",
                Some(AddAccountKind::Key),
            )],
        );
        let without = registry
            .clone()
            .without_provider(&ProviderId::new("example-cloud"));
        let host = ProviderRef::Host(String::from("api.example.com"));
        let by_host = key_account(host.clone(), "sk-made-up");
        let resolved = key_account(registry.resolve(&host), "sk-made-up");

        assert_eq!(own_source_of(&by_host, &without), None);
        assert_eq!(
            own_source_of(&resolved, &registry),
            Some(named("example-cloud"))
        );
    }

    /// Issue 17: an account added by hand for a provider no plugin part
    /// serves is not read; nothing is sent and its key is not looked at.
    #[test]
    fn an_added_account_no_part_serves_is_not_read() {
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
            &added("example-cloud", "EXAMPLE_API_KEY"),
            &KeyPlace::Added,
            at(NOW),
            |_| panic!("no key is read for an account no part serves"),
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
    }

    #[test]
    fn a_key_no_longer_in_its_files_sends_nothing() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let providers = [PluginProvider::made_up(
            "example-cloud",
            &["example.com"],
            calls,
        )];
        let profile = crate::Profile::default_for(
            crate::AgentId::new("test-agent"),
            "Test agent",
            PathBuf::from("/no/such/test-agent"),
        );
        let account =
            key_account(ProviderRef::of_id("example-cloud"), "sk-made-up");

        let numbers = read_account_numbers(
            &[],
            &providers,
            &account,
            &KeyPlace::ProfileFiles(profile),
            at(NOW),
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::KeyGone))
        );
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
    /// profile found it by.
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
            assert_eq!(
                own_source_of(&added, &registry),
                added_source,
                "{add_account:?}"
            );
        }
    }

    /// Issue 16: a key added by hand, in a file or a variable, is read by
    /// OTTY and reaches the provider part that serves it as a handle,
    /// once.
    #[test]
    fn an_added_key_reaches_the_provider_part_as_a_handle() {
        let temp = TempDir::new();
        let file = temp.path().join("example.key");
        write_file(&file, &format!("{MADE_UP_KEY}\n"));
        let accounts = [
            added("example-cloud", "EXAMPLE_API_KEY"),
            Account::added(
                ProviderRef::of_id("example-cloud"),
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
                "example-cloud",
                &["example.com"],
                calls,
            )];

            let numbers = read_account_numbers(
                &[],
                &providers,
                &account,
                &KeyPlace::Added,
                at(NOW),
                |name| {
                    (name == "EXAMPLE_API_KEY").then(|| MADE_UP_KEY.to_string())
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
            "example-cloud",
            &["example.com"],
            calls,
        )];

        let numbers = read_account_numbers(
            &[],
            &providers,
            &added("example-cloud", "EXAMPLE_API_KEY"),
            &KeyPlace::Added,
            at(NOW),
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::AdminKeyMissing(
                AdminKeyRef::EnvVar(String::from("EXAMPLE_API_KEY"))
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
            PluginProvider::made_up("example-cloud", &["example.com"], calls)
                .with_add_account(AddAccountKind::AdminKey);

        let numbers = read_account_numbers(
            &[],
            &[provider],
            &added("example-cloud", "EXAMPLE_ADMIN_KEY"),
            &KeyPlace::Added,
            at(NOW),
            |_| Some(String::from("sk-admin-made-up")),
        );

        let Ok(AccountNumbers::Bill(bill)) = numbers else {
            panic!("a bill: {numbers:?}");
        };
        assert_eq!(bill.days().len(), 31);
    }
}
