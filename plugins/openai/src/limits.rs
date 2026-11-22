//! What a ChatGPT login may still use, as the plugin reads it in any of
//! its ways: from `chatgpt.com`, from `codex app-server`, or from the last
//! record of the session files. The provider part `openai` answers it to
//! OTTY as the numbers of the account.

use otty_plugin_sdk::agents::{AccountNumbers, Credits, LimitWindow};

/// The windows, the credits and the plan of a login.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Limits {
    pub(crate) windows: Vec<LimitWindow>,
    pub(crate) credits: Option<Credits>,
    /// The name of the plan, when known.
    pub(crate) plan: Option<String>,
    /// When the numbers were true, for numbers read from a session file;
    /// none when they are true now.
    pub(crate) recorded_at_ms: Option<i64>,
}

impl Limits {
    /// The limits as the numbers of the account.
    pub(crate) fn into_numbers(self) -> AccountNumbers {
        AccountNumbers {
            windows: self.windows,
            credits: self.credits,
            extra_usage: None,
            left: None,
            key_limit: None,
            spent_this_month: None,
            plan: self.plan,
            recorded_at_ms: self.recorded_at_ms,
        }
    }
}
