//! Spend: how many tokens a profile used. Tokens only, no money.

use crate::error::SourceError;
use crate::profile::Profile;

/// The tokens a profile spent, today and for the last 30 days.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spend {
    today: TokenCount,
    last_30_days: TokenCount,
}

impl Spend {
    /// The tokens spent today.
    pub fn today(&self) -> TokenCount {
        self.today
    }

    /// The tokens spent in the last 30 days, today included.
    pub fn last_30_days(&self) -> TokenCount {
        self.last_30_days
    }

    /// Spend from its parts.
    ///
    /// ```
    /// use otty_agents::{Spend, TokenCount};
    ///
    /// let spend = Spend::new(TokenCount::new(10, 2), TokenCount::new(300, 40));
    ///
    /// assert_eq!(spend.today().output(), 2);
    /// ```
    pub fn new(today: TokenCount, last_30_days: TokenCount) -> Self {
        Self {
            today,
            last_30_days,
        }
    }
}

/// A number of tokens sent to the model and received from it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenCount {
    input: u64,
    output: u64,
}

impl TokenCount {
    /// The tokens sent to the model.
    pub fn input(&self) -> u64 {
        self.input
    }

    /// The tokens the model answered with.
    pub fn output(&self) -> u64 {
        self.output
    }

    /// A count from its parts.
    pub fn new(input: u64, output: u64) -> Self {
        Self { input, output }
    }
}

/// Counts the tokens a profile spent from the agent's local files.
///
/// It is a trait because each agent writes its numbers its own way, and
/// counting them reads files.
#[cfg_attr(test, mockall::automock)]
pub trait SpendCounter: Send {
    /// A short name of the counter, shown with its numbers.
    fn name(&self) -> &'static str;

    /// The tokens the profile spent.
    fn spend(&self, profile: &Profile) -> Result<Spend, SourceError>;
}
