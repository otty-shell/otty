//! A provider read by a provider part of a plugin (spec
//! 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8): the part
//! finds its accounts in the folders of the shown profiles of the agent
//! parts its manifest names in `logins`, and reads the numbers and the
//! bill of each of them in the folders it was found in. With each read it
//! gets the keys the user gave for the account after a need of the part
//! asked for them, each under its slot (spec 2026-11-19-agent-plugins,
//! issue 20). It never sees a key and never the profiles of another
//! plugin.

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use otty_plugin_host::agents::provider::{
    Account as PartAccount, AccountKind as PartKind,
    AccountNumbers as PartNumbers, Bill as PartBill, DayRange, LoginFolder,
    Money,
};
use otty_plugin_host::{
    CallBatch, CalledPart, KeySource, KeyText, LoadedPlugin, PartHealth,
    PartRef,
};

use crate::account::{Account, AccountKind, FoundIn};
use crate::account_numbers::AccountNumbers;
use crate::agent::{Agent, AgentId, agent_of};
use crate::balance::{Amount, Balance, KeyLimit};
use crate::bill::Bill;
use crate::command::AgentCommand;
use crate::error::{SourceError, Unavailable};
use crate::given_key::{GivenSecret, KeySecret};
use crate::limits::{Billing, ExtraUsage, Limits, UsageWindows};
use crate::own_login::OwnLogin;
use crate::part_need::{PartNeed, started_command};
use crate::plugin_bill::BillDays;
use crate::plugin_sources::read_error;
use crate::plugin_values::{credits_of, limits_of};
use crate::profile::Profile;
use crate::provider::{LogInWith, ProviderEntry, ProviderId};
use crate::source_id::SourceId;
use crate::walk::Answer;

/// The name of the read of a part's accounts in reports.
const READ_ACCOUNTS: &str = "accounts";
/// The name of the read of an account's numbers in reports.
const READ_NUMBERS: &str = "numbers";
/// The name of the read of a bill in reports.
const READ_BILL: &str = "read-bill";
/// The name of the call for the login command of an own login in
/// reports.
const READ_LOG_IN: &str = "log-in";

/// The calls a [`PluginProvider`] makes into its part. A trait so that
/// the tests count the calls a refresh makes without a plugin; the one
/// real implementation calls the part.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait ProviderPartCalls: Send + Sync {
    /// The accounts the part finds in `folders`, and its needs that belong
    /// to no account, as OTTY shows them.
    fn accounts(
        &self,
        folders: &[LoginFolder],
    ) -> Result<(Vec<Account>, Vec<PartNeed>), SourceError>;

    /// The numbers of `account`, read in its folders with the keys of
    /// `given` under their slots. The read starts the programs and hidden
    /// terminals the part declares only when `starts_programs`.
    fn read_numbers(
        &self,
        account: &PartAccount,
        given: &[(String, KeySource)],
        starts_programs: bool,
    ) -> Result<PartNumbers, SourceError>;

    /// The bill of the organization `account` over `days`, read in its
    /// folders with the keys of `given` under their slots.
    fn read_bill(
        &self,
        account: &PartAccount,
        given: &[(String, KeySource)],
        days: &DayRange,
    ) -> Result<PartBill, SourceError>;

    /// The agent's official login command pointed at OTTY's own login
    /// folder `folder`, as the app runs it; `None` when the part gives
    /// none.
    fn log_in(
        &self,
        folder: &Path,
    ) -> Result<Option<AgentCommand>, SourceError>;
}

/// One provider part of a loaded plugin, with the entry it puts in the
/// registry.
pub struct PluginProvider {
    entry: ProviderEntry,
    health: PartHealth,
    login_agents: Vec<AgentId>,
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
        if let Some(name) = manifest_part.priced_as() {
            entry = entry.with_priced_as(name);
        }
        if let Some(rule) = manifest_part.log_in() {
            let agent = AgentId::new(rule.agent());
            entry = entry.with_log_in(LogInWith::new(agent, rule.agent_name()));
        }
        let login_agents = manifest_part
            .logins()
            .iter()
            .map(|rule| AgentId::new(rule.agent()))
            .collect();

