//! Billing: how the account of a profile is paid for (spec Decision 9),
//! with the numbers its source gave.

use std::time::SystemTime;

/// How an account is paid for, with its numbers. Every number may be
/// missing: the screen shows only what the source gave.
#[derive(Debug, Clone, PartialEq)]
pub enum Billing {
    /// A subscription with usage windows, such as Claude Max or ChatGPT
    /// Plus.
    UsageWindows(UsageWindows),
    /// A pool of credits, such as Codex on an enterprise plan.
    Credits(Credits),
    /// An API key charged per token. It has no numbers of its own: the
    /// money comes from the tokens spent.
    ApiKey,
}

impl Billing {
    /// The kind of this billing, without its numbers.
    ///
    /// ```
    /// use otty_agents::{Billing, BillingKind};
    ///
    /// assert_eq!(Billing::ApiKey.kind(), BillingKind::ApiKey);
    /// ```
    pub fn kind(&self) -> BillingKind {
        match self {
            Billing::UsageWindows(_) => BillingKind::UsageWindows,
            Billing::Credits(_) => BillingKind::Credits,
            Billing::ApiKey => BillingKind::ApiKey,
        }
    }
}

/// The kind of a billing, without its numbers: what the files of a profile
/// tell before any limits are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BillingKind {
    /// A subscription with usage windows.
    UsageWindows,
    /// A pool of credits.
    Credits,
    /// An API key charged per token.
    ApiKey,
}

/// The usage windows of a subscription and its extra usage.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageWindows {
    windows: Vec<LimitWindow>,
    extra_usage: Option<ExtraUsage>,
}

impl UsageWindows {
    /// The windows of today, such as the five-hour and the weekly one.
    pub fn windows(&self) -> &[LimitWindow] {
        &self.windows
    }

    /// The extra usage the account may buy, when the plan has it turned on.
    pub fn extra_usage(&self) -> Option<&ExtraUsage> {
        self.extra_usage.as_ref()
    }

    /// Usage windows from their parts.
    ///
    /// ```
    /// use otty_agents::{LimitWindow, UsageWindows};
    ///
    /// let usage =
    ///     UsageWindows::new(vec![LimitWindow::new("5 hours", 42.0, None)], None);
    ///
    /// assert_eq!(usage.windows().len(), 1);
    /// ```
    pub fn new(
        windows: Vec<LimitWindow>,
        extra_usage: Option<ExtraUsage>,
    ) -> Self {
        Self {
            windows,
            extra_usage,
        }
    }

    /// The window with the highest percent used, the first of equal ones;
    /// `None` when there is no window.
    ///
    /// ```
    /// use otty_agents::{LimitWindow, UsageWindows};
    ///
    /// let usage = UsageWindows::new(
    ///     vec![
    ///         LimitWindow::new("5 hours", 12.0, None),
    ///         LimitWindow::new("7 days", 42.0, None),
    ///     ],
    ///     None,
    /// );
    ///
    /// assert_eq!(usage.most_used().map(LimitWindow::name), Some("7 days"));
    /// ```
    pub fn most_used(&self) -> Option<&LimitWindow> {
        let mut most_used: Option<&LimitWindow> = None;

        for window in &self.windows {
            let is_more_used = most_used
                .is_none_or(|most| window.percent_used > most.percent_used);
            if is_more_used {
                most_used = Some(window);
            }
        }

        most_used
    }
}

/// One usage window, such as "5 hours" or "7 days".
#[derive(Debug, Clone, PartialEq)]
pub struct LimitWindow {
    name: String,
    percent_used: f64,
    resets_at: Option<SystemTime>,
}

impl LimitWindow {
    /// The name of the window shown to the user.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How much of the window is used, from 0 to 100.
    pub fn percent_used(&self) -> f64 {
        self.percent_used
    }

    /// When the window starts again, when known.
    pub fn resets_at(&self) -> Option<SystemTime> {
        self.resets_at
    }

    /// A window from its parts.
    pub fn new(
        name: impl Into<String>,
        percent_used: f64,
        resets_at: Option<SystemTime>,
    ) -> Self {
        Self {
            name: name.into(),
            percent_used,
            resets_at,
        }
    }
}

