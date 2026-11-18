//! Limits: how much of an account's usage is spent, and the sources that
//! read them. The [`Billing`] of the limits says how the account is paid
//! for (spec Decision 9), with the numbers its source gave.

use std::time::{Duration, SystemTime};

use crate::error::SourceError;
use crate::percent::Percent;
use crate::profile::Profile;
use crate::source_id::SourceId;

/// Seconds in an hour.
const SECONDS_IN_HOUR: u64 = 60 * 60;
/// Seconds in a day.
const SECONDS_IN_DAY: u64 = 24 * SECONDS_IN_HOUR;

/// What an account may still use: the plan and the billing with its
/// numbers (spec Decision 9).
#[derive(Debug, Clone, PartialEq)]
pub struct Limits {
    billing: Billing,
    plan: Option<String>,
    recorded_at: Option<SystemTime>,
}

impl Limits {
    /// How the account is paid for, with the numbers the source gave.
    pub fn billing(&self) -> &Billing {
        &self.billing
    }

    /// The name of the account's plan, when known.
    pub fn plan(&self) -> Option<&str> {
        self.plan.as_deref()
    }

    /// When the numbers were true, for numbers older than the read: the
    /// time of the record in a session file they came from. `None` when
    /// the source told them as they are now.
    pub fn recorded_at(&self) -> Option<SystemTime> {
        self.recorded_at
    }

    /// Limits from their parts.
    ///
    /// ```
    /// use otty_agents::{Billing, Credits, Limits};
    ///
    /// let limits = Limits::new(
    ///     Billing::Credits(Credits::unlimited()),
    ///     Some(String::from("Enterprise")),
    /// );
    ///
    /// assert_eq!(limits.plan(), Some("Enterprise"));
    /// ```
    pub fn new(billing: Billing, plan: Option<String>) -> Self {
        Self {
            billing,
            plan,
            recorded_at: None,
        }
    }

    /// The same limits, recorded at `time` rather than told now.
    ///
    /// ```
    /// use std::time::SystemTime;
    ///
    /// use otty_agents::{Billing, Limits};
    ///
    /// let recorded_at = SystemTime::UNIX_EPOCH;
    /// let limits =
    ///     Limits::new(Billing::ApiKey, None).with_recorded_at(recorded_at);
    ///
    /// assert_eq!(limits.recorded_at(), Some(recorded_at));
    /// ```
    pub fn with_recorded_at(mut self, time: SystemTime) -> Self {
        self.recorded_at = Some(time);
        self
    }
}

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
    /// use otty_agents::{LimitWindow, UsageWindows, WindowId};
    ///
    /// let window = LimitWindow::new(WindowId::hours(5), 42.0, None);
    /// let usage = UsageWindows::new(vec![window], None);
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
    /// use otty_agents::{LimitWindow, UsageWindows, WindowId};
    ///
    /// let usage = UsageWindows::new(
    ///     vec![
    ///         LimitWindow::new(WindowId::hours(5), 12.0, None),
    ///         LimitWindow::new(WindowId::days(7), 42.0, None),
    ///     ],
    ///     None,
    /// );
    ///
    /// let most_used = usage.most_used().map(LimitWindow::id);
    /// assert_eq!(most_used, Some(&WindowId::days(7)));
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

/// Which usage window it is: its length and, for a window that counts one
/// model alone, the model. It is data, not text: the app names the window
/// on screen, such as "5 hours" or "7 days · Opus".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowId {
    length: Option<Duration>,
    model: Option<String>,
}

impl WindowId {
    /// How long the window is, such as five hours; `None` when the source
    /// did not tell it.
    pub fn length(&self) -> Option<Duration> {
        self.length
    }

    /// The model the window counts alone, as the agent names it, such as
    /// "Opus"; `None` for a window of every model.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// A window of every model that is `length` long, or of a length the
    /// source did not tell.
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use otty_agents::WindowId;
    ///
    /// let window = WindowId::new(Some(Duration::from_secs(5 * 3600)));
    ///
    /// assert_eq!(window, WindowId::hours(5));
    /// assert_eq!(WindowId::new(None).length(), None);
    /// ```
    pub fn new(length: Option<Duration>) -> Self {
        Self {
            length,
            model: None,
        }
    }

    /// A window of every model that is `hours` hours long.
    pub fn hours(hours: u64) -> Self {
        Self::new(Some(Duration::from_secs(hours * SECONDS_IN_HOUR)))
    }

    /// A window of every model that is `days` days long.
    pub fn days(days: u64) -> Self {
        Self::new(Some(Duration::from_secs(days * SECONDS_IN_DAY)))
    }

    /// The same window, counting `model` alone.
    ///
    /// ```
    /// use otty_agents::WindowId;
    ///
    /// let window = WindowId::days(7).for_model("Opus");
    ///
    /// assert_eq!(window.model(), Some("Opus"));
    /// ```
    pub fn for_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// One usage window, such as the five-hour or the weekly one.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitWindow {
    id: WindowId,
    percent_used: Percent,
    resets_at: Option<SystemTime>,
    reset_text: Option<String>,
}

impl LimitWindow {
    /// Which window it is.
    pub fn id(&self) -> &WindowId {
        &self.id
    }

