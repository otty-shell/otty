//! Limits: how much of an account's usage is spent, and the sources that
//! read them.

use crate::billing::Billing;
use crate::error::SourceError;
use crate::profile::Profile;

/// What an account may still use: the plan and the billing with its
/// numbers (spec Decision 9).
#[derive(Debug, Clone, PartialEq)]
pub struct Limits {
    billing: Billing,
    plan: Option<String>,
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
        Self { billing, plan }
    }
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
