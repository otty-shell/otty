//! The bill of an organization as a provider part reads it (spec
//! Decision 9; issue 15 of spec 2026-11-19-agent-plugins): OTTY asks for
//! the UTC days from 30 days before today to today, which always hold the
//! first day of the month, and makes one day of the bill of each, with
//! the money the part gave for it.

use std::collections::BTreeMap;
use std::time::SystemTime;

use jiff::ToSpan;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use otty_plugin_host::provider::{Bill as PartBill, DayRange};

use crate::bill::{Bill, DayCost};
use crate::dollars::Dollars;
use crate::local_time::{LocalNow, days_before};

/// The days of a bill: today and the 30 days before it.
const DAYS: i32 = 31;

/// The days of the bill OTTY asks for at `now`, from the first to the
/// last, both UTC days: the providers count their days in UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BillDays {
    first: Date,
    last: Date,
}

impl BillDays {
    /// The 31 UTC days up to the day of `now`.
    pub(crate) fn up_to(now: SystemTime) -> Self {
        let last = LocalNow::new(now, TimeZone::UTC).today();

        Self {
            first: days_before(last, DAYS - 1),
            last,
        }
    }

    /// The days as the provider contract asks for them, `YYYY-MM-DD`.
    pub(crate) fn range(&self) -> DayRange {
        DayRange {
            first: self.first.to_string(),
            last: self.last.to_string(),
        }
    }

    /// The bill of these days from what the part read: one day for each,
    /// oldest first and today last, a day the part gave nothing for at
    /// zero. A day outside these days, one that is not a date, an amount
    /// in another currency than US dollars and one that is not a number
    /// are left out.
    pub(crate) fn bill_of(&self, read: &PartBill) -> Bill {
        let mut costs: BTreeMap<Date, Dollars> = BTreeMap::new();
        for day in &read.days {
            let Ok(date) = day.day.parse::<Date>() else {
                continue;
            };
            if !day.cost.currency.eq_ignore_ascii_case("USD") {
                continue;
            }
            let Some(cost) = Dollars::from_dollars(day.cost.amount) else {
                continue;
            };

            let sum = costs.entry(date).or_default();
            *sum += cost;
        }

        let days = self
            .first
            .series(1.day())
            .take_while(|date| *date <= self.last)
            .map(|date| {
                let cost = costs.get(&date).copied().unwrap_or_default();
                DayCost::new(date, cost)
            })
            .collect();
        Bill::new(days)
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::provider::{DayCost as PartDay, Money};

    use super::*;
    use crate::testing::{NOW, at, dollars};

    fn part_day(day: &str, amount: f64, currency: &str) -> PartDay {
        PartDay {
            day: day.to_string(),
            cost: Money {
                amount,
                currency: currency.to_string(),
            },
        }
    }

    #[test]
    fn the_days_are_the_31_utc_days_up_to_today() {
        let range = BillDays::up_to(at(NOW)).range();

        assert_eq!(range.first, "2026-10-17");
        assert_eq!(range.last, "2026-11-16");
    }

    #[test]
    fn every_day_of_the_bill_is_there_and_a_day_without_money_is_zero() {
        let days = BillDays::up_to(at(NOW));
        let read = PartBill {
            days: vec![
                part_day("2026-11-01", 1.5, "USD"),
                part_day("2026-11-16", 2.0, "usd"),
                part_day("2026-11-16", 0.25, "USD"),
            ],
        };

        let bill = days.bill_of(&read);

        assert_eq!(bill.days().len(), 31);
        assert_eq!(bill.today(), Dollars::from_micro_dollars(2_250_000));
        assert_eq!(bill.this_month(), Dollars::from_micro_dollars(3_750_000));
        assert_eq!(bill.days()[0].cost(), Dollars::default());
    }

    #[test]
    fn what_can_not_be_read_is_left_out() {
        let days = BillDays::up_to(at(NOW));
        let read = PartBill {
            days: vec![
                part_day("yesterday", 5.0, "USD"),
                part_day("2026-09-01", 5.0, "USD"),
                part_day("2026-11-17", 5.0, "USD"),
                part_day("2026-11-16", 5.0, "EUR"),
                part_day("2026-11-16", -5.0, "USD"),
                part_day("2026-11-16", f64::NAN, "USD"),
                part_day("2026-11-16", 1.0, "USD"),
            ],
        };

        let bill = days.bill_of(&read);

        assert_eq!(bill.today(), dollars(1));
        assert_eq!(bill.this_month(), dollars(1));
    }
}
