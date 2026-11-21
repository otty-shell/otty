//! The numbers OTTY reads of an account by itself, with the account's own
//! key, rather than through the limit sources of a profile (spec
//! 2026-11-19, Decisions 5 and 7). The provider part of a plugin that
//! serves the account reads them (spec 2026-11-19-agent-plugins, issue
//! 15): the money of a key, the limits of a subscription login and, for
//! an account the user added, the bill when the part reads one. With each
//! read the part gets the keys the user gave for the account after a
//! need of the part asked for them (issue 20). An account whose provider
//! no part serves is not read.
//!
//! OTTY keeps no key: a key the user added or gave is read again right
//! before the request, from where they keep it ([`KeyPlace`]), and
//! reaches the part only as a handle; a key an agent part found stays
//! where it is.

use std::time::SystemTime;

use crate::account::{Account, FoundIn};
use crate::agent::{Agent, agent_of};
use crate::balance::Balance;
use crate::bill::Bill;
use crate::error::{SourceError, Unavailable};
use crate::given_key::{GivenKey, read_given_keys};
use crate::limits::Limits;
use crate::plugin_provider::PluginProvider;
use crate::profile::Profile;
use crate::provider::ProviderEntry;
use crate::refused_login::to_holding_agent;
use crate::registry::Registry;
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

/// The provider whose part OTTY reads `account` through with its own key,
/// by itself; `None` for an account it does not read that way.
///
/// It is `None` while no entry of `registry` serves the account's
/// provider (spec 2026-11-19-agent-plugins, "What the UI shows": no Left
/// or Limits rows with no provider part). The provider part that serves
/// it reads every account found in a profile, with the key of its login,
/// and every key the user added by hand when the part offers Add account.
///
/// ```
/// use otty_agents::{Account, KeyRef, ProviderRef, Registry, own_reader_of};
///
/// let added = Account::added(
///     ProviderRef::of_id("example-cloud"),
///     KeyRef::EnvVar("EXAMPLE_API_KEY".into()),
/// );
///
/// assert!(own_reader_of(&added, &Registry::default()).is_none());
/// ```
pub fn own_reader_of<'a>(
    account: &Account,
    registry: &'a Registry,
) -> Option<&'a ProviderEntry> {
    let entry = registry.provider_of(account.provider())?;

    let is_in_profile = account
        .found_in()
        .iter()
        .any(|place| matches!(place, FoundIn::ProfileFolder(_)));
    let is_added = account.added_key().is_some() && entry.can_add_by_hand();

    (is_in_profile || is_added).then_some(entry)
}

