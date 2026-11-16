//! The limit numbers each Codex source tells in its own words: the usage
//! windows, the credits and the plan. All three sources turn them into
//! [`Limits`] by the same rule (spec Decision 9): windows when there are
//! any, else the credits, else the answer is unexpected.

use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::billing::{Billing, Credits, LimitWindow, UsageWindows};
use crate::error::SourceError;
use crate::limits::Limits;

/// One usage window as a source tells it. A plain parameter bag.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct WindowNumbers {
    /// How much is used, from 0 to 100.
    pub(super) percent_used: f64,
    /// The length of the window in seconds, when known.
    pub(super) seconds: Option<u64>,
    /// When the window starts again, when known.
    pub(super) resets_at: Option<SystemTime>,
}

/// The limits from the windows, the credits and the plan type a source
/// told.
///
/// The windows come shorter first, each named from its length. When there
/// is a window, the credits are dropped; with no window the credits are
/// the billing; with neither the answer is unexpected.
pub(super) fn codex_limits(
    mut windows: Vec<WindowNumbers>,
    credits: Option<Credits>,
    plan_type: Option<&str>,
) -> Result<Limits, SourceError> {
    windows.sort_by_key(|window| window.seconds.unwrap_or(u64::MAX));

    let windows: Vec<LimitWindow> = windows
        .into_iter()
        .map(|window| {
            let name = window_name(window.seconds);
            LimitWindow::new(name, window.percent_used, window.resets_at)
        })
        .collect();
    let plan = plan_type.and_then(plan_name);

    let billing = if !windows.is_empty() {
        Billing::UsageWindows(UsageWindows::new(windows, None))
    } else if let Some(credits) = credits {
        Billing::Credits(credits)
    } else {
        return Err(SourceError::UnexpectedAnswer(String::from(
            "no usage window and no credits in the answer",
        )));
    };

    Ok(Limits::new(billing, plan))
}

/// The credits: unlimited, or the balance of an account that has credits.
/// The balance comes as a number or as text; a usage-based plan sends
/// none, so its credits are only available. No source tells the size of
/// the pool, so nothing is known of the credits used. An account with no
/// credits has none, whatever its balance says.
pub(super) fn codex_credits(
    unlimited: Option<&Value>,
    has_credits: Option<&Value>,
    balance: Option<&Value>,
) -> Option<Credits> {
    if unlimited.and_then(Value::as_bool) == Some(true) {
        return Some(Credits::unlimited());
    }
    if has_credits.and_then(Value::as_bool) != Some(true) {
        return None;
    }

    let balance = balance.and_then(|balance| match balance.as_str() {
        Some(text) => text.trim().parse().ok(),
        None => balance.as_f64(),
    });

    match balance {
        Some(balance) => Some(Credits::new(Some(balance), None, None, None)),
        None => Some(Credits::available()),
    }
}

/// The time `seconds` after 1970, as Codex writes the reset of a window.
pub(super) fn time_from_seconds(seconds: u64) -> Option<SystemTime> {
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
}

/// The name of a window from its length, such as "5 hours" or "7 days".
fn window_name(seconds: Option<u64>) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    let (count, one, many) = match seconds {
        Some(seconds) if seconds > 0 && seconds % DAY == 0 => {
            (seconds / DAY, "day", "days")
        },
        Some(seconds) if seconds > 0 && seconds % HOUR == 0 => {
            (seconds / HOUR, "hour", "hours")
        },
        Some(seconds) if seconds >= MINUTE => (seconds / MINUTE, "min", "min"),
        _ => return String::from("Limit"),
    };

    let unit = if count == 1 { one } else { many };
    format!("{count} {unit}")
}

/// The plan as shown: the name of a plan type the sources give, such as
/// "Pro Lite" for `prolite`, else the code in words, such as "Free
/// workspace" for `free_workspace`.
fn plan_name(plan_type: &str) -> Option<String> {
    let plan_type = plan_type.trim();
    let known = match plan_type {
        "enterprise_cbp_usage_based" => Some("Enterprise, usage-based"),
        "prolite" => Some("Pro Lite"),
        "plus" => Some("Plus"),
        "pro" => Some("Pro"),
        "team" => Some("Team"),
        "business" => Some("Business"),
        "enterprise" => Some("Enterprise"),
        "edu" => Some("Edu"),
        "free" => Some("Free"),
        _ => None,
    };
    if let Some(name) = known {
        return Some(String::from(name));
    }

    let words = plan_type.replace('_', " ");
    let mut letters = words.chars();
    let first = letters.next()?;

    Some(format!("{}{}", first.to_uppercase(), letters.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_plans_read_like_words() {
        let cases = [
            ("enterprise_cbp_usage_based", "Enterprise, usage-based"),
            ("prolite", "Pro Lite"),
            ("plus", "Plus"),
            ("pro", "Pro"),
            ("team", "Team"),
            ("business", "Business"),
            ("enterprise", "Enterprise"),
            ("edu", "Edu"),
            ("free", "Free"),
        ];

        for (plan_type, name) in cases {
            assert_eq!(
                plan_name(plan_type).as_deref(),
                Some(name),
                "{plan_type}"
            );
        }
    }

    #[test]
    fn an_unknown_plan_is_its_code_in_words() {
        assert_eq!(
            plan_name("free_workspace").as_deref(),
            Some("Free workspace")
        );
        assert_eq!(plan_name(" k12 ").as_deref(), Some("K12"));
        assert_eq!(plan_name("  "), None);
    }
}
