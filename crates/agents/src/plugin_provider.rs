//! A provider read by a provider part of a plugin (spec
//! 2026-11-19-agent-plugins, Decisions 4 and 5): for an account of its
//! own provider id or hosts, the part gets a handle to the key at the
//! place an agent part found it, or to a key OTTY read itself (one the
//! user added by hand, or one found in the system keyring), says whose
//! account it is and reads its numbers, once per account per refresh; with
//! an admin key it reads the bill of the organization. It never sees the
//! key.

use std::sync::Arc;
use std::time::SystemTime;

use otty_plugin_host::agent::{Identity, Login, LoginKind};
use otty_plugin_host::provider::{
    AccountFacts, AccountNumbers as PartNumbers, Bill as PartBill, DayRange,
    LoginFacts, Money,
};
use otty_plugin_host::{AddByHand, KeySource, KeyText, LoadedPlugin, PartRef};

use crate::account::{Account, AccountKind};
use crate::account_numbers::AccountNumbers;
use crate::admin_key::AdminKey;
use crate::balance::{Amount, Balance, KeyLimit};
use crate::bill::Bill;
use crate::called_part::CalledPart;
use crate::error::{SourceError, Unavailable};
use crate::limits::{Billing, ExtraUsage, Limits, UsageWindows};
use crate::part_health::PartHealth;
use crate::plugin_bill::BillDays;
use crate::plugin_sources::read_error;
use crate::plugin_values::{credits_of, limits_of};
use crate::provider::{AddAccountKind, ProviderEntry, ProviderId, ProviderRef};
use crate::source_id::SourceId;
use crate::walk::Answer;

/// The name of the read of an account's numbers in reports.
const READ_NUMBERS: &str = "numbers";
/// The name of the read of a bill in reports.
const READ_BILL: &str = "read-bill";

