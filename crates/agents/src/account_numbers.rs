//! The accounts the provider parts find, and the numbers of one of them
//! (spec 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8). Each
//! provider part finds its accounts in the profiles of its own plugin and
//! reads them itself, in the folders it found them in; OTTY joins no
//! accounts of two parts. With each read the part gets the keys the user
//! gave for the account after a need of the part asked for them (spec
//! 2026-11-19-agent-plugins, issue 20). Which way it reads is its
//! business, and with no way that works it answers nothing.
//!
//! OTTY keeps no key: a key the user gave is read again right before the
//! request, from where they keep it, and reaches the part only as a
//! handle.

use std::path::Path;
use std::sync::Arc;

use crate::account::Account;
use crate::agent::Agent;
use crate::api_prices::ApiPrices;
use crate::balance::Balance;
use crate::error::{SourceError, Unavailable};
use crate::given_key::{GivenKey, read_given_keys};
use crate::limits::Limits;
use crate::own_login::OwnLogin;
use crate::part_need::PartNeed;
use crate::plugin_provider::PluginProvider;
use crate::profile::Profile;
use crate::walk::Answer;

/// What OTTY read of an account.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountNumbers {
    /// What a key told of its money: the money left, the limit of the
    /// key, the money spent this month and the credits.
    Balance(Balance),
    /// The limits of a subscription.
    Limits(Answer<Limits>),
}

/// The accounts the provider parts found, and their needs that belong to
/// no account.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoundAccounts {
    accounts: Vec<Account>,
    needs: Vec<PartNeed>,
}

impl FoundAccounts {
    /// The accounts, part by part in the order of the parts, each in the
    /// order its part gave.
    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    /// What the parts asked the user that belongs to no account, such as
    /// a login file a part does not understand.
    pub fn needs(&self) -> &[PartNeed] {
        &self.needs
    }

    /// The accounts `accounts` with the needs `needs`.
    pub fn new(accounts: Vec<Account>, needs: Vec<PartNeed>) -> Self {
        Self { accounts, needs }
    }
}

/// The accounts each provider part of `providers` finds in the folders of
/// `profiles`, the shown profiles: only in those of the agent parts of its own
/// plugin it names in its `logins`, as `agents` tell; and in the folders in the
/// folder of the installed plugins `plugins_folder` of the own logins of
/// `own_logins` it made (spec 2026-11-23-accounts-apart-from-profiles, Decision
/// 6). A part that failed finds none this time; its failure counts in its
/// health. It blocks.
pub fn find_accounts(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    profiles: &[Profile],
    own_logins: &[OwnLogin],
    plugins_folder: &Path,
) -> FoundAccounts {
    let mut found = FoundAccounts::default();

    for provider in providers {
        let Ok((accounts, needs)) =
            provider.accounts(agents, profiles, own_logins, plugins_folder)
        else {
            continue;
        };

        found.accounts.extend(accounts);
        found.needs.extend(needs);
    }

    found
}

