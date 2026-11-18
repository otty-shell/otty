//! The real bill of an API key account, and the sources that read it (spec
//! Decision 9).

use std::time::SystemTime;

use jiff::civil::Date;

use crate::admin_key::AdminKey;
use crate::dollars::Dollars;
use crate::error::SourceError;
use crate::local_time::is_in_last_days;

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
            .filter(|day| day.date.first_of_month() == today.first_of_month())
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
    /// Each of the last 30 days, oldest first and today last.
    pub(crate) fn last_30_days_by_day(&self) -> &[DayCost] {
        let start = self.days.len().saturating_sub(30);

        &self.days[start..]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn day(month: i8, day: i8, dollars: u64) -> DayCost {
        DayCost::new(
            jiff::civil::date(2026, month, day),
            Dollars::from_micro_dollars(dollars * 1_000_000),
        )
    }

    /// The 31 days from 2026-10-17 to 2026-11-16, each costing its day of
    /// the month in dollars.
    fn bill() -> Bill {
        let dollars = |number: i8| u64::from(number.unsigned_abs());
        let october = (17..=31).map(|number| day(10, number, dollars(number)));
        let november = (1..=16).map(|number| day(11, number, dollars(number)));

        Bill::new(october.chain(november).collect())
    }

    fn dollars(amount: u64) -> Dollars {
        Dollars::from_micro_dollars(amount * 1_000_000)
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
    fn the_bars_are_the_last_30_days() {
        let bill = bill();

        let days = bill.last_30_days_by_day();

        assert_eq!(days.len(), 30);
        assert_eq!(days[0], day(10, 18, 18));
        assert_eq!(days[29], day(11, 16, 16));
    }

    #[test]
    fn an_empty_bill_is_nothing() {
        let bill = Bill::new(Vec::new());

        assert_eq!(bill.today(), Dollars::default());
        assert_eq!(bill.this_month(), Dollars::default());
        assert!(bill.last_30_days_by_day().is_empty());
    }
}
