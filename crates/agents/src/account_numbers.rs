//! The numbers of an account (spec 2026-11-19, Decisions 5 and 7). The
//! provider part of the plugin that reads the account reads them (spec
//! 2026-11-19-agent-plugins, issues 15 and 29): the money of a key and the
//! limits of a subscription login. With each read the part gets the keys
//! the user gave for the account after a need of the part asked for them
//! (issue 20), and every profile the account was found in, where it may
//! ask the agent itself, such as `claude /usage`; which way it reads is
//! its business, and with no way that works it answers nothing. An
//! account no plugin reads is not read.
//!
//! OTTY keeps no key: a key the user gave is read again right before the
//! request, from where they keep it, and reaches the part only as a
//! handle; a key an agent part found stays where it is.

use crate::account::Account;
use crate::agent::{Agent, agent_of};
use crate::balance::Balance;
use crate::error::{SourceError, Unavailable};
use crate::given_key::{GivenKey, read_given_keys};
use crate::limits::Limits;
use crate::plugin_provider::PluginProvider;
use crate::plugin_values::found_profile;
use crate::profile::Profile;
use crate::refused_login::to_holding_agent;
use crate::walk::Answer;

/// What OTTY read of an account with its own key.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountNumbers {
    /// What a key told of its money: the money left, the limit of the
    /// key, the money spent this month and the credits.
    Balance(Balance),
    /// The limits of a subscription login.
    Limits(Answer<Limits>),
}

