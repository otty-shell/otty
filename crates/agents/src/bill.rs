//! The real bill of an API key account, and its read through the provider
//! part that serves the organization (spec Decision 9). Which key reads
//! it is the part's business: a part that needs one the user must give
//! asks for it with its need (spec 2026-11-19-agent-plugins, issue 20).

use std::time::SystemTime;

use jiff::civil::Date;

use crate::account::Account;
use crate::agent::{Agent, agent_of};
use crate::dollars::Dollars;
use crate::error::{SourceError, Unavailable};
use crate::given_key::{GivenKey, read_given_keys};
use crate::local_time::{is_in_last_days, is_in_month_of};
use crate::plugin_provider::PluginProvider;
use crate::plugin_values::found_profile;
use crate::profile::Profile;
use crate::refused_login::to_holding_agent;

/// The money an organization was billed, day by day, as its provider
/// counts it. Unlike [`MoneyEstimate`](crate::MoneyEstimate) it is the
/// bill of the whole organization, not only of one profile.
///
/// ```
/// use otty_agents::{Bill, DayCost, Dollars};
///
/// let bill = Bill::new(vec![
///     DayCost::new(
///         jiff::civil::date(2026, 10, 31),
///         Dollars::from_micro_dollars(1_000_000),
///     ),
///     DayCost::new(
///         jiff::civil::date(2026, 11, 1),
///         Dollars::from_micro_dollars(2_500_000),
///     ),
/// ]);
///
/// assert_eq!(bill.today().micro_dollars(), 2_500_000);
/// assert_eq!(bill.this_month().micro_dollars(), 2_500_000);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bill {
    days: Vec<DayCost>,
}

impl Bill {
    /// The money of today, the last day of the bill.
    pub fn today(&self) -> Dollars {
        self.last_days(1)
    }

    /// The money of the last 7 days, today included.
    pub fn last_7_days(&self) -> Dollars {
        self.last_days(7)
    }

    /// The money of the month of today.
    pub fn this_month(&self) -> Dollars {
        let Some(today) = self.days.last().map(|day| day.date) else {
            return Dollars::default();
        };

        self.days
            .iter()
            .filter(|day| is_in_month_of(day.date, today))
            .fold(Dollars::default(), |sum, day| sum + day.cost)
    }

    /// A bill of `days`, one entry per day, oldest first and today last.
    pub fn new(days: Vec<DayCost>) -> Self {
        Self { days }
    }

    /// The money of the last `count` days, today included.
    fn last_days(&self, count: i32) -> Dollars {
        let Some(today) = self.days.last().map(|day| day.date) else {
            return Dollars::default();
        };

        self.days
            .iter()
            .filter(|day| is_in_last_days(day.date, today, count))
            .fold(Dollars::default(), |sum, day| sum + day.cost)
    }
}

#[cfg(test)]
impl Bill {
    /// Each day of the bill, oldest first and today last.
    pub(crate) fn days(&self) -> &[DayCost] {
        &self.days
    }
}

/// The money of one day of a bill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayCost {
    date: Date,
    cost: Dollars,
}

impl DayCost {
    /// The date of the day.
    pub fn date(&self) -> Date {
        self.date
    }

    /// The money of the day.
    pub fn cost(&self) -> Dollars {
        self.cost
    }

    /// The money `cost` of the day `date`.
    pub fn new(date: Date, cost: Dollars) -> Self {
        Self { date, cost }
    }
}

