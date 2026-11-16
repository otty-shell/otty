//! An amount of US dollars, kept in whole micro-dollars so that adding up
//! many small prices never drifts the way floating point numbers do.

use std::ops::{Add, AddAssign};

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
}