/// The numbers of `account`, read by the provider part of `providers` that
/// found it, in the folders it was found in, with the keys the user gave
/// for it (`given`), read now from their files or their variables
/// (`env_var`). The part's programs and hidden terminals start only when
/// `starts_programs`, while Agents mode is on screen. The part may price
/// tokens at `prices` while it reads, such as what the agent's own
/// sessions spent through a key this month. It blocks.
///
/// # Errors
///
/// An account no part reads is "not available" and nothing is sent; a
/// login that is not there now is the part's to tell, as nothing; what the
/// user must do, such as log in again or give another key, is the part's
/// need, [`SourceError::NeedsUser`].
pub fn read_account_numbers(
    providers: &[PluginProvider],
    account: &Account,
    given: &[GivenKey],
    starts_programs: bool,
    prices: &Arc<ApiPrices>,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<AccountNumbers, SourceError> {
    let provider = providers
        .iter()
        .find(|part| part.reads(account))
        .ok_or(SourceError::NotAvailable(Unavailable::NoAccountSource))?;

    let given = read_given_keys(given, &env_var)?;

    provider.read(account, &given, starts_programs, prices)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_plugin_host::KeySource;
    use otty_plugin_host::agents::provider::{
        AccountNumbers as PartNumbers, Money,
    };

    use super::*;
    use crate::account::{AccountKind, FoundIn};
    use crate::agent::AgentId;
    use crate::given_key::KeyRef;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::ProviderId;
    use crate::testing::{MockAgent, chatgpt_login, made_up_part};

    fn no_variables(_: &str) -> Option<String> {
        None
    }

    /// What a provider part reads of a key with 4.20 dollars left.
    fn four_dollars_twenty() -> PartNumbers {
        PartNumbers {
            windows: Vec::new(),
            credits: None,
            extra_usage: None,
            left: Some(Money {
                amount: 4.2,
                currency: "USD".to_string(),
                by_kind: None,
            }),
            key_limit: None,
            spent_this_month: None,
            plan: None,
            recorded_at_ms: None,
        }
    }

    /// A key of the plugin `example` read by its part `example-cloud`.
    fn example_key() -> Account {
        Account::new(
            "example",
            ProviderId::new("example-cloud"),
            "key:made-up",
            "key ••••a1f3",
            AccountKind::Key,
            vec![FoundIn::profile(
                AgentId::new("coder"),
                PathBuf::from("/home/user/.coder"),
            )],
        )
    }

    /// The made-up provider `example-cloud` of the plugin `example`, which
    /// reads the logins of Coder, whose part is called through `calls`.
    fn example_cloud(calls: MockProviderPartCalls) -> PluginProvider {
        PluginProvider::made_up("example-cloud", &["coder"], calls)
    }

    /// The made-up agent part `coder` of the plugin `example`.
    fn coder() -> Box<dyn Agent> {
        let mut agent = MockAgent::new();
        let entry = crate::agent::AgentEntry::new(
            AgentId::new("coder"),
            "Coder",
            "coder",
            "Example Cloud",
            made_up_part("coder"),
        );
        agent.expect_entry().return_const(entry);

        Box::new(agent)
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 1: each
    /// part finds its own accounts, and two parts that find one login give
    /// two accounts: OTTY joins none.
    #[test]
    fn each_part_finds_its_own_accounts_and_none_are_joined() {
        let part = |id: &str| {
            let mut calls = MockProviderPartCalls::new();
            let account = Account::new(
                "example",
                ProviderId::new(id),
                "chatgpt:made-up",
                "you@example.com",
                AccountKind::Subscription,
                vec![FoundIn::profile(
                    AgentId::new("coder"),
                    "/home/user/.coder",
                )],
            );
            calls
                .expect_accounts()
                .times(1)
                .returning(move |_| Ok((vec![account.clone()], Vec::new())));
            PluginProvider::made_up(id, &["coder"], calls)
        };
        let profile = crate::Profile::added(
            AgentId::new("coder"),
            "Coder",
            PathBuf::from("/home/user/.coder"),
        );

        let found = find_accounts(
            &[coder()],
            &[part("one-cloud"), part("other-cloud")],
            &[profile],
            &[],
            std::path::Path::new("/home/user/.local/share/otty/plugins"),
        );

        let providers: Vec<&str> = found
            .accounts()
            .iter()
            .map(|account| account.provider().as_str())
            .collect();
        assert_eq!(providers, ["one-cloud", "other-cloud"]);
    }

    /// A part that failed finds nothing this time; the others' accounts
    /// stay.
    #[test]
    fn a_part_that_failed_finds_nothing() {
        let mut failing = MockProviderPartCalls::new();
        failing.expect_accounts().returning(|_| {
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        });
        let mut working = MockProviderPartCalls::new();
        working
            .expect_accounts()
            .returning(|_| Ok((vec![chatgpt_login("made-up")], Vec::new())));
        let profile = crate::Profile::added(
            AgentId::new("coder"),
            "Coder",
            PathBuf::from("/home/user/.coder"),
        );

        let found = find_accounts(
            &[coder()],
            &[
                PluginProvider::made_up("failing", &["coder"], failing),
                PluginProvider::made_up("openai", &["coder"], working),
            ],
            &[profile],
            &[],
            std::path::Path::new("/home/user/.local/share/otty/plugins"),
        );

        assert_eq!(found.accounts(), [chatgpt_login("made-up")]);
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: an
    /// account is read by the part that found it, once, in its folders.
    #[test]
    fn an_account_is_read_by_the_part_that_found_it() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|account, _, starts_programs, _| {
                account.id == "key:made-up" && !*starts_programs
            })
            .times(1)
            .returning(|_, _, _, _| Ok(four_dollars_twenty()));

        let numbers = read_account_numbers(
            &[example_cloud(calls)],
            &example_key(),
            &[],
            false,
            &Arc::default(),
            no_variables,
        );

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    /// An account no provider part reads is not read: nothing is sent and
    /// no key is looked at.
    #[test]
    fn an_account_no_part_reads_is_not_read() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();

        let numbers = read_account_numbers(
            &[example_cloud(calls)],
            &chatgpt_login("made-up"),
            &[],
            true,
            &Arc::default(),
            |_| panic!("no key is read for an account no part reads"),
        );

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
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
            .withf(|_, given, _, _| {
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
            &[example_cloud(calls)],
            &example_key(),
            &given,
            true,
            &Arc::default(),
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
            &[example_cloud(calls)],
            &example_key(),
            &given,
            true,
            &Arc::default(),
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