/// The numbers of `account`, read by the provider part of
/// `providers` that reads it, with the key of the login the agent part of
/// `agents` found it by in the first of `found_in`, the profiles it was
/// found in, and the keys the user gave for it (`given`), read now from
/// their files or their variables (`env_var`). The part is told of every
/// profile of `found_in`. The part's programs and hidden terminals start
/// only when `starts_programs`, while Agents mode is on screen. It blocks.
///
/// # Errors
///
/// An account no part reads, or whose login the agent part no longer
/// finds, is "not available" and nothing is sent; a key that is not at
/// its place is the part's to tell, as nothing; what the user must do,
/// such as give another key, is the part's need,
/// [`SourceError::NeedsUser`]. A login of the profile the provider
/// refused is the need of the agent part that found it, else
/// [`SourceError::LoginRefused`].
pub fn read_account_numbers(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    account: &Account,
    found_in: &[Profile],
    given: &[GivenKey],
    starts_programs: bool,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<AccountNumbers, SourceError> {
    let no_source = || SourceError::NotAvailable(Unavailable::NoAccountSource);
    let gone = || SourceError::NotAvailable(Unavailable::KeyGone);
    let provider = providers
        .iter()
        .find(|part| part.reads(account))
        .ok_or_else(no_source)?;
    let profile = found_in.first().ok_or_else(gone)?;
    let agent = agent_of(agents, profile.agent()).ok_or_else(gone)?;
    let login = agent.found_login(profile, account).ok_or_else(gone)?;

    let found_profiles = found_profiles(agents, found_in);
    let given = read_given_keys(given, &env_var)?;

    provider
        .read(account, &login, found_profiles, &given, starts_programs)
        .map_err(|error| to_holding_agent(error, agent, profile, &login))
}

/// Each of `profiles` as a provider part is told an account was found in
/// it, in their order; a profile of an agent none of `agents` is, is left
/// out.
pub(crate) fn found_profiles(
    agents: &[Box<dyn Agent>],
    profiles: &[Profile],
) -> Vec<otty_plugin_host::agents::provider::FoundProfile> {
    profiles
        .iter()
        .filter_map(|profile| {
            let agent = agent_of(agents, profile.agent())?;
            Some(found_profile(agent, profile))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_plugin_host::agents::provider::{
        AccountFacts, AccountNumbers as PartNumbers, Money,
    };
    use otty_plugin_host::{KeySource, PluginPart};

    use super::*;
    use crate::given_key::KeyRef;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::{ProviderEntry, ProviderId, ProviderRef};
    use crate::testing::{key_account, no_identity, read_by_example};

    fn no_variables(_: &str) -> Option<String> {
        None
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
            recorded_at_ms: None,
        };

        (facts, numbers)
    }

    /// A made-up agent `id` whose part finds [`example_login`] in any
    /// profile and starts `<id>` there with the profile's folder.
    fn agent_finding_the_login(id: &str) -> Box<dyn Agent> {
        let mut agent = crate::testing::MockAgent::new();
        agent
            .expect_entry()
            .return_const(crate::testing::mock_entry(id, id));
        agent
            .expect_found_login()
            .returning(|_, _| Some(example_login()));
        let program = id.to_string();
        agent.expect_launch().returning(move |profile, _| {
            let folder = profile.config_dir().display().to_string();
            crate::AgentCommand::new(program.as_str())
                .with_env("HOME_OF", folder)
        });

        Box::new(agent)
    }

    /// The account of a made-up key the plugin `example` reads.
    fn example_key() -> Account {
        read_by_example(key_account(
            ProviderRef::of_id("example-cloud"),
            "made-up",
        ))
    }

    /// The made-up provider `example-cloud` whose part is called through
    /// `calls`.
    fn example_cloud(calls: MockProviderPartCalls) -> [PluginProvider; 1] {
        [PluginProvider::made_up(
            "example-cloud",
            &["example.com"],
            calls,
        )]
    }

    /// Issue 29: the provider part is told every profile the account was
    /// found in, with the agent part that found it and the command that
    /// starts the agent there, and starts its programs only when the read
    /// may.
    #[test]
    fn the_part_is_told_every_profile_the_account_was_found_in() {
        let profiles = vec![
            crate::testing::profile("Home"),
            crate::Profile::default_for(
                crate::AgentId::new("other-agent"),
                "Other",
                PathBuf::from("/home/user/.other"),
            ),
        ];
        let agents = vec![
            agent_finding_the_login("test-agent"),
            agent_finding_the_login("other-agent"),
        ];
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|_, found, _, starts_programs| {
                let folders: Vec<(&str, &str, &str)> = found
                    .found_in
                    .iter()
                    .map(|profile| {
                        (
                            profile.agent.as_str(),
                            profile.folder.as_str(),
                            profile.launch.program.as_str(),
                        )
                    })
                    .collect();
                folders
                    == [
                        ("test-agent", "/home/user/.config-Home", "test-agent"),
                        ("other-agent", "/home/user/.other", "other-agent"),
                    ]
                    && found.found_in[0].plugin == "example"
                    && !*starts_programs
            })
            .times(1)
            .returning(|_, _, _, _| Ok(four_dollars_twenty()));

        let numbers = read_account_numbers(
            &agents,
            &example_cloud(calls),
            &example_key(),
            &profiles,
            &[],
            false,
            no_variables,
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    /// Issue 29: two plugins serve one provider; each reads its own copy
    /// of the account, and only its own.
    #[test]
    fn each_plugin_reads_its_own_copy_of_an_account() {
        let part = |plugin: &str, times: usize| {
            let mut calls = MockProviderPartCalls::new();
            calls
                .expect_read_numbers()
                .times(times)
                .returning(|_, _, _, _| Ok(four_dollars_twenty()));
            let entry = ProviderEntry::new(
                ProviderId::new("example-cloud"),
                "Example Cloud",
                PluginPart::new(plugin, plugin, "0.1.0", "", "example-cloud"),
            );
            PluginProvider::of_calls(
                entry,
                otty_plugin_host::PartHealth::default(),
                Box::new(calls),
            )
        };
        let providers = [part("example", 0), part("example-fork", 1)];
        let agents = vec![agent_finding_the_login("test-agent")];
        let account =
            key_account(ProviderRef::of_id("example-cloud"), "made-up")
                .with_read_by(Some(String::from("example-fork")));

        let numbers = read_account_numbers(
            &agents,
            &providers,
            &account,
            &[crate::testing::profile("Home")],
            &[],
            true,
            no_variables,
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    #[test]
    fn a_key_no_longer_in_its_files_sends_nothing() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let profile = crate::Profile::default_for(
            crate::AgentId::new("test-agent"),
            "Test agent",
            PathBuf::from("/no/such/test-agent"),
        );

        let numbers = read_account_numbers(
            &[],
            &example_cloud(calls),
            &example_key(),
            &[profile],
            &[],
            true,
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::KeyGone))
        );
    }

    /// An account no provider part reads is not read: nothing is sent and
    /// no key is looked at.
    #[test]
    fn an_account_no_part_reads_is_not_read() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let other = read_by_example(key_account(
            ProviderRef::of_id("other-cloud"),
            "made-up",
        ));

        let numbers = read_account_numbers(
            &[agent_finding_the_login("test-agent")],
            &example_cloud(calls),
            &other,
            &[crate::testing::profile("Home")],
            &[],
            true,
            |_| panic!("no key is read for an account no part reads"),
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
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

        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("test-agent", "test-agent"));
        agent
            .expect_launch()
            .returning(|_, _| crate::AgentCommand::new("test-agent"));
        agent
            .expect_found_login()
            .times(1)
            .returning(|_, _| Some(example_login()));
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|key, _, _, _| matches!(key, KeySource::Place(_)))
            .times(1)
            .returning(move |_, _, _, _| Ok(four_dollars_twenty()));
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent)];

        let numbers = read_account_numbers(
            &agents,
            &example_cloud(calls),
            &example_key(),
            &[profile("Example")],
            &[],
            true,
            |_| None,
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    /// Issue 20: a key the user gave for an `ask-key` need with a slot is
    /// read from where they keep it and reaches the next read of the
    /// account under its slot, as a handle.
    #[test]
    fn a_key_given_for_a_slot_reaches_the_read_under_its_slot() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|_, _, given, _| {
                matches!(given, [(slot, KeySource::Given(key))]
                    if slot == "admin" && key.as_str() == "sk-made-up-org")
            })
            .times(1)
            .returning(|_, _, _, _| Ok(four_dollars_twenty()));
        let given = [GivenKey::new(
            "admin",
            KeyRef::EnvVar(String::from("EXAMPLE_ORG_KEY")),
        )];

        let numbers = read_account_numbers(
            &[agent_finding_the_login("test-agent")],
            &example_cloud(calls),
            &example_key(),
            &[crate::testing::profile("Home")],
            &given,
            true,
            |name| {
                (name == "EXAMPLE_ORG_KEY")
                    .then(|| String::from("sk-made-up-org"))
            },
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    /// A key the user gave that is not where they keep it fails the read,
    /// so the user sees why; the part gets nothing.
    #[test]
    fn a_missing_given_key_gives_the_part_nothing() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let given = [GivenKey::new(
            "admin",
            KeyRef::EnvVar(String::from("EXAMPLE_ORG_KEY")),
        )];

        let numbers = read_account_numbers(
            &[agent_finding_the_login("test-agent")],
            &example_cloud(calls),
            &example_key(),
            &[crate::testing::profile("Home")],
            &given,
            true,
            no_variables,
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::KeyMissing(
                KeyRef::EnvVar(String::from("EXAMPLE_ORG_KEY"))
            )))
        );
    }
}