        let health = part.health().clone();
        let calls = Box::new(PartCalls { part });
        Some(Self::of_calls(entry, health, calls).with_logins_of(login_agents))
    }

    /// The same provider with the SVG `icon` from the plugin's package.
    pub fn with_icon(mut self, icon: Vec<u8>) -> Self {
        self.entry = self.entry.with_icon(icon);
        self
    }

    /// Whether the part reads `account`: it found it, so the account is
    /// of the part's provider id and of the part's plugin. A part reads no
    /// other account (spec 2026-11-23-accounts-apart-from-profiles,
    /// Decision 1).
    pub fn reads(&self, account: &Account) -> bool {
        account.provider() == self.entry.id()
            && account.plugin() == self.entry.part().plugin()
    }

    /// Whether the part made the own login `login`: only it reads that
    /// folder.
    pub fn made(&self, login: &OwnLogin) -> bool {
        login.is_of(self.entry.part().plugin(), self.entry.id())
    }

    /// The accounts the part finds in the folders of `profiles` it reads:
    /// the profiles of the agent parts its manifest names in `logins`, of
    /// its own plugin, as `agents` tell; and in the folders in
    /// `plugins_folder` of the own logins of `own_logins` it made (spec
    /// Decision 6). With
    /// them come its needs that belong to no account. A part with no such
    /// folder is not called and finds none (spec Decision 11).
    pub(crate) fn accounts(
        &self,
        agents: &[Box<dyn Agent>],
        profiles: &[Profile],
        own_logins: &[OwnLogin],
        plugins_folder: &Path,
    ) -> Result<(Vec<Account>, Vec<PartNeed>), SourceError> {
        let profile_folders = profiles
            .iter()
            .filter(|profile| self.reads_logins_of(agents, profile))
            .map(|profile| {
                LoginFolder::Profile((
                    profile.agent().as_str().to_string(),
                    profile.config_dir().display().to_string(),
                ))
            });
        let own_folders = own_logins
            .iter()
            .filter(|login| self.made(login))
            .map(|login| {
                LoginFolder::OwnLogin((
                    login.id().to_string(),
                    login.folder_in(plugins_folder).display().to_string(),
                ))
            });
        let folders: Vec<LoginFolder> =
            profile_folders.chain(own_folders).collect();

        if folders.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        self.calls.accounts(&folders)
    }

    /// The numbers of `account`, read once by the part in the folders it
    /// was found in, with the keys the user gave for it (`given`): the
    /// money of a key or an organization, or the limits of a subscription,
    /// which the part may read any way it can. The part's programs and
    /// hidden terminals start only when `starts_programs`. An account the
    /// part does not read is not read.
    ///
    /// What the user must do, such as log in again, comes from the part
    /// as its need. Numbers with nothing in them are the part's
    /// "nothing": limits with no window and no credits, or a balance with
    /// nothing in it.
    pub(crate) fn read(
        &self,
        account: &Account,
        given: &[GivenSecret],
        starts_programs: bool,
    ) -> Result<AccountNumbers, SourceError> {
        if !self.reads(account) {
            return Err(SourceError::NotAvailable(
                Unavailable::NoAccountSource,
            ));
        }

        let given = given_sources(given)?;
        let numbers = self.calls.read_numbers(
            &part_account(account),
            &given,
            starts_programs,
        )?;

        Ok(match account.kind() {
            AccountKind::Subscription => {
                let plan = numbers
                    .plan
                    .clone()
                    .or_else(|| account.plan().map(String::from));
                let limits = subscription_limits(&numbers, plan);
                AccountNumbers::Limits(Answer::first(
                    limits,
                    SourceId::ProviderPart,
                ))
            },
            _ => AccountNumbers::Balance(balance_of(&numbers)),
        })
    }

    /// The bill up to `now` of the organization `account`, read once by
    /// the part in the folders it was found in, with the keys the user
    /// gave for it (`given`). A part that reads no bill answers "no bill";
    /// one that needs a key asks the user with its need. An account the
    /// part does not read is not read.
    pub(crate) fn read_bill(
        &self,
        account: &Account,
        given: &[GivenSecret],
        now: SystemTime,
    ) -> Result<Bill, SourceError> {
        if !self.reads(account) {
            return Err(SourceError::NotAvailable(Unavailable::NoBill));
        }

        let given = given_sources(given)?;
        let days = BillDays::up_to(now);
        let read = self.calls.read_bill(
            &part_account(account),
            &given,
            &days.range(),
        )?;

        Ok(days.bill_of(&read))
    }

    /// The agent's official login command the part gives for OTTY's own
    /// login folder `folder` (spec Decision 6); `None` from a part that
    /// offers no "Log in with".
    pub(crate) fn log_in(
        &self,
        folder: &Path,
    ) -> Result<Option<AgentCommand>, SourceError> {
        self.calls.log_in(folder)
    }

    /// A provider with the entry `entry` whose part is called through
    /// `calls`, reading the logins of no agent part.
    pub(crate) fn of_calls(
        entry: ProviderEntry,
        health: PartHealth,
        calls: Box<dyn ProviderPartCalls>,
    ) -> Self {
        Self {
            entry,
            health,
            login_agents: Vec::new(),
            calls,
        }
    }

    /// The same provider reading the logins in the profiles of `agents`,
    /// agent parts of its own plugin.
    pub(crate) fn with_logins_of(mut self, agents: Vec<AgentId>) -> Self {
        self.login_agents = agents;
        self
    }

    /// Whether the part reads the logins of `profile`: a profile of an
    /// agent part its manifest names, which the plugin of the part serves.
    fn reads_logins_of(
        &self,
        agents: &[Box<dyn Agent>],
        profile: &Profile,
    ) -> bool {
        let is_named = self.login_agents.contains(profile.agent());
        let is_of_plugin =
            agent_of(agents, profile.agent()).is_some_and(|agent| {
                agent.entry().part().plugin() == self.entry.part().plugin()
            });

        is_named && is_of_plugin
    }
}