    /// How much of the window is used, from 0 to 100; a source may tell
    /// more than 100 for a window used past its limit.
    pub fn percent_used(&self) -> Percent {
        self.percent_used
    }

    /// When the window starts again, when known.
    pub fn resets_at(&self) -> Option<SystemTime> {
        self.resets_at
    }

    /// When the window starts again as the agent wrote it, such as
    /// "Resets 5pm (Europe/Berlin)", for a source that gives no time.
    pub fn reset_text(&self) -> Option<&str> {
        self.reset_text.as_deref()
    }

    /// A window from its parts: `percent_used` is the percent as the
    /// source told it.
    pub fn new(
        id: WindowId,
        percent_used: f64,
        resets_at: Option<SystemTime>,
    ) -> Self {
        Self {
            id,
            percent_used: Percent::new(percent_used),
            resets_at,
            reset_text: None,
        }
    }

    /// The same window with the agent's own text of when it starts again.
    ///
    /// ```
    /// use otty_agents::{LimitWindow, WindowId};
    ///
    /// let window = LimitWindow::new(WindowId::hours(5), 42.0, None)
    ///     .with_reset_text("Resets 5pm (Europe/Berlin)");
    ///
    /// assert_eq!(window.reset_text(), Some("Resets 5pm (Europe/Berlin)"));
    /// ```
    pub fn with_reset_text(mut self, text: impl Into<String>) -> Self {
        self.reset_text = Some(text.into());
        self
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

/// The credits of an account paid for by a pool of credits. A value of it
/// always means the credits are on, even when no number came with them.
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

    /// Credits that are on, of a plan that tells no number of them, such as
    /// a usage-based Codex plan: nothing is guessed (spec Decision 9).
    ///
    /// ```
    /// use otty_agents::Credits;
    ///
    /// let credits = Credits::available();
    ///
    /// assert!(credits.is_only_available());
    /// assert_eq!(credits.left(), None);
    /// ```
    pub fn available() -> Self {
        Self::default()
    }

    /// Whether the source told only that the credits are on: no balance,
    /// no credits used, no pool and no "unlimited".
    pub fn is_only_available(&self) -> bool {
        self.left.is_none()
            && self.used.is_none()
            && self.pool.is_none()
            && !self.is_unlimited
    }
}

/// One way to read the limits of a profile, such as "ask the agent's
/// server".
///
/// It is a trait because each agent has its own ways, and each way crosses
/// into the network, a program or files.
#[cfg_attr(test, mockall::automock)]
pub trait LimitSource: Send {
    /// The id of the source, named on screen when it answered or failed.
    fn id(&self) -> SourceId;

    /// The limits of the profile.
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(windows: &[(WindowId, f64)]) -> UsageWindows {
        let windows = windows
            .iter()
            .map(|(id, percent)| LimitWindow::new(id.clone(), *percent, None))
            .collect();

        UsageWindows::new(windows, None)
    }

    fn most_used_id(usage: &UsageWindows) -> Option<&WindowId> {
        usage.most_used().map(LimitWindow::id)
    }

    #[test]
    fn the_most_used_window_has_the_highest_percent() {
        let usage = usage(&[
            (WindowId::hours(5), 42.0),
            (WindowId::days(7), 87.5),
            (WindowId::days(7).for_model("Opus"), 3.0),
        ]);

        assert_eq!(most_used_id(&usage), Some(&WindowId::days(7)));
    }

    #[test]
    fn of_equal_windows_the_first_is_the_most_used() {
        let usage =
            usage(&[(WindowId::hours(5), 42.0), (WindowId::days(7), 42.0)]);

        assert_eq!(most_used_id(&usage), Some(&WindowId::hours(5)));
    }

    #[test]
    fn a_window_is_told_by_its_length_and_its_model() {
        let five_hours = WindowId::new(Some(Duration::from_secs(18_000)));
        let week = WindowId::new(Some(Duration::from_secs(604_800)));

        assert_eq!(five_hours, WindowId::hours(5));
        assert_eq!(week, WindowId::days(7));
        assert_ne!(week, WindowId::days(7).for_model("Opus"));
        assert_eq!(WindowId::new(None).model(), None);
    }

    #[test]
    fn no_window_has_no_most_used_one() {
        let extra = ExtraUsage::new(Some(12.5), Some(50.0), None);
        let usage = UsageWindows::new(Vec::new(), Some(extra));

        assert_eq!(usage.most_used(), None);
    }

    #[test]
    fn credits_with_no_number_are_only_available() {
        let available = Credits::available();

        assert!(available.is_only_available());
        assert_eq!(available, Credits::new(None, None, None, None));
        assert_eq!(available.left(), None);
        assert!(!available.is_unlimited());
    }

    #[test]
    fn a_balance_a_pool_or_no_limit_is_more_than_available() {
        let balance = Credits::new(Some(3760.0), None, None, None);
        let pool = Credits::new(None, Some(1240.0), Some(5000.0), None);
        let zero = Credits::new(Some(0.0), None, None, None);

        assert!(!balance.is_only_available());
        assert!(!pool.is_only_available());
        assert!(!zero.is_only_available());
        assert!(!Credits::unlimited().is_only_available());
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
