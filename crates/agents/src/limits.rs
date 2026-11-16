//! Limits: how much of an account's usage is spent, and the sources that
//! read them.

use std::time::SystemTime;

use crate::billing::Billing;
use crate::error::SourceError;
use crate::profile::Profile;
use crate::source_id::SourceId;

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
