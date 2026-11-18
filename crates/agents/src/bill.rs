//! The real bill of an API key account, and the sources that read it (spec
//! Decision 9).

use std::time::SystemTime;

use jiff::civil::Date;

use crate::admin_key::AdminKey;
use crate::agent::{Agent, agent_of};
use crate::dollars::Dollars;
use crate::error::{SourceError, Unavailable};
use crate::local_time::{is_in_last_days, is_in_month_of};
use crate::profile::Profile;

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

/// One way to read the bill of an account with an admin key, such as
/// Anthropic's cost report.
///
/// It is a trait because it crosses into the network, and only some agents
/// have one: the app reads the bill of a profile only when its agent gives
/// a source.
#[cfg_attr(test, mockall::automock)]
pub trait BillSource: Send {
    /// The bill of the last 31 days up to `now`, read with `key`. The key
    /// is used for this request only and never kept.
    fn bill(
        &self,
        key: &AdminKey,
        now: SystemTime,
    ) -> Result<Bill, SourceError>;
}

/// The bill of `profile` up to `now`, read by the bill source of its agent
/// among `agents` with the profile's admin key; `env_var` gives the value
/// of an environment variable, for a key kept in one. An agent with no
/// bill and a profile with no admin key are "not available", and nothing
/// is sent.
///
/// The key is read from its file or its variable, used for this read only
/// and never kept.
pub fn read_bill(
    agents: &[Box<dyn Agent>],
    profile: &Profile,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<Bill, SourceError> {
    let source = agent_of(agents, profile.agent())
        .and_then(|agent| agent.bill_source())
        .ok_or(SourceError::NotAvailable(Unavailable::NoBill))?;
    let admin_key = profile
        .admin_key()
        .ok_or(SourceError::NotAvailable(Unavailable::NoAdminKey))?;

    let key = admin_key.read(env_var)?;
    source.bill(&key, now)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::admin_key::AdminKeyRef;
    use crate::agent::{AgentDescriptor, AgentId};
    use crate::known_agents;
    use crate::testing::{
        MockAgent, NOW, at, claude_profile, codex_profile, dollars,
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

    /// No environment variable is set.
    fn no_variables(_: &str) -> Option<String> {
        None
    }

    fn by_variable(variable: &str) -> Option<AdminKeyRef> {
        Some(AdminKeyRef::EnvVar(variable.to_string()))
    }

    #[test]
    fn no_admin_key_or_an_agent_with_no_bill_is_not_available() {
        let claude = claude_profile(Path::new("/home/user/.claude-api"));
        let codex = codex_profile(Path::new("/home/user/.codex-api"))
            .with_admin_key(by_variable("KEY"));
        let key = |_: &str| Some(String::from("made-up-key"));

        let no_key = read_bill(&known_agents(), &claude, at(NOW), key);
        let no_bill = read_bill(&known_agents(), &codex, at(NOW), key);

        assert_eq!(
            no_key,
            Err(SourceError::NotAvailable(Unavailable::NoAdminKey))
        );
        assert_eq!(
            no_bill,
            Err(SourceError::NotAvailable(Unavailable::NoBill))
        );
    }

    #[test]
    fn the_admin_key_is_read_and_given_to_the_bill_source() {
        const BILLED: AgentDescriptor = AgentDescriptor::new(
            AgentId::new("billed"),
            "Billed",
            "billed",
            "billed",
            ".billed",
            "BILLED_HOME",
            "Billed Labs",
        )
        .with_bill();
        let mut agent = MockAgent::new();
        agent.expect_descriptor().return_const(BILLED);
        agent.expect_bill_source().returning(|| {
            let mut source = MockBillSource::new();
            source
                .expect_bill()
                .withf(|key, now| {
                    key.secret() == "made-up-key" && *now == at(NOW)
                })
                .return_once(|_, _| Ok(bill()));
            Some(Box::new(source))
        });
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent)];
        let profile = Profile::added(
            BILLED.id(),
            "Billed",
            PathBuf::from("/home/user/.billed"),
        )
        .with_admin_key(by_variable("BILLED_ADMIN_KEY"));

        let read = read_bill(&agents, &profile, at(NOW), |variable| {
            (variable == "BILLED_ADMIN_KEY")
                .then(|| String::from("made-up-key"))
        });
        let unset = read_bill(&agents, &profile, at(NOW), no_variables);

        assert_eq!(read, Ok(bill()));
        assert!(matches!(unset, Err(SourceError::NotAvailable(_))));
    }
}