/// The calls a [`PluginProvider`] makes into its part. A trait so that
/// the tests count the calls a refresh makes without a plugin; the one
/// real implementation calls the part.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait ProviderPartCalls: Send + Sync {
    /// Whose account the key of `key` is, then its numbers, in one read
    /// of the part.
    fn read_numbers(
        &self,
        key: &KeySource,
        found: &LoginFacts,
    ) -> Result<(AccountFacts, PartNumbers), SourceError>;

    /// Whose organization the admin key of `admin_key` is, then its bill
    /// over `days`, in one read of the part.
    fn read_bill(
        &self,
        admin_key: &KeySource,
        days: &DayRange,
    ) -> Result<PartBill, SourceError>;
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

        let keyring_services = manifest_part
            .permissions()
            .keyring()
            .iter()
            .map(|rule| rule.service().to_string())
            .collect();
        let mut entry = ProviderEntry::new(
            ProviderId::new(id),
            manifest_part.name(),
            part.named().clone(),
        )
        .with_hosts(manifest_part.hosts().iter().map(String::as_str))
        .with_keyring_services(keyring_services);
        if let Some(page) = manifest_part.usage_page() {
            entry = entry.with_usage_page(page);
        }
        if let Some(page) = manifest_part
            .plan_usage_page()
            .or(manifest_part.usage_page())
        {
            entry = entry.with_plan_usage_page(page);
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
        self.serves_provider(account.provider())
    }

    /// Whether the part serves the provider `provider`: the part's id, or
    /// a host of the part's hosts.
    pub fn serves_provider(&self, provider: &ProviderRef) -> bool {
        match provider {
            ProviderRef::Id(id) => id == self.entry.id(),
            ProviderRef::Host(host) => self.entry.serves_host(host),
        }
    }

    /// The numbers of `account`, read once with the key of `login`, the
    /// login an agent part found it by: the money of a key, or the limits
    /// of a subscription. An account the part does not serve is not read,
    /// and the part gets no key.
    pub fn read(
        &self,
        account: &Account,
        login: &Login,
    ) -> Result<AccountNumbers, SourceError> {
        let key = KeySource::Place(login.place.clone());
        let found = LoginFacts {
            kind: login.kind,
            identity: login.identity.clone(),
            plan: login.plan.clone(),
        };

        self.read_with(account, &key, &found)
    }

    /// The numbers of `account`, read once with `key`, a key OTTY read
    /// itself: from the file or the variable the user gave for an account
    /// added by hand, or from the system keyring. The part gets only a
    /// handle to it, for this read. An account the part does not serve is
    /// not read, and the part gets no key.
    pub(crate) fn read_given(
        &self,
        account: &Account,
        key: &AdminKey,
    ) -> Result<AccountNumbers, SourceError> {
        let key = given(key)?;
        let found = LoginFacts {
            kind: LoginKind::ApiKey,
            identity: no_identity(),
            plan: None,
        };

        self.read_with(account, &key, &found)
    }

    /// The bill up to `now` of an organization at `provider`, read once
    /// with its admin key `admin_key`, which OTTY read from the file or
    /// the variable the user gave: the part gets only a handle to it, for
    /// this read. A provider the part does not serve is not read, and the
    /// part gets no key.
    pub(crate) fn read_bill(
        &self,
        provider: &ProviderRef,
        admin_key: &AdminKey,
        now: SystemTime,
    ) -> Result<Bill, SourceError> {
        if !self.serves_provider(provider) {
            return Err(SourceError::NotAvailable(Unavailable::NoBill));
        }

        let key = given(admin_key)?;
        let days = BillDays::up_to(now);
        let read = self.calls.read_bill(&key, &days.range())?;

        Ok(days.bill_of(&read))
    }

    /// The numbers of `account`, read once with the key of `key`, which
    /// `found` tells of; the part gets no key of an account it does not
    /// serve. A subscription whose login the provider refused is not
    /// logged in: the login expired or may not read the numbers, and OTTY
    /// renews no login.
    fn read_with(
        &self,
        account: &Account,
        key: &KeySource,
        found: &LoginFacts,
    ) -> Result<AccountNumbers, SourceError> {
        if !self.serves(account) {
            return Err(SourceError::NotAvailable(
                Unavailable::NoAccountSource,
            ));
        }

        let read = self.calls.read_numbers(key, found);
        let (facts, numbers) = match (read, account.kind()) {
            (Err(SourceError::KeyRefused), AccountKind::Money) => {
                return Err(SourceError::KeyRefused);
            },
            (Err(SourceError::KeyRefused), _) => {
                return Err(SourceError::NotLoggedIn);
            },
            (read, _) => read?,
        };

        let plan = numbers.plan.clone().or(facts.plan);
        Ok(match account.kind() {
            AccountKind::Money => AccountNumbers::Balance(balance_of(&numbers)),
            _ => {
                let limits = subscription_limits(&numbers, plan);
                AccountNumbers::Limits(Answer::first(
                    limits,
                    SourceId::ProviderPart,
                ))
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
        let entry = ProviderEntry::new(ProviderId::new(id), id, part)
            .with_hosts(hosts.iter().copied());

        Self::of_calls(entry, PartHealth::default(), Box::new(calls))
    }

    /// The same provider offering Add account with `kind`.
    pub(crate) fn with_add_account(mut self, kind: AddAccountKind) -> Self {
        self.entry = self.entry.with_add_account(kind);
        self
    }
}

/// The real calls of a provider part.
struct PartCalls {
    part: Arc<CalledPart>,
}

impl ProviderPartCalls for PartCalls {
    fn read_numbers(
        &self,
        key: &KeySource,
        found: &LoginFacts,
    ) -> Result<(AccountFacts, PartNumbers), SourceError> {
        let answer = self.part.read(READ_NUMBERS, |batch| {
            let mut calls = batch.provider()?;

            let facts = match calls.account(key, found)? {
                Ok(facts) => facts,
                Err(error) => return Ok(Err(error)),
            };
            let numbers = calls.numbers(&facts, key)?;

            Ok(numbers.map(|numbers| (facts, numbers)))
        })?;

        answer.map_err(|error| {
            let not_supported =
                SourceError::NotAvailable(Unavailable::NoAccountSource);

            read_error(&self.part, READ_NUMBERS, &error, not_supported)
        })
    }

    fn read_bill(
        &self,
        admin_key: &KeySource,
        days: &DayRange,
    ) -> Result<PartBill, SourceError> {
        let found = LoginFacts {
            kind: LoginKind::AdminKey,
            identity: no_identity(),
            plan: None,
        };

        let answer = self.part.read(READ_BILL, |batch| {
            let mut calls = batch.provider()?;

            let facts = match calls.account(admin_key, &found)? {
                Ok(facts) => facts,
                Err(error) => return Ok(Err(error)),
            };

            calls.read_bill(&facts, admin_key, days)
        })?;

        answer.map_err(|error| {
            let not_supported = SourceError::NotAvailable(Unavailable::NoBill);

            read_error(&self.part, READ_BILL, &error, not_supported)
        })
    }
}

/// A handle source for `key`, a key OTTY read itself; a blank key is
/// gone.
fn given(key: &AdminKey) -> Result<KeySource, SourceError> {
    KeyText::new(key.secret())
        .map(KeySource::Given)
        .ok_or(SourceError::NotAvailable(Unavailable::KeyGone))
}

/// An identity that names nothing: OTTY knows the account by the key.
fn no_identity() -> Identity {
    Identity {
        account_id: None,
        organization_id: None,
        email: None,
    }
}

/// What a key told in `numbers`: the money left, the key's own limit, the
/// money spent this month and the credits.
fn balance_of(numbers: &PartNumbers) -> Balance {
    let key_limit = numbers.key_limit.as_ref().and_then(|limit| {
        let used = amount_of(&limit.used)?;
        Some(KeyLimit::new(
            used,
            limit.limit.as_ref().and_then(amount_of),
        ))
    });

    Balance::new(numbers.left.as_ref().and_then(amount_of), key_limit)
        .with_spent_this_month(
            numbers.spent_this_month.as_ref().and_then(amount_of),
        )
        .with_credits(numbers.credits.as_ref().map(credits_of))
}

/// The limits of a subscription in `numbers`: its windows with the extra
/// usage it may buy, else its credits, with the plan `plan`.
fn subscription_limits(numbers: &PartNumbers, plan: Option<String>) -> Limits {
    let as_limits = otty_plugin_host::agent::Limits {
        windows: numbers.windows.clone(),
        credits: numbers.credits,
        plan,
        recorded_at_ms: None,
    };
    let limits = limits_of(&as_limits);

    let (Some(extra), Billing::UsageWindows(usage)) =
        (&numbers.extra_usage, limits.billing())
    else {
        return limits;
    };
    let extra = ExtraUsage::new(
        extra.used,
        extra.monthly_limit,
        extra.currency.clone(),
    );
    let usage = UsageWindows::new(usage.windows().to_vec(), Some(extra));
    Limits::new(
        Billing::UsageWindows(usage),
        limits.plan().map(String::from),
    )
}

/// An amount of money the part read; `None` when it is not a number.
fn amount_of(money: &Money) -> Option<Amount> {
    Amount::new(money.amount, &money.currency)
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agent::{JsonField, KeyPlace};

    use super::*;
    use crate::account::{AccountIdentity, AccountLogin, FoundIn};
    use crate::admin_key::AdminKeyRef;
    use crate::key_fingerprint::KeyFingerprint;
    use crate::limits::Credits;
    use crate::testing::{NOW, at};

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
            extra_usage: None,
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
        assert_eq!(limits.source(), SourceId::ProviderPart);
        assert_eq!(limits.value().plan(), Some("Team"));
        assert_eq!(
            limits.value().billing(),
            &Billing::UsageWindows(UsageWindows::default())
        );
    }

    /// Spec 2026-11-19-agent-plugins, Decision 5: a key the user added by
    /// hand reaches the part only as a handle to the key OTTY read.
    #[test]
    fn an_added_key_reaches_the_part_as_a_given_key() {
        let account = Account::added(
            ProviderRef::of_id("deepseek"),
            AdminKeyRef::EnvVar("DEEPSEEK_API_KEY".to_string()),
        );
        let key = AdminKey::made_up("sk-made-up-deepseek-key-0000");
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|key, found| {
                let KeySource::Given(given) = key else {
                    return false;
                };
                given.as_str() == "sk-made-up-deepseek-key-0000"
                    && found.kind == LoginKind::ApiKey
            })
            .times(1)
            .returning(|_, _| Ok((facts(), twelve_dollars_left())));
        let provider =
            PluginProvider::made_up("deepseek", &["deepseek.com"], calls);

        let numbers = provider.read_given(&account, &key);

        assert_eq!(
            numbers,
            Ok(AccountNumbers::Balance(Balance::new(
                Amount::new(12.5, "USD"),
                None
            )))
        );
    }

    #[test]
    fn a_part_gets_no_added_key_of_another_provider() {
        let account = Account::added(
            ProviderRef::of_id("openrouter"),
            AdminKeyRef::EnvVar("OPENROUTER_API_KEY".to_string()),
        );
        let key = AdminKey::made_up("sk-or-v1-made-up-key-0000");
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let provider =
            PluginProvider::made_up("deepseek", &["deepseek.com"], calls);

        let numbers = provider.read_given(&account, &key);

        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
    }

    /// Issue 16: a DeepSeek key Claude Code sends to
    /// `ANTHROPIC_BASE_URL=https://api.deepseek.com/anthropic` is a key for
    /// the host `api.deepseek.com`, which the DeepSeek part serves; it gets
    /// a handle to the key's place in `settings.json`.
    #[test]
    fn a_deepseek_key_behind_claude_codes_base_url_is_read_by_the_deepseek_part()
     {
        let settings = "/home/dev/.claude-deepseek/settings.json";
        let account = key_account(
            ProviderRef::Host("api.deepseek.com".to_string()),
            &["/home/dev/.claude-deepseek"],
        );
        let login = Login {
            kind: LoginKind::ApiKey,
            for_whom: otty_plugin_host::agent::ProviderRef::Host(
                "api.deepseek.com".to_string(),
            ),
            place: KeyPlace::JsonField(JsonField {
                file: settings.to_string(),
                pointer: "/env/ANTHROPIC_AUTH_TOKEN".to_string(),
            }),
            identity: no_identity(),
            plan: None,
        };
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(move |key, _| {
                let KeySource::Place(KeyPlace::JsonField(field)) = key else {
                    return false;
                };
                field.file == settings
                    && field.pointer == "/env/ANTHROPIC_AUTH_TOKEN"
            })
            .times(1)
            .returning(|_, _| Ok((facts(), twelve_dollars_left())));
        let provider =
            PluginProvider::made_up("deepseek", &["deepseek.com"], calls);

        let numbers = provider.read(&account, &login);

        assert!(provider.serves(&account));
        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    fn subscription(provider: &str) -> Account {
        Account::new(
            ProviderRef::of_id(provider),
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/dev/.example-agent".into()),
        )
    }

    fn nothing() -> PartNumbers {
        PartNumbers {
            windows: Vec::new(),
            credits: None,
            extra_usage: None,
            left: None,
            key_limit: None,
            spent_this_month: None,
            plan: None,
        }
    }

    fn usd(amount: f64) -> Money {
        Money {
            amount,
            currency: "USD".to_string(),
        }
    }

    /// Issue 12 of spec 2026-11-19-agent-plugins: a part tries a login;
    /// one the provider refused is "not logged in" for a subscription and
    /// "key refused" for a key.
    #[test]
    fn a_refused_login_is_not_logged_in_and_a_refused_key_refused() {
        let refused = || {
            let mut calls = MockProviderPartCalls::new();
            calls
                .expect_read_numbers()
                .times(1)
                .returning(|_, _| Err(SourceError::KeyRefused));
            PluginProvider::made_up("example-cloud", &["example.com"], calls)
        };
        let key = key_account(
            ProviderRef::of_id("example-cloud"),
            &["/home/dev/.example-agent"],
        );

        let login =
            refused().read(&subscription("example-cloud"), &key_login());
        let api_key = refused().read(&key, &key_login());

        assert_eq!(login, Err(SourceError::NotLoggedIn));
        assert_eq!(api_key, Err(SourceError::KeyRefused));
    }

    /// Issue 15: a key account shows the money spent this month and the
    /// credits its provider tells, next to the money left.
    #[test]
    fn a_key_gives_its_money_spent_this_month_and_its_credits() {
        let account = key_account(
            ProviderRef::of_id("example-cloud"),
            &["/home/dev/.example-agent"],
        );
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().returning(|_, _| {
            let numbers = PartNumbers {
                spent_this_month: Some(usd(4.2)),
                credits: Some(otty_plugin_host::agent::Credits {
                    left: Some(80.0),
                    used: Some(20.0),
                    pool: Some(100.0),
                    renews_at_ms: None,
                    is_unlimited: false,
                }),
                ..nothing()
            };
            Ok((facts(), numbers))
        });
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers = provider.read(&account, &key_login());

        let Ok(AccountNumbers::Balance(balance)) = numbers else {
            panic!("a balance: {numbers:?}");
        };
        assert_eq!(balance.left(), None);
        assert_eq!(
            balance.spent_this_month(),
            Amount::new(4.2, "USD").as_ref()
        );
        assert_eq!(
            balance.credits(),
            Some(&Credits::new(Some(80.0), Some(20.0), Some(100.0), None))
        );
    }

    /// The extra usage of a Claude login is kept next to its windows.
    #[test]
    fn a_subscription_keeps_the_extra_usage_next_to_its_windows() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().returning(|_, _| {
            let numbers = PartNumbers {
                extra_usage: Some(otty_plugin_host::provider::ExtraUsage {
                    used: Some(8.3),
                    monthly_limit: Some(50.0),
                    currency: Some("USD".to_string()),
                }),
                ..nothing()
            };
            Ok((facts(), numbers))
        });
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers =
            provider.read(&subscription("example-cloud"), &key_login());

        let Ok(AccountNumbers::Limits(limits)) = numbers else {
            panic!("the limits: {numbers:?}");
        };
        let Billing::UsageWindows(usage) = limits.value().billing() else {
            panic!("usage windows: {limits:?}");
        };
        assert_eq!(
            usage.extra_usage(),
            Some(&ExtraUsage::new(
                Some(8.3),
                Some(50.0),
                Some("USD".to_string())
            ))
        );
    }

    /// Issue 14 and 15: a ChatGPT login found only in OpenCode is read by
    /// the provider part once, with a handle to the place OpenCode keeps
    /// its token: a field of `auth.json`, or a field of the `credential`
    /// row of its database.
    #[test]
    fn a_chatgpt_login_found_in_opencode_is_read_once_with_its_place() {
        use otty_plugin_host::agent::SqliteField;

        let auth = KeyPlace::JsonField(JsonField {
            file: "/home/dev/.local/share/opencode/auth.json".to_string(),
            pointer: "/openai/access".to_string(),
        });
        let row = KeyPlace::SqliteField(SqliteField {
            database: "/home/dev/.local/share/opencode/opencode.db".to_string(),
            table: "credential".to_string(),
            column: "value".to_string(),
            row_id: 3,
            pointer: Some("/access".to_string()),
        });

        for place in [auth, row] {
            let login = Login {
                kind: LoginKind::Subscription,
                for_whom: otty_plugin_host::agent::ProviderRef::Id(
                    "openai".to_string(),
                ),
                place: place.clone(),
                identity: Identity {
                    account_id: Some("made-up-account".to_string()),
                    organization_id: None,
                    email: Some("dev@example.com".to_string()),
                },
                plan: Some("Plus".to_string()),
            };
            let mut calls = MockProviderPartCalls::new();
            calls
                .expect_read_numbers()
                .withf(move |key, found| {
                    matches!(key, KeySource::Place(sent)
                        if crate::plugin_values::is_same_place(sent, &place))
                        && found.kind == LoginKind::Subscription
                        && found.identity.account_id.as_deref()
                            == Some("made-up-account")
                })
                .times(1)
                .returning(|_, found| {
                    let facts = AccountFacts {
                        identity: found.identity.clone(),
                        plan: found.plan.clone(),
                    };
                    Ok((facts, nothing()))
                });
            let provider = PluginProvider::made_up(
                "openai",
                &["openai.com", "chatgpt.com"],
                calls,
            );

            let numbers = provider.read(&subscription("openai"), &login);

            let Ok(AccountNumbers::Limits(limits)) = numbers else {
                panic!("the limits: {numbers:?}");
            };
            assert_eq!(limits.value().plan(), Some("Plus"));
        }
    }

    /// Issue 11: a ChatGPT login behind a gateway is for the gateway's
    /// host; the `openai` part does not serve it, so its token never goes
    /// to ChatGPT.
    #[test]
    fn a_login_behind_a_gateway_is_not_read_by_the_openai_part() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let provider = PluginProvider::made_up(
            "openai",
            &["openai.com", "chatgpt.com"],
            calls,
        );
        let gateway = Account::new(
            ProviderRef::Host("gateway.example.com".to_string()),
            AccountIdentity::AccountId("made-up-account".into()),
            AccountLogin::Unnamed,
            AccountKind::Limits,
            FoundIn::ProfileFolder("/home/dev/.codex-gateway".into()),
        );

        let numbers = provider.read(&gateway, &key_login());

        assert!(!provider.serves(&gateway));
        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
    }

    /// Issue 15: the admin key of an organization reaches the part only
    /// as a handle, once, for the 31 UTC days up to today; the bill holds
    /// each of them.
    #[test]
    fn an_admin_key_reads_the_bill_of_the_31_days_up_to_today() {
        let organization = Account::added(
            ProviderRef::of_id("anthropic"),
            AdminKeyRef::EnvVar("ANTHROPIC_ADMIN_KEY".to_string()),
        );
        let key = AdminKey::made_up("sk-ant-admin01-made-up-key");
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_bill()
            .withf(|key, days| {
                matches!(key, KeySource::Given(given)
                    if given.as_str() == "sk-ant-admin01-made-up-key")
                    && days.first == "2026-10-17"
                    && days.last == "2026-11-16"
            })
            .times(1)
            .returning(|_, _| {
                Ok(PartBill {
                    days: vec![otty_plugin_host::provider::DayCost {
                        day: "2026-11-16".to_string(),
                        cost: usd(12.4),
                    }],
                })
            });
        let provider =
            PluginProvider::made_up("anthropic", &["anthropic.com"], calls);

        let bill = provider
            .read_bill(organization.provider(), &key, at(NOW))
            .expect("the bill");

        assert_eq!(bill.days().len(), 31);
        assert_eq!(bill.today().micro_dollars(), 12_400_000);
    }

    #[test]
    fn a_part_gets_no_admin_key_of_another_provider() {
        let organization = Account::added(
            ProviderRef::of_id("openai"),
            AdminKeyRef::EnvVar("OPENAI_ADMIN_KEY".to_string()),
        );
        let key = AdminKey::made_up("sk-admin-made-up-key");
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_bill().never();
        let provider =
            PluginProvider::made_up("anthropic", &["anthropic.com"], calls);

        let bill = provider.read_bill(organization.provider(), &key, at(NOW));

        assert_eq!(bill, Err(SourceError::NotAvailable(Unavailable::NoBill)));
    }
}
