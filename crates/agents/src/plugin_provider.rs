//! A provider read by a provider part of a plugin (spec
//! 2026-11-19-agent-plugins, Decisions 4 and 5): for an account of its
//! own provider id or hosts, the part gets a handle to the key at the
//! place an agent part found it, says whose account it is and reads its
//! numbers, once per account per refresh. It never sees the key.

use std::sync::Arc;

use otty_plugin_host::agent::{KeyPlace, Login};
use otty_plugin_host::provider::{
    AccountFacts, AccountNumbers as PartNumbers, LoginFacts, Money,
};
use otty_plugin_host::{AddByHand, LoadedPlugin, PartRef};

use crate::account::{Account, AccountKind};
use crate::account_numbers::AccountNumbers;
use crate::balance::{Amount, Balance, KeyLimit};
use crate::called_part::CalledPart;
use crate::error::{SourceError, Unavailable};
use crate::limits::Limits;
use crate::part_health::PartHealth;
use crate::plugin_sources::read_error;
use crate::plugin_values::limits_of;
use crate::provider::{AddAccountKind, ProviderEntry, ProviderId, ProviderRef};
use crate::registry::Reader;
use crate::source_id::SourceId;
use crate::walk::Answer;

/// The name of the read of an account's numbers in reports.
const READ_NUMBERS: &str = "numbers";

/// The calls a [`PluginProvider`] makes into its part. A trait so that
/// the tests count the calls a refresh makes without a plugin; the one
/// real implementation calls the part.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait ProviderPartCalls: Send + Sync {
    /// Whose account the key at `place` is, then its numbers, in one read
    /// of the part.
    fn read_numbers(
        &self,
        place: &KeyPlace,
        found: &LoginFacts,
    ) -> Result<(AccountFacts, PartNumbers), SourceError>;
}

/// One provider part of a loaded plugin, with the entry it puts in the
/// registry.
pub struct PluginProvider {
    entry: ProviderEntry,
    health: PartHealth,
    calls: Box<dyn ProviderPartCalls>,
}

impl PluginProvider {
    /// The entry the part puts in the registry.
    pub fn entry(&self) -> &ProviderEntry {
        &self.entry
    }

    /// How the part's reads went: the Plugins page pauses and starts it
    /// with this.
    pub fn health(&self) -> &PartHealth {
        &self.health
    }

    /// The provider part `id` of `plugin`; `None` when the plugin has no
    /// such part.
    pub fn new(plugin: Arc<LoadedPlugin>, id: &str) -> Option<Self> {
        let manifest_part = plugin
            .manifest()
            .providers()
            .iter()
            .find(|part| part.id() == id)?
            .clone();
        let part = Arc::new(CalledPart::new(plugin, PartRef::provider(id)));

        let mut entry = ProviderEntry::new(
            ProviderId::new(id),
            manifest_part.name(),
            Reader::Plugin(part.named().clone()),
        )
        .with_hosts(manifest_part.hosts().iter().map(String::as_str));
        if let Some(page) = manifest_part.usage_page() {
            entry = entry.with_usage_page(page).with_plan_usage_page(page);
        }
        match manifest_part.add_by_hand() {
            Some(AddByHand::Key) => {
                entry = entry.with_add_account(AddAccountKind::Key);
            },
            Some(AddByHand::AdminKey) => {
                entry = entry.with_add_account(AddAccountKind::AdminKey);
            },
            None => {},
        }

        let health = part.health().clone();
        Some(Self::of_calls(entry, health, Box::new(PartCalls { part })))
    }

    /// The same provider with the SVG `icon` from the plugin's package.
    pub fn with_icon(mut self, icon: Vec<u8>) -> Self {
        self.entry = self.entry.with_icon(icon);
        self
    }

    /// Whether the part serves `account`: the account's provider is the
    /// part's id, or a host of the part's hosts. A part gets keys of no
    /// other account.
    pub fn serves(&self, account: &Account) -> bool {
        match account.provider() {
            ProviderRef::Id(id) => id == self.entry.id(),
            ProviderRef::Host(host) => self.entry.serves_host(host),
        }
    }

