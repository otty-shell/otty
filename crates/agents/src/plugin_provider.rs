//! A provider read by a provider part of a plugin (spec
//! 2026-11-19-agent-plugins, Decisions 4 and 5): for an account of its
//! own provider id or hosts, the part gets a handle to the key at the
//! place an agent part found it, or to a key OTTY read itself (one the
//! user added by hand, or one found in the system keyring), says whose
//! account it is and reads its numbers, once per account per refresh, and
//! the bill of an organization. With each read it gets the keys the user
//! gave for the account after a need of the part asked for them, each
//! under its slot (issue 20), and the profiles the account was found in,
//! where it may ask the agent itself (issue 29). It never sees a key.

use std::sync::Arc;
use std::time::SystemTime;

use otty_plugin_host::agents::agent::{Identity, Login, LoginKind};
use otty_plugin_host::agents::provider::{
    AccountFacts, AccountNumbers as PartNumbers, Bill as PartBill, DayRange,
    FoundProfile, LoginFacts, Money,
};
use otty_plugin_host::{
    CallBatch, CalledPart, KeySource, KeyText, LoadedPlugin, PartHealth,
    PartRef,
};

use crate::account::{Account, AccountKind};
use crate::account_numbers::AccountNumbers;
use crate::balance::{Amount, Balance, KeyLimit};
use crate::bill::Bill;
use crate::error::{SourceError, Unavailable};
use crate::given_key::{GivenSecret, KeySecret};
use crate::limits::{Billing, ExtraUsage, Limits, UsageWindows};
use crate::plugin_bill::BillDays;
use crate::plugin_sources::read_error;
use crate::plugin_values::{credits_of, limits_of};
use crate::provider::{ProviderEntry, ProviderId, ProviderRef};
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
    /// of the part, with the keys of `given` under their slots. The read
    /// starts the programs and hidden terminals the part declares only
    /// when `starts_programs`.
    fn read_numbers(
        &self,
        key: &KeySource,
        found: &LoginFacts,
        given: &[(String, KeySource)],
        starts_programs: bool,
    ) -> Result<(AccountFacts, PartNumbers), SourceError>;

    /// Whose organization the key of `key` is, then its bill over `days`,
    /// in one read of the part, with the keys of `given` under their
    /// slots.
    fn read_bill(
        &self,
        key: &KeySource,
        found: &LoginFacts,
        given: &[(String, KeySource)],
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

    /// How the part's reads went: Plugins mode pauses and starts it
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
        if manifest_part.can_add_by_hand() {
            entry = entry.with_add_by_hand();
        }
        if let Some(name) = manifest_part.priced_as() {
            entry = entry.with_priced_as(name);
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

    /// Whether the part reads `account`: it serves the account, which is
    /// read by the part's plugin (issue 29).
    pub fn reads(&self, account: &Account) -> bool {
        self.serves(account)
            && account.read_by() == Some(self.entry.part().plugin())
    }

    /// The numbers of `account`, read once with the key of `login`, the
    /// login an agent part found it by, the keys the user gave for it
    /// (`given`) and the profiles it was found in (`found_in`): the money
    /// of a key, or the limits of a subscription, which the part may read
    /// any way it can. The part's programs and hidden terminals start only
    /// when `starts_programs`. An account the part does not serve is not
    /// read, and the part gets no key.
    pub(crate) fn read(
        &self,
        account: &Account,
        login: &Login,
        found_in: Vec<FoundProfile>,
        given: &[GivenSecret],
        starts_programs: bool,
    ) -> Result<AccountNumbers, SourceError> {
        let key = KeySource::Place(login.place.clone());
        let found = login_facts(login, found_in);

        self.read_with(account, &key, &found, given, starts_programs)
    }

    /// The numbers of `account`, read once with `key`, a key OTTY read
    /// itself: from the file or the variable the user gave for an account
    /// added by hand, or from the system keyring, and the keys the user
    /// gave for it (`given`). The part gets only a handle to each, for
    /// this read. An account the part does not serve is not read, and the
    /// part gets no key.
    pub(crate) fn read_given(
        &self,
        account: &Account,
        key: &KeySecret,
        given: &[GivenSecret],
        starts_programs: bool,
    ) -> Result<AccountNumbers, SourceError> {
        let key = given_source(key)?;

        self.read_with(account, &key, &key_facts(), given, starts_programs)
    }

    /// The bill up to `now` of the organization `account`, read once with
    /// the key of `login`, the login an agent part found it by in the
    /// profiles `found_in`, and the keys the user gave for it (`given`).
    /// An account the part does not serve is not read, and the part gets
    /// no key.
    pub(crate) fn read_bill(
        &self,
        account: &Account,
        login: &Login,
        found_in: Vec<FoundProfile>,
        given: &[GivenSecret],
        now: SystemTime,
    ) -> Result<Bill, SourceError> {
        let key = KeySource::Place(login.place.clone());
        let found = login_facts(login, found_in);

        self.read_bill_with(account, &key, &found, given, now)
    }

    /// The bill up to `now` of `account`, an account the user added, read
    /// once with its key `key`, which OTTY read from the file or the
    /// variable the user gave, and the keys the user gave for it (`given`).
    pub(crate) fn read_given_bill(
        &self,
        account: &Account,
        key: &KeySecret,
        given: &[GivenSecret],
        now: SystemTime,
    ) -> Result<Bill, SourceError> {
        let key = given_source(key)?;

        self.read_bill_with(account, &key, &key_facts(), given, now)
    }

    /// The bill up to `now` of `account`, read once with the key of
    /// `key`, which `found` tells of, and the keys of `given`. A part that
    /// reads no bill answers "no bill"; one that needs a key asks the
    /// user with its need.
    fn read_bill_with(
        &self,
        account: &Account,
        key: &KeySource,
        found: &LoginFacts,
        given: &[GivenSecret],
        now: SystemTime,
    ) -> Result<Bill, SourceError> {
        if !self.serves(account) {
            return Err(SourceError::NotAvailable(Unavailable::NoBill));
        }

        let given = given_sources(given)?;
        let days = BillDays::up_to(now);
        let read = self.calls.read_bill(key, found, &given, &days.range())?;

        Ok(days.bill_of(&read))
    }

    /// The numbers of `account`, read once with the key of `key`, which
    /// `found` tells of, and the keys of `given`; the part gets no key of
    /// an account it does not serve. What the user must do, such as log
    /// in again, comes from the part as its need. Numbers with nothing in
    /// them are the part's "nothing": limits with no window and no
    /// credits, or a balance with nothing in it.
    fn read_with(
        &self,
        account: &Account,
        key: &KeySource,
        found: &LoginFacts,
        given: &[GivenSecret],
        starts_programs: bool,
    ) -> Result<AccountNumbers, SourceError> {
        if !self.serves(account) {
            return Err(SourceError::NotAvailable(
                Unavailable::NoAccountSource,
            ));
        }

        let given = given_sources(given)?;
        let (facts, numbers) =
            self.calls
                .read_numbers(key, found, &given, starts_programs)?;

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
        let part = otty_plugin_host::PluginPart::new(
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

    /// The same provider offered by Add account.
    pub(crate) fn with_add_by_hand(mut self) -> Self {
        self.entry = self.entry.with_add_by_hand();
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
        given: &[(String, KeySource)],
        starts_programs: bool,
    ) -> Result<(AccountFacts, PartNumbers), SourceError> {
        let read = |batch: &mut CallBatch<'_>| {
            let mut calls = batch.provider()?;

            let facts = match calls.account(key, found)? {
                Ok(facts) => facts,
                Err(error) => return Ok(Err(error)),
            };
            let numbers = calls.numbers(found, &facts, key, given)?;

            Ok(numbers.map(|numbers| (facts, numbers)))
        };
        let answer = match starts_programs {
            true => self.part.read(READ_NUMBERS, read)?,
            false => self.part.read_starting_nothing(READ_NUMBERS, read)?,
        };

        answer.map_err(|error| {
            let not_supported =
                SourceError::NotAvailable(Unavailable::NoAccountSource);

            read_error(&self.part, READ_NUMBERS, &error, not_supported, None)
        })
    }

    fn read_bill(
        &self,
        key: &KeySource,
        found: &LoginFacts,
        given: &[(String, KeySource)],
        days: &DayRange,
    ) -> Result<PartBill, SourceError> {
        let answer = self.part.read(READ_BILL, |batch| {
            let mut calls = batch.provider()?;

            let facts = match calls.account(key, found)? {
                Ok(facts) => facts,
                Err(error) => return Ok(Err(error)),
            };

            calls.read_bill(found, &facts, key, given, days)
        })?;

        answer.map_err(|error| {
            let not_supported = SourceError::NotAvailable(Unavailable::NoBill);

            read_error(&self.part, READ_BILL, &error, not_supported, None)
        })
    }
}

/// A handle source for `key`, a key OTTY read itself; a blank key is
/// gone.
fn given_source(key: &KeySecret) -> Result<KeySource, SourceError> {
    KeyText::new(key.secret())
        .map(KeySource::Given)
        .ok_or(SourceError::NotAvailable(Unavailable::KeyGone))
}

/// A handle source for each key of `given`, with its slot.
fn given_sources(
    given: &[GivenSecret],
) -> Result<Vec<(String, KeySource)>, SourceError> {
    given
        .iter()
        .map(|given| {
            Ok((given.slot().to_string(), given_source(given.secret())?))
        })
        .collect()
}

/// What the agent part found of `login`, in the profiles `found_in`.
fn login_facts(login: &Login, found_in: Vec<FoundProfile>) -> LoginFacts {
    LoginFacts {
        kind: login.kind,
        identity: login.identity.clone(),
        plan: login.plan.clone(),
        found_in,
    }
}

/// What OTTY knows of a key it read itself: an API key, known by the key
/// and found in no profile.
fn key_facts() -> LoginFacts {
    LoginFacts {
        kind: LoginKind::ApiKey,
        identity: no_identity(),
        plan: None,
        found_in: Vec::new(),
    }
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
/// usage it may buy, else its credits, with the plan `plan` and the time
/// the numbers were true.
fn subscription_limits(numbers: &PartNumbers, plan: Option<String>) -> Limits {
    let with_plan = PartNumbers {
        plan,
        ..numbers.clone()
    };
    let limits = limits_of(&with_plan);

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
    let with_extra = Limits::new(
        Billing::UsageWindows(usage),
        limits.plan().map(String::from),
    );
    match limits.recorded_at() {
        Some(time) => with_extra.with_recorded_at(time),
        None => with_extra,
    }
}

/// An amount of money the part read; `None` when it is not a number.
fn amount_of(money: &Money) -> Option<Amount> {
    Amount::new(money.amount, &money.currency)
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agents::agent::{JsonField, KeyPlace};

    use super::*;
    use crate::account::{AccountIdentity, AccountLogin, FoundIn};
    use crate::given_key::{GivenSecret, KeyRef};
    use crate::key_fingerprint::KeyFingerprint;
    use crate::limits::Credits;
    use crate::testing::{NOW, at};

    fn key_login() -> Login {
        Login {
            kind: LoginKind::ApiKey,
            for_whom: otty_plugin_host::agents::agent::ProviderRef::Id(
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
            recorded_at_ms: None,
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
            .returning(|_, _, _, _| Ok((facts(), twelve_dollars_left())));
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers =
            provider.read(&account, &key_login(), Vec::new(), &[], true);

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

        let numbers =
            provider.read(&other, &key_login(), Vec::new(), &[], true);

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
        calls.expect_read_numbers().returning(|_, _, _, _| {
            let facts = AccountFacts {
                identity: no_identity(),
                plan: Some("Team".to_string()),
            };
            Ok((facts, twelve_dollars_left()))
        });
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers =
            provider.read(&account, &key_login(), Vec::new(), &[], true);

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
            KeyRef::EnvVar("DEEPSEEK_API_KEY".to_string()),
        );
        let key = KeySecret::made_up("sk-made-up-deepseek-key-0000");
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|key, found, _, _| {
                let KeySource::Given(given) = key else {
                    return false;
                };
                given.as_str() == "sk-made-up-deepseek-key-0000"
                    && found.kind == LoginKind::ApiKey
            })
            .times(1)
            .returning(|_, _, _, _| Ok((facts(), twelve_dollars_left())));
        let provider =
            PluginProvider::made_up("deepseek", &["deepseek.com"], calls);

        let numbers = provider.read_given(&account, &key, &[], true);

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
            KeyRef::EnvVar("OPENROUTER_API_KEY".to_string()),
        );
        let key = KeySecret::made_up("sk-or-v1-made-up-key-0000");
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let provider =
            PluginProvider::made_up("deepseek", &["deepseek.com"], calls);

        let numbers = provider.read_given(&account, &key, &[], true);

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
            for_whom: otty_plugin_host::agents::agent::ProviderRef::Host(
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
            .withf(move |key, _, _, _| {
                let KeySource::Place(KeyPlace::JsonField(field)) = key else {
                    return false;
                };
                field.file == settings
                    && field.pointer == "/env/ANTHROPIC_AUTH_TOKEN"
            })
            .times(1)
            .returning(|_, _, _, _| Ok((facts(), twelve_dollars_left())));
        let provider =
            PluginProvider::made_up("deepseek", &["deepseek.com"], calls);

        let numbers = provider.read(&account, &login, Vec::new(), &[], true);

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
            recorded_at_ms: None,
        }
    }

    fn usd(amount: f64) -> Money {
        Money {
            amount,
            currency: "USD".to_string(),
        }
    }

    /// Issue 20: what the user must do comes from the part as its need,
    /// the same for a subscription and a key; OTTY writes none of it.
    #[test]
    fn a_need_of_the_part_is_the_answer_of_the_read_as_it_is() {
        let need = || {
            Box::new(crate::fixtures::key_need(
                "example",
                "example-cloud",
                "admin",
            ))
        };
        let asking = || {
            let mut calls = MockProviderPartCalls::new();
            calls.expect_read_numbers().times(1).returning(
                move |_, _, _, _| Err(SourceError::NeedsUser(need())),
            );
            PluginProvider::made_up("example-cloud", &["example.com"], calls)
        };
        let key = key_account(
            ProviderRef::of_id("example-cloud"),
            &["/home/dev/.example-agent"],
        );

        let login = asking().read(
            &subscription("example-cloud"),
            &key_login(),
            Vec::new(),
            &[],
            true,
        );
        let api_key = asking().read(&key, &key_login(), Vec::new(), &[], true);

        assert_eq!(login, Err(SourceError::NeedsUser(need())));
        assert_eq!(api_key, Err(SourceError::NeedsUser(need())));
    }

    /// Issue 20: the keys the user gave for the account reach the part's
    /// read with their slots, each as a handle to the key OTTY read.
    #[test]
    fn the_given_keys_reach_the_read_with_their_slots() {
        let account = key_account(
            ProviderRef::of_id("example-cloud"),
            &["/home/dev/.example-agent"],
        );
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|_, _, given, _| {
                matches!(given, [(slot, KeySource::Given(key))]
                    if slot == "admin" && key.as_str() == "sk-made-up-admin-1")
            })
            .times(1)
            .returning(|_, _, _, _| Ok((facts(), twelve_dollars_left())));
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);
        let given = [GivenSecret::made_up("admin", "sk-made-up-admin-1")];

        let numbers =
            provider.read(&account, &key_login(), Vec::new(), &given, true);

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
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
        calls.expect_read_numbers().returning(|_, _, _, _| {
            let numbers = PartNumbers {
                spent_this_month: Some(usd(4.2)),
                credits: Some(otty_plugin_host::agents::agent::Credits {
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

        let numbers =
            provider.read(&account, &key_login(), Vec::new(), &[], true);

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
        calls.expect_read_numbers().returning(|_, _, _, _| {
            let numbers = PartNumbers {
                extra_usage: Some(
                    otty_plugin_host::agents::provider::ExtraUsage {
                        used: Some(8.3),
                        monthly_limit: Some(50.0),
                        currency: Some("USD".to_string()),
                    },
                ),
                ..nothing()
            };
            Ok((facts(), numbers))
        });
        let provider =
            PluginProvider::made_up("example-cloud", &["example.com"], calls);

        let numbers = provider.read(
            &subscription("example-cloud"),
            &key_login(),
            Vec::new(),
            &[],
            true,
        );

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
        use otty_plugin_host::agents::agent::SqliteField;

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
                for_whom: otty_plugin_host::agents::agent::ProviderRef::Id(
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
                .withf(move |key, found, _, _| {
                    matches!(key, KeySource::Place(sent)
                        if crate::plugin_values::is_same_place(sent, &place))
                        && found.kind == LoginKind::Subscription
                        && found.identity.account_id.as_deref()
                            == Some("made-up-account")
                })
                .times(1)
                .returning(|_, found, _, _| {
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

            let numbers = provider.read(
                &subscription("openai"),
                &login,
                Vec::new(),
                &[],
                true,
            );

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

        let numbers =
            provider.read(&gateway, &key_login(), Vec::new(), &[], true);

        assert!(!provider.serves(&gateway));
        assert_eq!(
            numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
    }

    /// Issue 15 and 20: the key of an account the user added and the keys
    /// given for it reach the part only as handles, once, for the 31 UTC
    /// days up to today; the bill holds each of them.
    #[test]
    fn a_given_key_reads_the_bill_of_the_31_days_up_to_today() {
        let organization = Account::added(
            ProviderRef::of_id("anthropic"),
            KeyRef::EnvVar("ANTHROPIC_API_KEY".to_string()),
        );
        let key = KeySecret::made_up("sk-made-up-key");
        let given = [GivenSecret::made_up("admin", "sk-made-up-org-key")];
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_bill()
            .withf(|key, _, given, days| {
                matches!(key, KeySource::Given(own)
                    if own.as_str() == "sk-made-up-key")
                    && matches!(given, [(slot, KeySource::Given(org))]
                        if slot == "admin"
                            && org.as_str() == "sk-made-up-org-key")
                    && days.first == "2026-10-17"
                    && days.last == "2026-11-16"
            })
            .times(1)
            .returning(|_, _, _, _| {
                Ok(PartBill {
                    days: vec![otty_plugin_host::agents::provider::DayCost {
                        day: "2026-11-16".to_string(),
                        cost: usd(12.4),
                    }],
                })
            });
        let provider =
            PluginProvider::made_up("anthropic", &["anthropic.com"], calls);

        let bill = provider
            .read_given_bill(&organization, &key, &given, at(NOW))
            .expect("the bill");

        assert_eq!(bill.days().len(), 31);
        assert_eq!(bill.today().micro_dollars(), 12_400_000);
    }

    #[test]
    fn a_part_gets_no_key_of_a_bill_of_another_provider() {
        let organization = Account::added(
            ProviderRef::of_id("openai"),
            KeyRef::EnvVar("OPENAI_API_KEY".to_string()),
        );
        let key = KeySecret::made_up("sk-made-up-key");
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_bill().never();
        let provider =
            PluginProvider::made_up("anthropic", &["anthropic.com"], calls);

        let bill = provider.read_given_bill(&organization, &key, &[], at(NOW));

        assert_eq!(bill, Err(SourceError::NotAvailable(Unavailable::NoBill)));
    }
}
