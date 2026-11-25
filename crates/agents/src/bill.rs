//! The real bill of an organization, and its read through the provider
//! part that found it (spec Decision 9; spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 8). Which key reads
//! it is the part's business: a part that needs one the user must give
//! asks for it with its need (spec 2026-11-19-agent-plugins, issue 20).

use std::time::SystemTime;

use jiff::civil::Date;

use crate::account::Account;
use crate::dollars::Dollars;
use crate::error::{SourceError, Unavailable};
use crate::given_key::{GivenKey, read_given_keys};
use crate::local_time::{is_in_last_days, is_in_month_of};
use crate::plugin_provider::PluginProvider;

/// The money an organization was billed, day by day, as its provider
/// counts it. Unlike the money of a profile at API prices it is the
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
/// Decision 9) by the provider part of `providers` that found it, in the
/// folders it was found in, with the keys the user gave for it (`given`);
/// `env_var` gives the value of an environment variable, for a key kept
/// in one. With no such part there is no bill: "not available", and
/// nothing is sent.
///
/// Each given key is read from its file or its variable, used for this
/// read only and never kept: the part gets only a handle to it.
pub fn read_bill(
    providers: &[PluginProvider],
    organization: &Account,
    given: &[GivenKey],
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> Result<Bill, SourceError> {
    let part = providers
        .iter()
        .find(|part| part.reads(organization))
        .ok_or(SourceError::NotAvailable(Unavailable::NoBill))?;

    let given = read_given_keys(given, &env_var)?;

    part.read_bill(organization, &given, now)
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::KeySource;
    use otty_plugin_host::agents::provider::Bill as PartBill;

    use super::*;
    use crate::account::AccountKind;
    use crate::given_key::KeyRef;
    use crate::plugin_provider::MockProviderPartCalls;
    use crate::provider::ProviderId;
    use crate::testing::{NOW, at, dollars, organization};

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

    /// The provider part of Anthropic of the plugin `example`, whose bill
    /// is read `times`.
    fn anthropic_part(times: usize) -> PluginProvider {
        let mut calls = MockProviderPartCalls::new();
        calls
            .expect_read_bill()
            .withf(|account, given, days| {
                account.id == "org:made-up-org"
                    && matches!(given, [(slot, KeySource::Given(key))]
                        if slot == "admin" && key.as_str() == "made-up-key")
                    && days.last == "2026-11-16"
            })
            .times(times)
            .returning(|_, _, _| Ok(PartBill { days: Vec::new() }));

        PluginProvider::made_up("anthropic", &["claude-code"], calls)
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
    fn an_organization_no_part_found_has_no_bill() {
        let of_another_plugin = Account::new(
            "another",
            ProviderId::new("anthropic"),
            "org:made-up-org",
            "Made-up org",
            AccountKind::Organization,
            Vec::new(),
        );

        let no_part = read_bill(
            &[],
            &organization("made-up-org"),
            &given_admin(),
            at(NOW),
            org_key,
        );
        let other_plugin = read_bill(
            &[anthropic_part(0)],
            &of_another_plugin,
            &given_admin(),
            at(NOW),
            org_key,
        );

        for read in [no_part, other_plugin] {
            assert_eq!(
                read,
                Err(SourceError::NotAvailable(Unavailable::NoBill))
            );
        }
    }

    /// Issue 20: the bill is read by the part that found the
    /// organization, with the key the user gave under its slot, as a
    /// handle.
    #[test]
    fn the_bill_is_read_with_the_given_keys() {
        let read = read_bill(
            &[anthropic_part(1)],
            &organization("made-up-org"),
            &given_admin(),
            at(NOW),
            org_key,
        );
        let unset = read_bill(
            &[anthropic_part(0)],
            &organization("made-up-org"),
            &given_admin(),
            at(NOW),
            |_| None,
        );

        assert_eq!(read.map(|bill| bill.days().len()), Ok(31));
        assert!(matches!(
            unset,
            Err(SourceError::NotAvailable(Unavailable::KeyMissing(_)))
        ));
    }
}