/// The bill up to `now` of `organization`, read once for it (spec
/// Decision 9) through `profile`, which pays through it: by the provider
/// part of `providers` that reads it, with the key of the login the
/// agent part of `agents` found it by and the keys the user gave for it
/// (`given`); `env_var` gives the value of an environment variable, for a
/// key kept in one. With no such part there is no bill: "not available",
/// and nothing is sent.
///
/// Each given key is read from its file or its variable, used for this
/// read only and never kept: the part gets only a handle to it.
pub fn read_bill(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    organization: &Account,
    profile: &Profile,
    given: &[GivenKey],
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<Bill, SourceError> {
    let part = providers
        .iter()
        .find(|part| part.reads(organization))
        .ok_or(SourceError::NotAvailable(Unavailable::NoBill))?;
    let gone = || SourceError::NotAvailable(Unavailable::KeyGone);
    let agent = agent_of(agents, profile.agent()).ok_or_else(gone)?;
    let login = agent.found_login(profile, organization).ok_or_else(gone)?;
    let found_in = vec![found_profile(agent, profile)];

    let given = read_given_keys(given, &env_var)?;
    part.read_bill(organization, &login, found_in, &given, now)
        .map_err(|error| to_holding_agent(error, agent, profile, &login))
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::KeySource;
    use otty_plugin_host::agents::agent::KeyPlace;
    use otty_plugin_host::agents::provider::Bill as PartBill;

    use super::*;
    use crate::account::{AccountIdentity, AccountKind, AccountLogin, FoundIn};
    use crate::given_key::KeyRef;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::ProviderRef;
    use crate::testing::{
        MockAgent, NOW, at, dollars, mock_entry, organization, profile,
        read_by_example,
    };

    fn day(month: i8, day: i8, amount: u64) -> DayCost {
        DayCost::new(jiff::civil::date(2026, month, day), dollars(amount))
    }

    /// The 31 days from 2026-10-17 to 2026-11-16, each costing its day of
    /// the month in dollars.
    fn bill() -> Bill {
        let amount = |number: i8| u64::from(number.unsigned_abs());
        let october = (17..=31).map(|number| day(10, number, amount(number)));
        let november = (1..=16).map(|number| day(11, number, amount(number)));

        Bill::new(october.chain(november).collect())
    }

    #[test]
    fn today_7_days_and_this_month_add_up_their_days() {
        let bill = bill();

        assert_eq!(bill.today(), dollars(16));
        assert_eq!(
            bill.last_7_days(),
            dollars(10 + 11 + 12 + 13 + 14 + 15 + 16)
        );
        assert_eq!(bill.this_month(), dollars((1..=16).sum()));
    }

    #[test]
    fn an_empty_bill_is_nothing() {
        let bill = Bill::new(Vec::new());

        assert_eq!(bill.today(), Dollars::default());
        assert_eq!(bill.this_month(), Dollars::default());
    }

    /// An API key login of the profile, with the organization's id.
    fn api_key_login() -> otty_plugin_host::agents::agent::Login {
        use otty_plugin_host::agents::agent::{
            Identity, JsonField, Login, LoginKind, ProviderRef as PartProvider,
        };

        Login {
            kind: LoginKind::ApiKey,
            for_whom: PartProvider::Id("anthropic".to_string()),
            place: KeyPlace::JsonField(JsonField {
                file: "/home/user/.config-Work/settings.json".to_string(),
                pointer: "/apiKey".to_string(),
            }),
            identity: Identity {
                account_id: None,
                organization_id: Some("made-up-org".to_string()),
                email: None,
            },
            plan: None,
        }
    }

    /// The agent of the profile, which finds the login of the
    /// organization when `finds`.
    fn agents(finds: bool) -> Vec<Box<dyn Agent>> {
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("test-agent", "test-agent"));
        agent
            .expect_launch()
            .returning(|_, _| crate::AgentCommand::new("test-agent"));
        agent
            .expect_found_login()
            .returning(move |_, _| finds.then(api_key_login));

        vec![Box::new(agent)]
    }

    /// The provider part of Anthropic, whose bill is read `times`.
    fn anthropic_part(times: usize) -> PluginProvider {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_bill()
            .withf(|key, found, given, days| {
                matches!(key, KeySource::Place(KeyPlace::JsonField(field))
                    if field.pointer == "/apiKey")
                    && found.identity.organization_id.as_deref()
                        == Some("made-up-org")
                    && matches!(given, [(slot, KeySource::Given(key))]
                        if slot == "admin" && key.as_str() == "made-up-key")
                    && days.last == "2026-11-16"
            })
            .times(times)
            .returning(|_, _, _, _| Ok(PartBill { days: Vec::new() }));

        PluginProvider::made_up("anthropic", &["anthropic.com"], calls)
    }

    fn given_admin() -> Vec<GivenKey> {
        vec![GivenKey::new(
            "admin",
            KeyRef::EnvVar("ANTHROPIC_ORG_KEY".to_string()),
        )]
    }

    fn org_key(name: &str) -> Option<String> {
        (name == "ANTHROPIC_ORG_KEY").then(|| String::from("made-up-key"))
    }

    #[test]
    fn an_organization_no_part_serves_has_no_bill() {
        let other = Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::OrganizationId("made-up-org".into()),
            AccountLogin::Unnamed,
            AccountKind::Money,
            FoundIn::ProfileFolder("/home/user/.config-Work".into()),
        );

        let no_part = read_bill(
            &agents(true),
            &[],
            &read_by_example(organization("made-up-org")),
            &profile("Work"),
            &given_admin(),
            at(NOW),
            org_key,
        );
        let other_provider = read_bill(
            &agents(true),
            &[anthropic_part(0)],
            &other,
            &profile("Work"),
            &given_admin(),
            at(NOW),
            org_key,
        );

        for read in [no_part, other_provider] {
            assert_eq!(
                read,
                Err(SourceError::NotAvailable(Unavailable::NoBill))
            );
        }
    }

    /// Issue 20: the bill is read through the profile with the key of the
    /// login the agent part found and the key the user gave under its
    /// slot, each as a handle.
    #[test]
    fn the_bill_is_read_with_the_login_and_the_given_keys() {
        let read = read_bill(
            &agents(true),
            &[anthropic_part(1)],
            &read_by_example(organization("made-up-org")),
            &profile("Work"),
            &given_admin(),
            at(NOW),
            org_key,
        );
        let unset = read_bill(
            &agents(true),
            &[anthropic_part(0)],
            &read_by_example(organization("made-up-org")),
            &profile("Work"),
            &given_admin(),
            at(NOW),
            |_| None,
        );
        let gone = read_bill(
            &agents(false),
            &[anthropic_part(0)],
            &read_by_example(organization("made-up-org")),
            &profile("Work"),
            &given_admin(),
            at(NOW),
            org_key,
        );

        assert_eq!(read.map(|bill| bill.days().len()), Ok(31));
        assert!(matches!(
            unset,
            Err(SourceError::NotAvailable(Unavailable::KeyMissing(_)))
        ));
        assert_eq!(gone, Err(SourceError::NotAvailable(Unavailable::KeyGone)));
    }
}