/// The extra usage a subscription may buy beyond its windows, in money.
///
/// The amounts are in whole units of the currency, such as dollars, not
/// cents.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtraUsage {
    spent: Option<f64>,
    limit: Option<f64>,
    currency: Option<String>,
}

impl ExtraUsage {
    /// How much was spent this month, when known.
    pub fn spent(&self) -> Option<f64> {
        self.spent
    }

    /// How much may be spent in a month, when known.
    pub fn limit(&self) -> Option<f64> {
        self.limit
    }

    /// The currency code of the amounts, such as `USD`, when known.
    pub fn currency(&self) -> Option<&str> {
        self.currency.as_deref()
    }

    /// Extra usage from its parts.
    ///
    /// ```
    /// use otty_agents::ExtraUsage;
    ///
    /// let extra =
    ///     ExtraUsage::new(Some(12.5), Some(50.0), Some(String::from("USD")));
    ///
    /// assert_eq!(extra.limit(), Some(50.0));
    /// ```
    pub fn new(
        spent: Option<f64>,
        limit: Option<f64>,
        currency: Option<String>,
    ) -> Self {
        Self {
            spent,
            limit,
            currency,
        }
    }
}

/// The credits of an account paid for by a pool of credits.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Credits {
    left: Option<f64>,
    used: Option<f64>,
    pool: Option<f64>,
    renews_at: Option<SystemTime>,
    is_unlimited: bool,
}

impl Credits {
    /// The credits left, when known.
    pub fn left(&self) -> Option<f64> {
        self.left
    }

    /// The credits used of the pool, when known.
    pub fn used(&self) -> Option<f64> {
        self.used
    }

    /// The size of the pool, when known.
    pub fn pool(&self) -> Option<f64> {
        self.pool
    }

    /// When the pool renews, when known.
    pub fn renews_at(&self) -> Option<SystemTime> {
        self.renews_at
    }

    /// Whether the account has no limit on credits.
    pub fn is_unlimited(&self) -> bool {
        self.is_unlimited
    }

    /// Credits from the numbers the source gave. A source that tells only
    /// the balance gives no "used" and no pool: nothing is guessed.
    ///
    /// ```
    /// use otty_agents::Credits;
    ///
    /// let credits = Credits::new(Some(3760.0), None, None, None);
    ///
    /// assert_eq!(credits.left(), Some(3760.0));
    /// assert_eq!(credits.pool(), None);
    /// ```
    pub fn new(
        left: Option<f64>,
        used: Option<f64>,
        pool: Option<f64>,
        renews_at: Option<SystemTime>,
    ) -> Self {
        Self {
            left,
            used,
            pool,
            renews_at,
            is_unlimited: false,
        }
    }

    /// Credits with no limit.
    pub fn unlimited() -> Self {
        Self {
            is_unlimited: true,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(windows: &[(&str, f64)]) -> UsageWindows {
        let windows = windows
            .iter()
            .map(|(name, percent)| LimitWindow::new(*name, *percent, None))
            .collect();

        UsageWindows::new(windows, None)
    }

    fn most_used_name(usage: &UsageWindows) -> Option<&str> {
        usage.most_used().map(LimitWindow::name)
    }

    #[test]
    fn the_most_used_window_has_the_highest_percent() {
        let usage =
            usage(&[("5 hours", 42.0), ("7 days", 87.5), ("Opus", 3.0)]);

        assert_eq!(most_used_name(&usage), Some("7 days"));
    }

    #[test]
    fn of_equal_windows_the_first_is_the_most_used() {
        let usage = usage(&[("5 hours", 42.0), ("7 days", 42.0)]);

        assert_eq!(most_used_name(&usage), Some("5 hours"));
    }

    #[test]
    fn no_window_has_no_most_used_one() {
        let extra = ExtraUsage::new(Some(12.5), Some(50.0), None);
        let usage = UsageWindows::new(Vec::new(), Some(extra));

        assert_eq!(usage.most_used(), None);
    }

    #[test]
    fn each_billing_tells_its_kind() {
        let credits = Billing::Credits(Credits::unlimited());
        let windows = Billing::UsageWindows(UsageWindows::default());

        assert_eq!(windows.kind(), BillingKind::UsageWindows);
        assert_eq!(credits.kind(), BillingKind::Credits);
        assert_eq!(Billing::ApiKey.kind(), BillingKind::ApiKey);
    }
}
