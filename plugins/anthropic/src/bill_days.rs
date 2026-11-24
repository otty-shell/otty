//! The money of each day of a bill, added up while the pages of a cost
//! report are read. The money is counted in micro-dollars, so the sums of
//! many small costs stay exact.

use std::collections::BTreeMap;

use jiff::civil::Date;
use otty_plugin_sdk::agents::{Bill, DayCost, DayRange, Money, ReadError};

/// Micro-dollars in one dollar.
const MICRO_DOLLARS_IN_DOLLAR: f64 = 1_000_000.0;

/// The sums of the days of one bill, from its first day to its last.
#[derive(Debug)]
pub(crate) struct DaySums {
    first: Date,
    last: Date,
    sums: BTreeMap<Date, i64>,
}

impl DaySums {
    /// The first day of the bill.
    pub(crate) fn first(&self) -> Date {
        self.first
    }

    /// The last day of the bill.
    pub(crate) fn last(&self) -> Date {
        self.last
    }

    /// No money yet for the days of `range`; unexpected when its days are
    /// not `YYYY-MM-DD` dates, the first not after the last.
    pub(crate) fn new(range: &DayRange) -> Result<Self, ReadError> {
        let day = |text: &str| -> Result<Date, ReadError> {
            text.parse().map_err(|_| {
                ReadError::Unexpected(format!("{text} is not a day"))
            })
        };
        let first = day(&range.first)?;
        let last = day(&range.last)?;
        if first > last {
            return Err(ReadError::Unexpected(format!(
                "the bill starts on {first}, after its last day {last}"
            )));
        }

        Ok(Self {
            first,
            last,
            sums: BTreeMap::new(),
        })
    }

    /// Adds `micro_dollars` to `day`; a day outside the bill is skipped.
    pub(crate) fn add(&mut self, day: Date, micro_dollars: i64) {
        if day < self.first || day > self.last {
            return;
        }

        let sum = self.sums.entry(day).or_default();
        *sum = sum.saturating_add(micro_dollars);
    }

    /// The bill: each day that had a cost, in US dollars, oldest first. A
    /// day whose sum is below zero, such as a day of refunds only, costs
    /// nothing.
    pub(crate) fn into_bill(self) -> Bill {
        let days = self
            .sums
            .into_iter()
            .map(|(day, sum)| DayCost {
                day: day.to_string(),
                cost: Money {
                    amount: sum.max(0) as f64 / MICRO_DOLLARS_IN_DOLLAR,
                    currency: "USD".to_string(),
                    by_kind: None,
                },
            })
            .collect();

        Bill { days }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(first: &str, last: &str) -> DayRange {
        DayRange {
            first: first.to_string(),
            last: last.to_string(),
        }
    }

    #[test]
    fn the_days_of_the_range_add_up_and_others_are_skipped() {
        let mut sums =
            DaySums::new(&range("2026-10-17", "2026-11-16")).expect("days");

        sums.add(jiff::civil::date(2026, 11, 16), 1_500_000);
        sums.add(jiff::civil::date(2026, 11, 16), 250_000);
        sums.add(jiff::civil::date(2026, 11, 1), -5);
        sums.add(jiff::civil::date(2026, 10, 16), 9_000_000);
        sums.add(jiff::civil::date(2026, 11, 17), 9_000_000);

        let bill = sums.into_bill();
        let days: Vec<(&str, f64)> = bill
            .days
            .iter()
            .map(|day| (day.day.as_str(), day.cost.amount))
            .collect();
        assert_eq!(days, [("2026-11-01", 0.0), ("2026-11-16", 1.75)]);
    }

    #[test]
    fn a_range_that_is_no_days_is_unexpected() {
        for (first, last) in
            [("yesterday", "2026-11-16"), ("2026-11-17", "2026-11-16")]
        {
            let sums = DaySums::new(&range(first, last));

            assert!(matches!(sums, Err(ReadError::Unexpected(_))), "{first}");
        }
    }
}
