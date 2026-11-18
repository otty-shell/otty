//! An amount of US dollars, kept in whole micro-dollars so that adding up
//! many small prices never drifts the way floating point numbers do.

use std::ops::{Add, AddAssign};

/// Micro-dollars in one cent.
const MICRO_DOLLARS_IN_CENT: u64 = 10_000;
/// Micro-dollars in one dollar.
const MICRO_DOLLARS_IN_DOLLAR: f64 = 1_000_000.0;

/// An amount of US dollars, kept as whole micro-dollars: one millionth of a
/// dollar, so `$12.40` is `12_400_000`.
///
/// A micro-dollar is fine enough for the cheapest list price per token and
/// large enough for any spend: a `u64` holds more than eighteen million
/// million dollars.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dollars {
    micro_dollars: u64,
}

impl Dollars {
    /// The amount in micro-dollars.
    pub fn micro_dollars(&self) -> u64 {
        self.micro_dollars
    }

    /// The amount in whole cents; half a cent and more rounds up.
    ///
    /// ```
    /// use otty_agents::Dollars;
    ///
    /// assert_eq!(Dollars::from_micro_dollars(12_405_000).cents(), 1_241);
    /// ```
    pub fn cents(&self) -> u64 {
        self.micro_dollars.saturating_add(MICRO_DOLLARS_IN_CENT / 2)
            / MICRO_DOLLARS_IN_CENT
    }

    /// The amount in dollars, such as `49.5`, for a file that keeps
    /// amounts as plain numbers.
    pub fn dollars(&self) -> f64 {
        self.micro_dollars as f64 / MICRO_DOLLARS_IN_DOLLAR
    }

    /// An amount of `micro_dollars` millionths of a dollar.
    ///
    /// ```
    /// use otty_agents::Dollars;
    ///
    /// let price = Dollars::from_micro_dollars(12_400_000);
    ///
    /// assert_eq!(price.micro_dollars(), 12_400_000);
    /// ```
    pub const fn from_micro_dollars(micro_dollars: u64) -> Self {
        Self { micro_dollars }
    }

    /// An amount of `cents` whole cents; `None` when it is too large to
    /// keep.
    ///
    /// ```
    /// use otty_agents::Dollars;
    ///
    /// let budget = Dollars::from_cents(4_950);
    ///
    /// assert_eq!(
    ///     budget.map(|budget| budget.micro_dollars()),
    ///     Some(49_500_000)
    /// );
    /// ```
    pub fn from_cents(cents: u64) -> Option<Self> {
        let micro_dollars = cents.checked_mul(MICRO_DOLLARS_IN_CENT)?;

        Some(Self::from_micro_dollars(micro_dollars))
    }

    /// An amount of `dollars` written as a plain number, such as `49.5`,
    /// to the nearest micro-dollar; `None` for an amount below zero or not
    /// a number. An amount too large to keep stays at the largest one.
    ///
    /// ```
    /// use otty_agents::Dollars;
    ///
    /// let budget = Dollars::from_dollars(49.5);
    ///
    /// assert_eq!(
    ///     budget.map(|budget| budget.micro_dollars()),
    ///     Some(49_500_000)
    /// );
    /// assert_eq!(Dollars::from_dollars(-1.0), None);
    /// ```
    pub fn from_dollars(dollars: f64) -> Option<Self> {
        if !dollars.is_finite() || dollars < 0.0 {
            return None;
        }

        let micro_dollars = (dollars * MICRO_DOLLARS_IN_DOLLAR).round() as u64;
        Some(Self::from_micro_dollars(micro_dollars))
    }
}

impl Add for Dollars {
    type Output = Self;

    /// The sum of two amounts. A sum too large to keep stays at the
    /// largest amount.
    fn add(self, other: Self) -> Self {
        Self {
            micro_dollars: self
                .micro_dollars
                .saturating_add(other.micro_dollars),
        }
    }
}

impl AddAssign for Dollars {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_add_up() {
        let mut sum = Dollars::from_micro_dollars(1_500_000);

        sum += Dollars::from_micro_dollars(250_000);

        assert_eq!(sum, Dollars::from_micro_dollars(1_750_000));
    }

    #[test]
    fn a_sum_too_large_stays_at_the_largest_amount() {
        let sum = Dollars::from_micro_dollars(u64::MAX)
            + Dollars::from_micro_dollars(1);

        assert_eq!(sum.micro_dollars(), u64::MAX);
    }

    #[test]
    fn half_a_cent_and_more_rounds_up_to_the_next_cent() {
        let cents =
            |micro_dollars| Dollars::from_micro_dollars(micro_dollars).cents();

        assert_eq!(cents(0), 0);
        assert_eq!(cents(4_999), 0);
        assert_eq!(cents(5_000), 1);
        assert_eq!(cents(12_400_000), 1_240);
        assert_eq!(cents(u64::MAX), u64::MAX / 10_000);
    }

    #[test]
    fn cents_are_ten_thousand_micro_dollars_each() {
        assert_eq!(
            Dollars::from_cents(4_950),
            Some(Dollars::from_micro_dollars(49_500_000))
        );
        assert_eq!(Dollars::from_cents(u64::MAX), None);
    }

    #[test]
    fn whole_dollars_turn_to_micro_dollars_and_back() {
        let budget = Dollars::from_dollars(49.5);

        assert_eq!(budget, Some(Dollars::from_micro_dollars(49_500_000)));
        assert_eq!(budget.map(|budget| budget.dollars()), Some(49.5));
        assert_eq!(
            Dollars::from_dollars(0.000_000_4),
            Some(Dollars::default())
        );
    }

    #[test]
    fn dollars_below_zero_or_not_a_number_are_no_amount() {
        for wrong in [-0.01, f64::NAN, f64::INFINITY] {
            assert_eq!(Dollars::from_dollars(wrong), None, "{wrong}");
        }
    }
}