#[cfg(test)]
impl PluginProvider {
    /// The provider `id` of the plugin `example`, which reads the logins
    /// in the profiles of `logins_of`, whose part is called through
    /// `calls`.
    pub(crate) fn made_up(
        id: &str,
        logins_of: &[&str],
        calls: MockProviderPartCalls,
    ) -> Self {
        let part = otty_plugin_host::PluginPart::new(
            "example",
            "Example",
            "0.1.0",
            "https://github.com/otty-shell/otty-plugin-example",
            id,
        );
        let entry = ProviderEntry::new(ProviderId::new(id), id, part);
        let agents = logins_of.iter().copied().map(AgentId::new).collect();

        Self::of_calls(entry, PartHealth::default(), Box::new(calls))
            .with_logins_of(agents)
    }
}

/// The real calls of a provider part.
struct PartCalls {
    part: Arc<CalledPart>,
}

impl ProviderPartCalls for PartCalls {
    fn accounts(
        &self,
        folders: &[LoginFolder],
    ) -> Result<(Vec<Account>, Vec<PartNeed>), SourceError> {
        let found = self
            .part
            .read(READ_ACCOUNTS, |batch| batch.provider()?.accounts(folders))?;

        let need_of = |need| PartNeed::of_called_part(&self.part, need);
        let part = self.part.named();
        let accounts = found
            .accounts
            .iter()
            .map(|account| {
                let needs = account.needs.iter().filter_map(need_of).collect();
                account_of(part.plugin(), part.part(), account, needs)
            })
            .collect();
        let needs = found.needs.iter().filter_map(need_of).collect();
        Ok((accounts, needs))
    }

    fn read_numbers(
        &self,
        account: &PartAccount,
        given: &[(String, KeySource)],
        starts_programs: bool,
    ) -> Result<PartNumbers, SourceError> {
        let read = |batch: &mut CallBatch<'_>| {
            batch.provider()?.numbers(account, given)
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
        account: &PartAccount,
        given: &[(String, KeySource)],
        days: &DayRange,
    ) -> Result<PartBill, SourceError> {
        let answer = self.part.read(READ_BILL, |batch| {
            batch.provider()?.read_bill(account, given, days)
        })?;

        answer.map_err(|error| {
            let not_supported = SourceError::NotAvailable(Unavailable::NoBill);

            read_error(&self.part, READ_BILL, &error, not_supported, None)
        })
    }

    fn log_in(
        &self,
        folder: &Path,
    ) -> Result<Option<AgentCommand>, SourceError> {
        let folder = folder.display().to_string();

        let command = self
            .part
            .read(READ_LOG_IN, |batch| batch.provider()?.log_in(&folder))?;

        Ok(
            command
                .map(|command| started_command(self.part.plugin(), &command)),
        )
    }
}