    /// The numbers of `account`, read once with the key of `login`, the
    /// login an agent part found it by: the money left on a key, or the
    /// limits of a subscription. An account the part does not serve is
    /// not read, and the part gets no key.
    pub fn read(
        &self,
        account: &Account,
        login: &Login,
    ) -> Result<AccountNumbers, SourceError> {
        if !self.serves(account) {
            return Err(SourceError::NotAvailable(
                Unavailable::NoAccountSource,
            ));
        }

        let found = LoginFacts {
            kind: login.kind,
            identity: login.identity.clone(),
            plan: login.plan.clone(),
        };
        let (facts, numbers) = self.calls.read_numbers(&login.place, &found)?;

        let plan = numbers.plan.clone().or(facts.plan);
        Ok(match account.kind() {
            AccountKind::Money => AccountNumbers::Balance(balance_of(&numbers)),
            _ => {
                let limits = subscription_limits(&numbers, plan);
                AccountNumbers::Limits(Answer::first(limits, SourceId::Plugin))
            },
        })
    }

    /// A provider with the entry `entry` whose part is called through
    /// `calls`.
    pub(crate) fn of_calls(
        entry: ProviderEntry,
        health: PartHealth,
        calls: Box<dyn ProviderPartCalls>,
    ) -> Self {
        Self {
            entry,
            health,
            calls,
        }
    }
}

#[cfg(test)]
impl PluginProvider {
    /// The provider `id` serving `hosts`, whose part is called through
    /// `calls`.
    pub(crate) fn made_up(
        id: &str,
        hosts: &[&str],
        calls: MockProviderPartCalls,
    ) -> Self {
        let part = crate::registry::PluginPart::new(
            "example",
            "Example",
            "0.1.0",
            "https://github.com/otty-shell/otty-plugin-example",
            id,
        );
        let entry =
            ProviderEntry::new(ProviderId::new(id), id, Reader::Plugin(part))
                .with_hosts(hosts.iter().copied());

        Self::of_calls(entry, PartHealth::default(), Box::new(calls))
    }
}

/// The real calls of a provider part.
struct PartCalls {
    part: Arc<CalledPart>,
}

impl ProviderPartCalls for PartCalls {
    fn read_numbers(
        &self,
        place: &KeyPlace,
        found: &LoginFacts,
    ) -> Result<(AccountFacts, PartNumbers), SourceError> {
        let answer = self.part.read(READ_NUMBERS, |batch| {
            let mut calls = batch.provider()?;

            let facts = match calls.account(place, found)? {
                Ok(facts) => facts,
                Err(error) => return Ok(Err(error)),
            };
            let numbers = calls.numbers(&facts, place)?;

            Ok(numbers.map(|numbers| (facts, numbers)))
        })?;

        answer.map_err(|error| {
            let not_supported =
                SourceError::NotAvailable(Unavailable::NoAccountSource);

            read_error(&self.part, READ_NUMBERS, &error, not_supported)
        })
    }
}

/// The money left and the key's own limit of `numbers`.
fn balance_of(numbers: &PartNumbers) -> Balance {
    let key_limit = numbers.key_limit.as_ref().and_then(|limit| {
        let used = amount_of(&limit.used)?;
        Some(KeyLimit::new(
            used,
            limit.limit.as_ref().and_then(amount_of),
        ))
    });

    Balance::new(numbers.left.as_ref().and_then(amount_of), key_limit)
}

/// The limits of a subscription in `numbers`: its windows, else its
/// credits, with the plan `plan`.
fn subscription_limits(numbers: &PartNumbers, plan: Option<String>) -> Limits {
    let as_limits = otty_plugin_host::agent::Limits {
        windows: numbers.windows.clone(),
        credits: numbers.credits,
        plan,
        recorded_at_ms: None,
    };

    limits_of(&as_limits)
}

