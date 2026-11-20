//! The real bill of an API key account, and its read through the provider
//! part that serves the organization (spec Decision 9).

use std::time::SystemTime;

use jiff::civil::Date;

use crate::admin_key::AdminKeyRef;
use crate::dollars::Dollars;
use crate::error::{SourceError, Unavailable};
use crate::local_time::{is_in_last_days, is_in_month_of};
use crate::plugin_provider::PluginProvider;
use crate::provider::{AddAccountKind, ProviderRef};

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

/// The bill up to `now` of an organization at `provider`, read once for
/// it (spec Decision 9) by the provider part of `providers` that serves
/// `provider` and offers Add account with an admin key, with the
/// organization's admin key kept at `admin_key`; `env_var` gives the value
/// of an environment variable, for a key kept in one. A provider with no
/// such part has no bill: "not available", and nothing is sent.
///
/// The key is read from its file or its variable, used for this read only
/// and never kept: the part gets only a handle to it.
pub fn read_bill(
    providers: &[PluginProvider],
    provider: &ProviderRef,
    admin_key: &AdminKeyRef,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<Bill, SourceError> {
    let part = providers
        .iter()
        .filter(|part| {
            part.entry().add_account() == Some(AddAccountKind::AdminKey)
        })
        .find(|part| part.serves_provider(provider))
        .ok_or(SourceError::NotAvailable(Unavailable::NoBill))?;

    let key = admin_key.read(env_var)?;
    part.read_bill(provider, &key, now)
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::KeySource;
    use otty_plugin_host::provider::Bill as PartBill;

    use super::*;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::testing::{NOW, at, dollars};

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

    fn by_variable(variable: &str) -> AdminKeyRef {
        AdminKeyRef::EnvVar(variable.to_string())
    }

    fn anthropic() -> ProviderRef {
        ProviderRef::of_id("anthropic")
    }

    /// The provider part of Anthropic, offering an admin key when
    /// `admin_key`, whose bill is read `times`.
    fn anthropic_part(admin_key: bool, times: usize) -> PluginProvider {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_bill()
            .withf(|key, days| {
                matches!(key, KeySource::Given(given)
                    if given.as_str() == "made-up-key")
                    && days.last == "2026-11-16"
            })
            .times(times)
            .returning(|_, _| Ok(PartBill { days: Vec::new() }));
        let part =
            PluginProvider::made_up("anthropic", &["anthropic.com"], calls);

        match admin_key {
            true => part.with_add_account(AddAccountKind::AdminKey),
            false => part,
        }
    }

    #[test]
    fn a_provider_with_no_part_for_admin_keys_is_not_available() {
        let key = |_: &str| Some(String::from("made-up-key"));

        let no_part =
            read_bill(&[], &anthropic(), &by_variable("KEY"), at(NOW), key);
        let no_admin_key = read_bill(
            &[anthropic_part(false, 0)],
            &anthropic(),
            &by_variable("KEY"),
            at(NOW),
            key,
        );
        let other = read_bill(
            &[anthropic_part(true, 0)],
            &ProviderRef::of_id("openai"),
            &by_variable("KEY"),
            at(NOW),
            key,
        );

        for read in [no_part, no_admin_key, other] {
            assert_eq!(
                read,
                Err(SourceError::NotAvailable(Unavailable::NoBill))
            );
        }
    }

    /// Issue 13 and 15 of spec 2026-11-19-agent-plugins: the admin key
    /// saved for a Claude Code profile reads the bill of its organization
    /// again, through the provider part `anthropic`.
    #[test]
    fn the_admin_key_is_read_and_given_to_the_provider_part() {
        let admin_key = by_variable("ANTHROPIC_ADMIN_KEY");

        let read = read_bill(
            &[anthropic_part(true, 1)],
            &anthropic(),
            &admin_key,
            at(NOW),
            |name| {
                (name == "ANTHROPIC_ADMIN_KEY")
                    .then(|| String::from("made-up-key"))
            },
        );
        let unset = read_bill(
            &[anthropic_part(true, 0)],
            &anthropic(),
            &admin_key,
            at(NOW),
            no_variables,
        );

        assert_eq!(read.map(|bill| bill.days().len()), Ok(31));
        assert!(matches!(unset, Err(SourceError::NotAvailable(_))));
    }
}