/// The account `account` the provider part `provider` of the plugin
/// `plugin` found, with the needs it gave for it as OTTY shows them.
fn account_of(
    plugin: &str,
    provider: &str,
    account: &PartAccount,
    needs: Vec<PartNeed>,
) -> Account {
    let kind = match account.kind {
        PartKind::Subscription => AccountKind::Subscription,
        PartKind::Key => AccountKind::Key,
        PartKind::Organization => AccountKind::Organization,
    };
    let found_in = account
        .found_in
        .iter()
        .map(|place| match place {
            LoginFolder::Profile((agent, folder)) => {
                FoundIn::profile(AgentId::new(agent.as_str()), folder)
            },
            LoginFolder::OwnLogin((id, folder)) => {
                FoundIn::own_login(id.as_str(), folder)
            },
        })
        .collect();

    Account::new(
        plugin,
        ProviderId::new(provider),
        account.id.as_str(),
        account.name.as_str(),
        kind,
        found_in,
    )
    .with_plan(account.plan.clone())
    .with_needs(needs)
}

/// `account` as its part is given it to read: its id, name, kind, plan
/// and folders. Its needs are OTTY's to show, so they are not sent back.
fn part_account(account: &Account) -> PartAccount {
    let kind = match account.kind() {
        AccountKind::Subscription => PartKind::Subscription,
        AccountKind::Key => PartKind::Key,
        AccountKind::Organization => PartKind::Organization,
    };
    let found_in = account
        .found_in()
        .iter()
        .map(|place| match place {
            FoundIn::ProfileFolder { agent, folder } => LoginFolder::Profile((
                agent.as_str().to_string(),
                folder.display().to_string(),
            )),
            FoundIn::OwnLogin { id, folder } => LoginFolder::OwnLogin((
                id.clone(),
                folder.display().to_string(),
            )),
        })
        .collect();

    PartAccount {
        id: account.key().id().to_string(),
        name: account.name().to_string(),
        kind,
        plan: account.plan().map(String::from),
        found_in,
        needs: Vec::new(),
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
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::given_key::GivenSecret;
    use crate::limits::Credits;
    use crate::testing::{NOW, at};

    /// The made-up agent part `coder` of the plugin `plugin`.
    fn coder_of(plugin: &str) -> Box<dyn Agent> {
        let mut agent = crate::testing::MockAgent::new();
        let part = otty_plugin_host::PluginPart::new(
            plugin, plugin, "0.1.0", "", "coder",
        );
        let entry = crate::agent::AgentEntry::new(
            AgentId::new("coder"),
            "Coder",
            "coder",
            "Example Cloud",
            part,
        );
        agent.expect_entry().return_const(entry);

        Box::new(agent)
    }

    /// A profile of the agent `agent` in `folder`.
    fn profile_of(agent: &str, folder: &str) -> Profile {
        Profile::added(AgentId::new(agent), folder, PathBuf::from(folder))
    }

    /// A subscription of the plugin `example` found in the Coder profile
    /// `/home/dev/.coder`.
    fn subscription() -> Account {
        Account::new(
            "example",
            ProviderId::new("example-cloud"),
            "chatgpt:made-up",
            "dev@example.com",
            AccountKind::Subscription,
            vec![FoundIn::profile(AgentId::new("coder"), "/home/dev/.coder")],
        )
    }

    /// A key of the plugin `example` found in the Coder profile
    /// `/home/dev/.coder`.
    fn key() -> Account {
        Account::new(
            "example",
            ProviderId::new("example-cloud"),
            "key:made-up",
            "key ••••a1f3",
            AccountKind::Key,
            vec![FoundIn::profile(AgentId::new("coder"), "/home/dev/.coder")],
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

    /// The folders of `folders`, as text: the agent part of a profile
    /// folder, the id of an own login folder.
    fn folders_of(folders: &[LoginFolder]) -> Vec<(&str, &str)> {
        folders
            .iter()
            .map(|folder| match folder {
                LoginFolder::Profile((agent, folder))
                | LoginFolder::OwnLogin((agent, folder)) => {
                    (agent.as_str(), folder.as_str())
                },
            })
            .collect()
    }

    const PLUGINS_FOLDER: &str = "/home/dev/.local/share/otty/plugins";

    /// An own login of the part `provider` of the plugin `plugin`.
    fn own_login_of(id: &str, plugin: &str, provider: &str) -> OwnLogin {
        OwnLogin::new(
            id,
            plugin,
            ProviderId::new(provider),
            AgentId::new("coder"),
        )
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6: the part
    /// gets the folders of its own logins next to its profile folders,
    /// each under the folder of its plugin; the own logins of another part
    /// or plugin are not its.
    #[test]
    fn the_part_gets_its_own_logins_next_to_its_profile_folders() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_accounts()
            .withf(|folders| {
                folders_of(folders)
                    == [
                        ("coder", "/home/dev/.coder"),
                        (
                            "a1",
                            "/home/dev/.local/share/otty/plugins/example/accounts/a1",
                        ),
                    ]
            })
            .times(1)
            .returning(|_| Ok((Vec::new(), Vec::new())));
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);
        let own_logins = [
            own_login_of("a1", "example", "example-cloud"),
            own_login_of("b2", "another", "example-cloud"),
            own_login_of("c3", "example", "other-cloud"),
        ];

        let found = provider.accounts(
            &[coder_of("example")],
            &[profile_of("coder", "/home/dev/.coder")],
            &own_logins,
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(found, Ok((Vec::new(), Vec::new())));
    }

    /// Decision 6: an own login is enough to ask the part, with no
    /// profile of its agent part shown.
    #[test]
    fn a_part_with_only_an_own_login_is_asked() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_accounts()
            .times(1)
            .returning(|_| Ok((Vec::new(), Vec::new())));
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);

        let found = provider.accounts(
            &[],
            &[],
            &[own_login_of("a1", "example", "example-cloud")],
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(found, Ok((Vec::new(), Vec::new())));
    }

    /// Decision 8: an account found in an own login folder names that
    /// login, and goes back to the part the same way.
    #[test]
    fn an_account_found_in_an_own_login_keeps_that_place() {
        let part = PartAccount {
            id: "key:made-up".to_string(),
            name: "dev@example.com".to_string(),
            kind: PartKind::Key,
            plan: None,
            found_in: vec![
                LoginFolder::Profile((
                    "coder".to_string(),
                    "/home/dev/.coder".to_string(),
                )),
                LoginFolder::OwnLogin((
                    "a1".to_string(),
                    "/home/dev/.local/share/otty/plugins/example/accounts/a1"
                        .to_string(),
                )),
            ],
            needs: Vec::new(),
        };

        let account = account_of("example", "example-cloud", &part, Vec::new());

        assert_eq!(
            account.found_in(),
            [
                FoundIn::profile(AgentId::new("coder"), "/home/dev/.coder"),
                FoundIn::own_login(
                    "a1",
                    "/home/dev/.local/share/otty/plugins/example/accounts/a1"
                ),
            ]
        );
        assert_eq!(
            format!("{:?}", part_account(&account)),
            format!("{part:?}")
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: the part
    /// is asked for accounts with the folders of the profiles of the agent
    /// parts its `logins` name, of its own plugin, and of no other.
    #[test]
    fn the_part_gets_the_folders_of_its_own_agent_parts_only() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_accounts()
            .withf(|folders| {
                folders_of(folders)
                    == [
                        ("coder", "/home/dev/.coder"),
                        ("coder", "/home/dev/.w"),
                    ]
            })
            .times(1)
            .returning(|_| Ok((vec![subscription()], Vec::new())));
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);
        let profiles = [
            profile_of("coder", "/home/dev/.coder"),
            profile_of("other", "/home/dev/.other"),
            profile_of("coder", "/home/dev/.w"),
        ];

        let found = provider.accounts(
            &[coder_of("example")],
            &profiles,
            &[],
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(found, Ok((vec![subscription()], Vec::new())));
    }

    /// Decision 5: a profile of an agent part of another plugin gives the
    /// part no folder, even with the same agent id, and a part with no
    /// folder is not asked.
    #[test]
    fn a_part_with_no_folder_of_its_plugin_is_not_asked() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_accounts().never();
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);
        let profiles = [profile_of("coder", "/home/dev/.coder")];

        let found = provider.accounts(
            &[coder_of("another")],
            &profiles,
            &[],
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(found, Ok((Vec::new(), Vec::new())));
    }

    /// Decision 11: a part whose manifest names no `logins` finds no
    /// account, such as DeepSeek or OpenRouter.
    #[test]
    fn a_part_with_no_logins_finds_no_account() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_accounts().never();
        let provider = PluginProvider::made_up("example-cloud", &[], calls);
        let profiles = [profile_of("coder", "/home/dev/.coder")];

        let found = provider.accounts(
            &[coder_of("example")],
            &profiles,
            &[],
            Path::new(PLUGINS_FOLDER),
        );

        assert_eq!(found, Ok((Vec::new(), Vec::new())));
    }

    /// Decision 8: what the part tells of an account is the account, its
    /// folders by the agent part that holds them.
    #[test]
    fn an_account_of_the_part_keeps_its_id_name_kind_plan_and_folders() {
        let part = PartAccount {
            id: "chatgpt:made-up".to_string(),
            name: "dev@example.com".to_string(),
            kind: PartKind::Subscription,
            plan: Some("Plus".to_string()),
            found_in: vec![
                LoginFolder::Profile((
                    "coder".to_string(),
                    "/home/dev/.coder".to_string(),
                )),
                LoginFolder::Profile((
                    "coder".to_string(),
                    "/home/dev/.w".to_string(),
                )),
            ],
            needs: Vec::new(),
        };

        let account = account_of("example", "example-cloud", &part, Vec::new());

        assert_eq!(account.key().plugin(), "example");
        assert_eq!(account.key().id(), "chatgpt:made-up");
        assert_eq!(account.provider(), &ProviderId::new("example-cloud"));
        assert_eq!(account.name(), "dev@example.com");
        assert_eq!(account.kind(), AccountKind::Subscription);
        assert_eq!(account.plan(), Some("Plus"));
        let folders: Vec<&Path> =
            account.found_in().iter().map(FoundIn::folder).collect();
        assert_eq!(
            folders,
            [Path::new("/home/dev/.coder"), Path::new("/home/dev/.w")]
        );
        assert_eq!(
            format!("{:?}", part_account(&account)),
            format!("{part:?}")
        );
    }

    /// The part reads an account of its own plugin and provider id only.
    #[test]
    fn a_part_reads_no_account_of_another_plugin_or_provider() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().never();
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);
        let of_another_plugin = Account::new(
            "another",
            ProviderId::new("example-cloud"),
            "chatgpt:made-up",
            "dev@example.com",
            AccountKind::Subscription,
            Vec::new(),
        );
        let of_another_provider = Account::new(
            "example",
            ProviderId::new("other-cloud"),
            "chatgpt:made-up",
            "dev@example.com",
            AccountKind::Subscription,
            Vec::new(),
        );

        for account in [of_another_plugin, of_another_provider] {
            assert!(!provider.reads(&account));
            assert_eq!(
                provider.read(&account, &[], true),
                Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
            );
        }
    }

    /// Decision 5: the part reads an account with its id and its folders,
    /// and starts its programs only when the read may.
    #[test]
    fn a_subscription_is_read_in_its_folders_and_gives_its_limits() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|account, given, starts_programs| {
                account.id == "chatgpt:made-up"
                    && folders_of(&account.found_in)
                        == [("coder", "/home/dev/.coder")]
                    && given.is_empty()
                    && !*starts_programs
            })
            .times(1)
            .returning(|_, _, _| {
                Ok(PartNumbers {
                    windows: vec![
                        otty_plugin_host::agents::provider::LimitWindow {
                            length_minutes: Some(300),
                            model: None,
                            percent_used: 40.0,
                            resets_at_ms: None,
                            reset_text: None,
                        },
                    ],
                    ..nothing()
                })
            });
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);
        let account = subscription().with_plan(Some("Plus".into()));

        let numbers = provider.read(&account, &[], false);

        let Ok(AccountNumbers::Limits(limits)) = numbers else {
            panic!("the limits: {numbers:?}");
        };
        assert_eq!(limits.value().plan(), Some("Plus"));
    }

    /// Issue 20: what the user must do comes from the part as its need;
    /// OTTY writes none of it.
    #[test]
    fn a_need_of_the_part_is_the_answer_of_the_read_as_it_is() {
        let need = || {
            Box::new(crate::fixtures::key_need(
                "example",
                "example-cloud",
                "admin",
            ))
        };
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .times(1)
            .returning(move |_, _, _| Err(SourceError::NeedsUser(need())));
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);

        let numbers = provider.read(&key(), &[], true);

        assert_eq!(numbers, Err(SourceError::NeedsUser(need())));
    }

    /// Issue 20: the keys the user gave for the account reach the part's
    /// read with their slots, each as a handle to the key OTTY read.
    #[test]
    fn the_given_keys_reach_the_read_with_their_slots() {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_numbers()
            .withf(|_, given, _| {
                matches!(given, [(slot, KeySource::Given(key))]
                    if slot == "admin" && key.as_str() == "sk-made-up-admin-1")
            })
            .times(1)
            .returning(|_, _, _| Ok(nothing()));
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);
        let given = [GivenSecret::made_up("admin", "sk-made-up-admin-1")];

        let numbers = provider.read(&key(), &given, true);

        assert!(
            matches!(numbers, Ok(AccountNumbers::Balance(_))),
            "{numbers:?}"
        );
    }

    /// Issue 15: a key account shows the money spent this month and the
    /// credits its provider tells, next to the money left.
    #[test]
    fn a_key_gives_its_money_spent_this_month_and_its_credits() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_numbers().returning(|_, _, _| {
            Ok(PartNumbers {
                spent_this_month: Some(usd(4.2)),
                credits: Some(otty_plugin_host::agents::provider::Credits {
                    left: Some(80.0),
                    used: Some(20.0),
                    pool: Some(100.0),
                    renews_at_ms: None,
                    is_unlimited: false,
                }),
                ..nothing()
            })
        });
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);

        let numbers = provider.read(&key(), &[], true);

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
        calls.expect_read_numbers().returning(|_, _, _| {
            Ok(PartNumbers {
                extra_usage: Some(
                    otty_plugin_host::agents::provider::ExtraUsage {
                        used: Some(8.3),
                        monthly_limit: Some(50.0),
                        currency: Some("USD".to_string()),
                    },
                ),
                ..nothing()
            })
        });
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);

        let numbers = provider.read(&subscription(), &[], true);

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

    /// An organization of the plugin `example`, found in a Coder profile.
    fn organization() -> Account {
        Account::new(
            "example",
            ProviderId::new("example-cloud"),
            "org:made-up",
            "Made-up org",
            AccountKind::Organization,
            vec![FoundIn::profile(AgentId::new("coder"), "/home/dev/.coder")],
        )
    }

    /// Issue 15 and 20: the keys given for the organization reach the
    /// part only as handles, once, for the 31 UTC days up to today; the
    /// bill holds each of them.
    #[test]
    fn a_given_key_reads_the_bill_of_the_31_days_up_to_today() {
        let given = [GivenSecret::made_up("admin", "sk-made-up-org-key")];
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_bill()
            .withf(|account, given, days| {
                account.id == "org:made-up"
                    && matches!(given, [(slot, KeySource::Given(org))]
                        if slot == "admin"
                            && org.as_str() == "sk-made-up-org-key")
                    && days.first == "2026-10-17"
                    && days.last == "2026-11-16"
            })
            .times(1)
            .returning(|_, _, _| {
                Ok(PartBill {
                    days: vec![otty_plugin_host::agents::provider::DayCost {
                        day: "2026-11-16".to_string(),
                        cost: usd(12.4),
                    }],
                })
            });
        let provider =
            PluginProvider::made_up("example-cloud", &["coder"], calls);

        let bill = provider
            .read_bill(&organization(), &given, at(NOW))
            .expect("the bill");

        assert_eq!(bill.days().len(), 31);
        assert_eq!(bill.today().micro_dollars(), 12_400_000);
    }

    #[test]
    fn a_part_reads_no_bill_of_another_provider() {
        let mut calls = MockProviderPartCalls::new();
        calls.expect_read_bill().never();
        let provider = PluginProvider::made_up("openai", &["coder"], calls);

        let bill = provider.read_bill(&organization(), &[], at(NOW));

        assert_eq!(bill, Err(SourceError::NotAvailable(Unavailable::NoBill)));
    }
}
