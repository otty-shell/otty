//! The limit numbers each Codex source tells in its own words: the usage
//! windows, the credits and the plan. All three sources turn them into
//! [`Limits`] by the same rule (spec Decision 9): windows when there are
//! any, else the credits, else the answer is unexpected.

use std::time::{Duration, SystemTime};

use serde_json::Value;

use super::individual_limit::pool_credits;
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

/// The credits: the pool of the individual limit of a workspace, else
/// unlimited, else the balance of an account that has credits.
///
/// The individual limit wins over the rest, as CodexBar shows it even next
/// to a balance of zero: it has the pool, the used part and the renewal.
/// The balance comes as a number or as text; a usage-based plan sends
/// none, so its credits are only available. A balance tells no pool, so
/// nothing is known of the credits used. An account with no credits has
/// none, whatever its balance says.
pub(super) fn codex_credits(
    unlimited: Option<&Value>,
    has_credits: Option<&Value>,
    balance: Option<&Value>,
    individual_limit: Option<&Value>,
) -> Option<Credits> {
    if let Some(pool) = individual_limit.and_then(pool_credits) {
        return Some(pool);
    }
    if unlimited.and_then(Value::as_bool) == Some(true) {
        return Some(Credits::unlimited());
    }
    if has_credits.and_then(Value::as_bool) != Some(true) {
        return None;
    }

    let balance = balance.and_then(codex_number);

    match balance {
        Some(balance) => Some(Credits::new(Some(balance), None, None, None)),
        None => Some(Credits::available()),
    }
}

/// A number Codex sends as a JSON number or as text, such as `12.5` or
/// `" 12.5 "`; `None` for anything else, or for a number that is not
/// finite.
pub(super) fn codex_number(value: &Value) -> Option<f64> {
    let number = match value.as_str() {
        Some(text) => text.trim().parse().ok(),
        None => value.as_f64(),
    };

    number.filter(|number: &f64| number.is_finite())
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

/// The plan as Codex shows it in `/status`, with nothing added: "Enterprise"
/// for `enterprise_cbp_usage_based`, else the code in words, such as "Plus"
/// for `plus` or "Free workspace" for `free_workspace`.
fn plan_name(plan_type: &str) -> Option<String> {
    let plan_type = plan_type.trim();
    let known = match plan_type {
        "enterprise_cbp_usage_based" => Some("Enterprise"),
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
    use serde_json::json;

    use super::*;

    /// Owner, issue 33: a plan is named as Codex names it in `/status`,
    /// with nothing added; a code Codex shows under another name is
    /// written as its code in words.
    #[test]
    fn a_plan_is_named_as_codex_shows_it_with_nothing_added() {
        let cases = [
            ("enterprise_cbp_usage_based", "Enterprise"),
            ("enterprise", "Enterprise"),
            ("plus", "Plus"),
            ("edu", "Edu"),
            ("free", "Free"),
            ("prolite", "Prolite"),
            ("pro", "Pro"),
            ("team", "Team"),
            ("business", "Business"),
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
    fn a_number_may_come_as_text() {
        assert_eq!(codex_number(&json!(12.5)), Some(12.5));
        assert_eq!(codex_number(&json!(7761)), Some(7761.0));
        assert_eq!(codex_number(&json!(" 36.5 ")), Some(36.5));
        assert_eq!(codex_number(&json!("-4")), Some(-4.0));
    }

    #[test]
    fn text_that_is_no_finite_number_and_other_values_are_none() {
        for value in [
            json!("a lot"),
            json!("NaN"),
            json!("Infinity"),
            json!(""),
            json!(null),
            json!(true),
            json!({"amount": 14}),
        ] {
            assert_eq!(codex_number(&value), None, "{value}");
        }
    }

    #[test]
    fn an_individual_limit_gives_the_pool_whatever_the_balance_says() {
        let limit =
            json!({"limit": 100000, "used": "7761", "resets_at": 1782864000});
        let pool = Credits::new(
            Some(92_239.0),
            Some(7761.0),
            Some(100_000.0),
            time_from_seconds(1_782_864_000),
        );

        let zero_balance = codex_credits(
            Some(&json!(false)),
            Some(&json!(true)),
            Some(&json!("0")),
            Some(&limit),
        );
        let no_credits_part = codex_credits(None, None, None, Some(&limit));

        assert_eq!(zero_balance, Some(pool));
        assert_eq!(no_credits_part, Some(pool));
    }

    #[test]
    fn an_individual_limit_with_no_pool_leaves_the_credits_as_told() {
        let no_pool = json!({"limit": 0});

        let credits = codex_credits(
            Some(&json!(false)),
            Some(&json!(true)),
            Some(&json!(null)),
            Some(&no_pool),
        );

        assert_eq!(credits, Some(Credits::available()));
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