/// The numbers of `account` up to `now`, read by the provider part of
/// `providers` that serves it, with the keys the user gave for it
/// (`given`): with the key the user added by hand, read now from its file
/// or its variable (`env_var`), its bill when the part reads one, else
/// its numbers; or with the key of the login the agent part of `agents`
/// found it by in the profile of `place`. It blocks.
///
/// # Errors
///
/// An account no part serves, or whose key is no longer where it was
/// found, is "not available" and nothing is sent. So is a login with no
/// key at its place now, [`Unavailable::NoLoginKey`], which is no failure
/// to show (issue 13). What the user must do,
/// such as give another key, is the part's need,
/// [`SourceError::NeedsUser`]. A login of the profile the provider
/// refused is the need of the agent part that found it, else
/// [`SourceError::LoginRefused`].
pub fn read_account_numbers(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    account: &Account,
    place: &KeyPlace,
    given: &[GivenKey],
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
            let key = reference.read(&env_var)?;
            let given = read_given_keys(given, &env_var)?;

            match provider.read_given_bill(account, &key, &given, now) {
                Err(SourceError::NotAvailable(Unavailable::NoBill)) => {
                    provider.read_given(account, &key, &given)
                },
                bill => bill.map(AccountNumbers::Bill),
            }
        },
        KeyPlace::ProfileFiles(profile) => {
            let gone = || SourceError::NotAvailable(Unavailable::KeyGone);
            let agent = agent_of(agents, profile.agent()).ok_or_else(gone)?;
            let login = agent
                .found_login(profile, account)
                .map_err(SourceError::NotAvailable)?;
            let given = read_given_keys(given, &env_var)?;

            provider.read(account, &login, &given).map_err(|error| {
                to_holding_agent(error, agent, profile, &login)
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_plugin_host::agents::provider::{
        AccountFacts, AccountNumbers as PartNumbers, Bill as PartBill, Money,
    };
    use otty_plugin_host::{KeySource, PluginPart};

    use super::*;
    use crate::given_key::KeyRef;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::{ProviderEntry, ProviderId, ProviderRef};
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
            KeyRef::EnvVar(variable.to_string()),
        )
    }

    /// The name of the provider whose part reads `account` by itself.
    fn reader(account: &Account, registry: &Registry) -> Option<String> {
        own_reader_of(account, registry).map(|entry| entry.name().to_string())
    }

    /// The name of the provider of the entry named `name`.
    fn named(name: &str) -> String {
        name.to_string()
    }

    /// The registry entry of a provider a plugin part reads, offered by
    /// Add account when `add_by_hand`.
    fn plugin_entry(id: &str, host: &str, add_by_hand: bool) -> ProviderEntry {
        let part = PluginPart::new(id, id, "0.1.0", "", id);
        let entry = ProviderEntry::new(ProviderId::new(id), id, part)
            .with_hosts([host]);

        match add_by_hand {
            true => entry.with_add_by_hand(),
            false => entry,
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
                plugin_entry("anthropic", "anthropic.com", true),
                plugin_entry("openai", "openai.com", true),
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

        for (account, reader_name) in cases {
            assert_eq!(reader(&account, &registry), reader_name, "{account:?}");
        }
    }

    /// Spec 2026-11-19-agent-plugins, "What the UI shows": with no entry
    /// for its provider, an account has no Left or Limits rows, so nothing
    /// is read for it; a key for a host is read once an entry serves it.
    #[test]
    fn an_account_whose_provider_no_entry_serves_has_no_reader() {
        let registry = Registry::new(
            Vec::new(),
            vec![plugin_entry("example-cloud", "example.com", true)],
        );
        let without = registry
            .clone()
            .without_provider(&ProviderId::new("example-cloud"));
        let host = ProviderRef::Host(String::from("api.example.com"));
        let by_host = key_account(host.clone(), "sk-made-up");
        let resolved = key_account(registry.resolve(&host), "sk-made-up");

        assert_eq!(reader(&by_host, &without), None);
        assert_eq!(reader(&resolved, &registry), Some(named("example-cloud")));
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
            &[],
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
            &[],
            at(NOW),
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::KeyGone))
        );
    }

    fn example_login() -> otty_plugin_host::agents::agent::Login {
        use otty_plugin_host::agents::agent::{
            JsonField, KeyPlace as PartPlace, LoginKind,
            ProviderRef as PartProvider,
        };

        otty_plugin_host::agents::agent::Login {
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
            .returning(|_, _| Ok(example_login()));
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .times(1)
            .returning(move |_, _, _| Ok(four_dollars_twenty()));
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
            &[],
            at(NOW),
            |_| None,
        );
        let added = read_account_numbers(
            &agents,
            &providers,
            &account,
            &KeyPlace::Added,
            &[],
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

    /// Issue 13: the login an agent part found has no key at its place
    /// now, such as a Claude Code profile whose credentials file is `{}`.
    /// The provider part is not asked, and the answer is the quiet "no
    /// key of the login" rather than a failure of the part.
    #[test]
    fn a_login_without_its_key_does_not_ask_the_provider_part() {
        use crate::testing::{MockAgent, mock_entry, profile};

        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("test-agent", "test-agent"));
        agent
            .expect_found_login()
            .returning(|_, _| Err(Unavailable::NoLoginKey));
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let providers = [PluginProvider::made_up(
            "example-cloud",
            &["example.com"],
            calls,
        )];
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent)];

        let numbers = read_account_numbers(
            &agents,
            &providers,
            &key_account(ProviderRef::of_id("example-cloud"), "made-up"),
            &KeyPlace::ProfileFiles(profile("Example")),
            &[],
            at(NOW),
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoLoginKey))
        );
    }

    #[test]
    fn a_provider_a_plugin_reads_reads_found_and_added_accounts() {
        let found = key_account(ProviderRef::of_id("example-cloud"), "made-up");
        let added = added("example-cloud", "EXAMPLE_KEY");

        let part = named("example-cloud");
        for (add_by_hand, added_reader) in
            [(true, Some(part.clone())), (false, None)]
        {
            let registry = Registry::new(
                Vec::new(),
                vec![plugin_entry("example-cloud", "example.com", add_by_hand)],
            );

            assert_eq!(reader(&found, &registry), Some(part.clone()));
            assert_eq!(
                reader(&added, &registry),
                added_reader,
                "{add_by_hand:?}"
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
                KeyRef::File(file),
            ),
        ];

        for account in accounts {
            let mut calls = MockProviderPartCalls::new();
            calls.expect_read_bill().times(1).returning(|_, _, _, _| {
                Err(SourceError::NotAvailable(Unavailable::NoBill))
            });
            calls
                .expect_read_numbers()
                .withf(|key, _, _| {
                    matches!(key, KeySource::Given(given)
                        if given.as_str() == MADE_UP_KEY)
                })
                .times(1)
                .returning(|_, _, _| Ok(four_dollars_twenty()));
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
                &[],
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
            &[],
            at(NOW),
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::KeyMissing(
                KeyRef::EnvVar(String::from("EXAMPLE_API_KEY"))
            )))
        );
    }

    /// Issue 20: an account added by hand reads its bill when its part
    /// reads one, with its key and the keys given for it, each as a
    /// handle; its numbers are then not read.
    #[test]
    fn an_added_account_reads_its_bill_with_the_given_keys() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        calls
            .expect_read_bill()
            .withf(|key, _, given, _| {
                matches!(key, KeySource::Given(own)
                    if own.as_str() == "sk-made-up-own")
                    && matches!(given, [(slot, KeySource::Given(org))]
                        if slot == "admin" && org.as_str() == "sk-made-up-org")
            })
            .times(1)
            .returning(|_, _, _, _| Ok(PartBill { days: Vec::new() }));
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls)
                .with_add_by_hand();
        let given = [GivenKey::new(
            "admin",
            KeyRef::EnvVar(String::from("EXAMPLE_ORG_KEY")),
        )];

        let numbers = read_account_numbers(
            &[],
            &[provider],
            &added("example-cloud", "EXAMPLE_OWN_KEY"),
            &KeyPlace::Added,
            &given,
            at(NOW),
            |name| match name {
                "EXAMPLE_OWN_KEY" => Some(String::from("sk-made-up-own")),
                "EXAMPLE_ORG_KEY" => Some(String::from("sk-made-up-org")),
                _ => None,
            },
        );

        let Ok(AccountNumbers::Bill(bill)) = numbers else {
            panic!("a bill: {numbers:?}");
        };
        assert_eq!(bill.days().len(), 31);
    }
}