/// An amount of money the part read; `None` when it is not a number.
fn amount_of(money: &Money) -> Option<Amount> {
    Amount::new(money.amount, &money.currency)
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agent::{JsonField, LoginKind};

    use super::*;
    use crate::account::{AccountIdentity, AccountLogin, FoundIn};
    use crate::key_fingerprint::KeyFingerprint;
    use crate::limits::{Billing, UsageWindows};
    use crate::testing::no_identity;

    fn key_login() -> Login {
        Login {
            kind: LoginKind::ApiKey,
            for_whom: otty_plugin_host::agent::ProviderRef::Id(
                "example-cloud".to_string(),
            ),
            place: KeyPlace::JsonField(JsonField {
                file: "/home/dev/.example-agent/auth.json".to_string(),
                pointer: "/api_key".to_string(),
            }),
            identity: no_identity(),
            plan: None,
        }
    }

    fn key_account(provider: ProviderRef, folders: &[&str]) -> Account {
        let mut account = Account::new(
            provider,
            AccountIdentity::KeyFingerprint(KeyFingerprint::of("made-up")),
            AccountLogin::Key(None),
            AccountKind::Money,
            FoundIn::ProfileFolder(folders[0].into()),
        );
        for folder in &folders[1..] {
            let copy = Account::new(
                account.provider().clone(),
                account.identity().clone(),
                AccountLogin::Key(None),
                AccountKind::Money,
                FoundIn::ProfileFolder((*folder).into()),
            );
            account =
                crate::account::join_copies(vec![account, copy]).remove(0);
        }
        account
    }

    fn twelve_dollars_left() -> PartNumbers {
        PartNumbers {
            windows: Vec::new(),
            credits: None,
            left: Some(Money {
                amount: 12.5,
                currency: "USD".to_string(),
            }),
            key_limit: None,
            spent_this_month: None,
            plan: None,
        }
    }

    fn facts() -> AccountFacts {
        AccountFacts {
            identity: no_identity(),
            plan: None,
        }
    }

    /// Spec "How plugins talk to OTTY": an account found in two profiles
    /// is one account, read with one call.
    #[test]
    fn an_account_in_two_profiles_is_read_with_one_call() {
        let account = key_account(
            ProviderRef::of_id("example-cloud"),
            &["/home/dev/.example-agent", "/home/dev/.example-agent-work"],
        );
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .times(1)
            .returning(|_, _| Ok((facts(), twelve_dollars_left())));
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers = provider.read(&account, &key_login());

        assert_eq!(account.found_in().len(), 2);
        assert_eq!(
            numbers,
            Ok(AccountNumbers::Balance(Balance::new(
                Amount::new(12.5, "USD"),
                None
            )))
        );
    }

    /// Spec Decision 7: a provider part gets keys only of accounts of its
    /// own provider id or hosts.
    #[test]
    fn a_part_gets_no_key_of_an_account_of_another_provider() {
        let other = key_account(
            ProviderRef::of_id("deepseek"),
            &["/home/dev/.example-agent"],
        );
        let by_host = key_account(
            ProviderRef::Host("api.example.com".to_string()),
            &["/home/dev/.example-agent"],
        );
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers = provider.read(&other, &key_login());

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
        assert!(!provider.serves(&other));
        assert!(provider.serves(&by_host));
    }

    #[test]
    fn a_subscription_gives_its_limits_with_the_plan() {
        let account = Account::new(
            ProviderRef::of_id("example-cloud"),
            AccountIdentity::AccountId("acct_1".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/dev/.example-agent".into()),
        );
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().returning(|_, _| {
            let facts = AccountFacts {
                identity: no_identity(),
                plan: Some("Team".to_string()),
            };
            Ok((facts, twelve_dollars_left()))
        });
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers = provider.read(&account, &key_login());

        let Ok(AccountNumbers::Limits(limits)) = numbers else {
            panic!("the limits: {numbers:?}");
        };
        assert_eq!(limits.source(), SourceId::Plugin);
        assert_eq!(limits.value().plan(), Some("Team"));
        assert_eq!(
            limits.value().billing(),
            &Billing::UsageWindows(UsageWindows::default())
        );
    }
}
