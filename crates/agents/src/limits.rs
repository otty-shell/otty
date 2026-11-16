//! Limits: how much of an account's usage is spent.

use std::time::SystemTime;

use crate::error::SourceError;
use crate::profile::Profile;

/// What an account may still use. Every part may be missing: an account
/// that pays per use has no windows at all.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Limits {
    windows: Vec<LimitWindow>,
    credits: Option<Credits>,
    plan: Option<String>,
}

impl Limits {
    /// The usage windows, such as the five-hour and the weekly one.
    pub fn windows(&self) -> &[LimitWindow] {
        &self.windows
    }

    /// The credits of the account, when it has any.
    pub fn credits(&self) -> Option<Credits> {
        self.credits
    }

    /// The name of the account's plan, when known.
    pub fn plan(&self) -> Option<&str> {
        self.plan.as_deref()
    }

    /// Limits from their parts.
    ///
    /// ```
    /// use otty_agents::{Credits, Limits};
    ///
    /// let limits = Limits::new(Vec::new(), Some(Credits::Unlimited), None);
    ///
    /// assert!(limits.windows().is_empty());
    /// assert_eq!(limits.credits(), Some(Credits::Unlimited));
    /// ```
    pub fn new(
        windows: Vec<LimitWindow>,
        credits: Option<Credits>,
        plan: Option<String>,
    ) -> Self {
        Self {
            windows,
            credits,
            plan,
        }
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

/// The credits of an account.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Credits {
    /// The credits left.
    Balance(f64),
    /// The account has no limit on credits.
    Unlimited,
}

/// One way to read the limits of a profile, such as "ask the agent's
/// server".
///
/// It is a trait because each agent has its own ways, and each way crosses
/// into the network, a program or files.
#[cfg_attr(test, mockall::automock)]
pub trait LimitSource: Send {
    /// A short name of the source, shown when it answered or failed.
    fn name(&self) -> &'static str;

    /// The limits of the profile.
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError>;
}
