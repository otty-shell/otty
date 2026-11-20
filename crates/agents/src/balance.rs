//! The money a provider tells of a key (spec 2026-11-19, Decision 5): the
//! money left on the account, the limit of the key and how much of it was
//! used, the money spent this month and the credits of the account.

use crate::limits::Credits;

/// An amount of money as a provider told it, in its currency, such as
/// `110.0` Chinese yuan.
#[derive(Debug, Clone, PartialEq)]
pub struct Amount {
    value: f64,
    currency: String,
}

impl Amount {
    /// The amount in whole units of its currency, never below zero.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The ISO code of the currency in capitals, such as `USD` or `CNY`.
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// `value` in the currency `currency`; an amount below zero, as a
    /// provider may tell of an account that spent more than it had, is
    /// zero. `None` for a value that is not a number.
    ///
    /// ```
    /// use otty_agents::Amount;
    ///
    /// let left = Amount::new(-0.5, "cny").expect("a number");
    ///
    /// assert_eq!(left.value(), 0.0);
    /// assert_eq!(left.currency(), "CNY");
    /// ```
    pub fn new(value: f64, currency: &str) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }

        Some(Self {
            value: value.max(0.0),
            currency: currency.trim().to_ascii_uppercase(),
        })
    }

    /// `value` US dollars; `None` for a value that is not a number.
    pub(crate) fn dollars(value: f64) -> Option<Self> {
        Self::new(value, "USD")
    }
}

/// The limit of an OpenRouter key and how much of it was used, in US
/// dollars.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyLimit {
    used: Amount,
    limit: Option<Amount>,
}

impl KeyLimit {
    /// The money the key used.
    pub fn used(&self) -> &Amount {
        &self.used
    }

    /// The most the key may use; `None` for a key with no limit.
    pub fn limit(&self) -> Option<&Amount> {
        self.limit.as_ref()
    }

    /// A key that used `used` of `limit`.
    pub fn new(used: Amount, limit: Option<Amount>) -> Self {
        Self { used, limit }
    }
}

/// What a provider told of the money of a key: the money left on the
/// account, the limit of the key, the money spent this month and the
/// credits of the account. Any may be missing: the screen shows only what
/// the provider told.
///
/// ```
/// use otty_agents::{Amount, Balance};
///
/// let balance = Balance::new(Amount::new(110.0, "CNY"), None);
///
/// assert_eq!(balance.left().map(|left| left.currency()), Some("CNY"));
/// assert_eq!(balance.key_limit(), None);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    left: Option<Amount>,
    key_limit: Option<KeyLimit>,
    spent_this_month: Option<Amount>,
    credits: Option<Credits>,
}

impl Balance {
    /// The money left on the account, as "Left" shows it.
    pub fn left(&self) -> Option<&Amount> {
        self.left.as_ref()
    }

    /// The limit of the key and what of it was used, as "Key limit" shows
    /// it.
    pub fn key_limit(&self) -> Option<&KeyLimit> {
        self.key_limit.as_ref()
    }

    /// The money spent this month, as the provider counts it.
    pub fn spent_this_month(&self) -> Option<&Amount> {
        self.spent_this_month.as_ref()
    }

    /// The credits of an account paid for by a pool of credits.
    pub fn credits(&self) -> Option<&Credits> {
        self.credits.as_ref()
    }

    /// A balance of the money left and the limit of the key.
    pub fn new(left: Option<Amount>, key_limit: Option<KeyLimit>) -> Self {
        Self {
            left,
            key_limit,
            spent_this_month: None,
            credits: None,
        }
    }

    /// The same balance with the money spent this month, `spent`.
    ///
    /// ```
    /// use otty_agents::{Amount, Balance};
    ///
    /// let balance =
    ///     Balance::new(None, None).with_spent_this_month(Amount::new(4.2, "USD"));
    ///
    /// assert_eq!(
    ///     balance.spent_this_month().map(|spent| spent.value()),
    ///     Some(4.2)
    /// );
    /// ```
    pub fn with_spent_this_month(mut self, spent: Option<Amount>) -> Self {
        self.spent_this_month = spent;
        self
    }

    /// The same balance with the credits `credits` of the account.
    pub fn with_credits(mut self, credits: Option<Credits>) -> Self {
        self.credits = credits;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_amount_below_zero_is_zero_and_not_a_number_is_none() {
        assert_eq!(Amount::dollars(-3.0).map(|left| left.value()), Some(0.0));
        assert_eq!(Amount::dollars(f64::NAN), None);
        assert_eq!(Amount::dollars(f64::INFINITY), None);
    }

    #[test]
    fn the_currency_is_kept_in_capitals() {
        let amount = Amount::new(1.5, " usd ").expect("a number");

        assert_eq!(amount.currency(), "USD");
        assert_eq!(amount.value(), 1.5);
    }
}
