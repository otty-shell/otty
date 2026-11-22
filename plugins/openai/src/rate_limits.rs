//! The limit numbers each Codex source tells in its own words: the usage
//! windows, the credits and the plan. `codex app-server` and the session
//! records turn them into [`Limits`] by the same rule: windows when there
//! are any, else the credits, else the answer has no limits.

use otty_plugin_sdk::agents::{Credits, LimitWindow, ReadError};
use serde_json::Value;

use crate::individual_limit::pool_credits;
use crate::limits::Limits;

/// One usage window as a source tells it. A plain parameter bag.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowNumbers {
    /// How much is used, from 0 to 100.
    pub(crate) percent_used: f64,
    /// The length of the window in minutes, when known.
    pub(crate) minutes: Option<u32>,
    /// When the window starts again, in milliseconds since the Unix
    /// epoch, when known.
    pub(crate) resets_at_ms: Option<i64>,
}

/// The limits from the windows, the credits and the plan type a source
/// told.
///
/// The windows come shorter first. When there is a window, the credits
/// are dropped; with no window the credits are the limits; with neither
/// the answer is unexpected.
pub(crate) fn codex_limits(
    mut windows: Vec<WindowNumbers>,
    credits: Option<Credits>,
    plan_type: Option<&str>,
) -> Result<Limits, ReadError> {
    windows.sort_by_key(|window| window.minutes.unwrap_or(u32::MAX));

    let windows: Vec<LimitWindow> = windows
        .into_iter()
        .map(|window| LimitWindow {
            length_minutes: window.minutes,
            model: None,
            percent_used: window.percent_used,
            resets_at_ms: window.resets_at_ms,
            reset_text: None,
        })
        .collect();
    let plan = plan_type.and_then(plan_name);

    let credits = if !windows.is_empty() {
        None
    } else if credits.is_some() {
        credits
    } else {
        return Err(ReadError::Unexpected(
            "no usage window and no credits".to_string(),
        ));
    };

    Ok(Limits {
        windows,
        credits,
        plan,
        recorded_at_ms: None,
    })
}

/// The credits: the pool of the individual limit of a workspace, else
/// unlimited, else the balance of an account that has credits.
///
/// The individual limit wins over the rest, as CodexBar shows it even next
/// to a balance of zero: it has the pool, the used part and the renewal.
/// The balance comes as a number or as text; a usage-based plan sends
/// none, so its credits are only there. A balance tells no pool, so
/// nothing is known of the credits used. An account with no credits has
/// none, whatever its balance says.
pub(crate) fn codex_credits(
    unlimited: Option<&Value>,
    has_credits: Option<&Value>,
    balance: Option<&Value>,
    individual_limit: Option<&Value>,
) -> Option<Credits> {
    if let Some(pool) = individual_limit.and_then(pool_credits) {
        return Some(pool);
    }
    if unlimited.and_then(Value::as_bool) == Some(true) {
        return Some(Credits {
            is_unlimited: true,
            ..no_numbers()
        });
    }
    if has_credits.and_then(Value::as_bool) != Some(true) {
        return None;
    }

    Some(Credits {
        left: balance.and_then(codex_number),
        ..no_numbers()
    })
}

/// Credits that are on, with no number told.
pub(crate) fn no_numbers() -> Credits {
    Credits {
        left: None,
        used: None,
        pool: None,
        renews_at_ms: None,
        is_unlimited: false,
    }
}

/// A number Codex sends as a JSON number or as text, such as `12.5` or
/// `" 12.5 "`; `None` for anything else, or for a number that is not
/// finite.
pub(crate) fn codex_number(value: &Value) -> Option<f64> {
    let number = match value.as_str() {
        Some(text) => text.trim().parse().ok(),
        None => value.as_f64(),
    };

    number.filter(|number: &f64| number.is_finite())
}

/// A time Codex sends in whole seconds since the Unix epoch, in
/// milliseconds; `None` for anything else.
pub(crate) fn millis_of_seconds(value: &Value) -> Option<i64> {
    let seconds = i64::try_from(value.as_u64()?).ok()?;

    seconds.checked_mul(1000)
}

/// The plan as Codex shows it in `/status`: the names of
/// `SubscriptionDisplay::label` with `Status` in
/// `codex-rs/tui/src/subscription.rs`, for the codes of `PlanType` in
/// `codex-rs/protocol/src/account.rs` (read on 2026-11-17), such as
/// "Pro 200" for `pro` or "Enterprise" for `business`. A code not in that
/// table is the code in words, such as "Free workspace" for
/// `free_workspace`.
pub(crate) fn plan_name(plan_type: &str) -> Option<String> {
    let plan_type = plan_type.trim();
    let known = match plan_type {
        "free" => Some("Free"),
        "go" => Some("Go"),
        "plus" => Some("Plus"),
        "pro" => Some("Pro 200"),
        "prolite" => Some("Pro 100"),
        "promax" => Some("Pro 500"),
        "team" | "self_serve_business_usage_based" => Some("Business"),
        "self_serve_business_prolite" => Some("Business Premium"),
        "enterprise_cbp_automation" => Some("Enterprise (Automation)"),
        "business" | "enterprise" | "ent26" | "enterprise_cbp_usage_based" => {
            Some("Enterprise")
        },
        "edu" => Some("Edu"),
        "edu_plus" => Some("Edu Plus"),
        "edu_pro" => Some("Edu Pro"),
        "sci" => Some("Academic Researchers"),
        "sci_plus" => Some("Academic Researchers Plus"),
        _ => None,
    };
    if let Some(name) = known {
        return Some(name.to_string());
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

    fn window(minutes: Option<u32>, percent_used: f64) -> WindowNumbers {
        WindowNumbers {
            percent_used,
            minutes,
            resets_at_ms: None,
        }
    }

    /// Owner, issue 34 of the old spec: a plan is named as Codex names it
    /// in `/status` (`SubscriptionDisplay::label` of
    /// `codex-rs/tui/src/subscription.rs`, read on 2026-11-17).
    #[test]
    fn every_plan_codex_knows_is_named_as_codex_shows_it() {
        let cases = [
            ("free", "Free"),
            ("go", "Go"),
            ("plus", "Plus"),
            ("pro", "Pro 200"),
            ("prolite", "Pro 100"),
            ("promax", "Pro 500"),
            ("team", "Business"),
            ("self_serve_business_usage_based", "Business"),
            ("business", "Enterprise"),
            ("self_serve_business_prolite", "Business Premium"),
            ("enterprise_cbp_automation", "Enterprise (Automation)"),
            ("enterprise", "Enterprise"),
            ("ent26", "Enterprise"),
            ("enterprise_cbp_usage_based", "Enterprise"),
            ("edu", "Edu"),
            ("edu_plus", "Edu Plus"),
            ("edu_pro", "Edu Pro"),
            ("sci", "Academic Researchers"),
            ("sci_plus", "Academic Researchers Plus"),
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
    fn the_shorter_window_comes_first_and_the_credits_are_dropped() {
        let credits = Some(Credits {
            left: Some(3.0),
            ..no_numbers()
        });

        let limits = codex_limits(
            vec![
                window(Some(10_080), 17.0),
                window(None, 1.0),
                window(Some(300), 42.0),
            ],
            credits,
            Some("plus"),
        )
        .expect("the limits");

        let lengths: Vec<Option<u32>> = limits
            .windows
            .iter()
            .map(|window| window.length_minutes)
            .collect();
        assert_eq!(lengths, [Some(300), Some(10_080), None]);
        assert_eq!(limits.credits, None);
        assert_eq!(limits.plan.as_deref(), Some("Plus"));
    }

    #[test]
    fn with_no_window_the_credits_are_the_limits() {
        let credits = Credits {
            is_unlimited: true,
            ..no_numbers()
        };

        let limits =
            codex_limits(Vec::new(), Some(credits), None).expect("the limits");

        assert_eq!(limits.credits, Some(credits));
        assert_eq!(limits.plan, None);
    }

    #[test]
    fn with_no_window_and_no_credits_there_are_no_limits() {
        let limits = codex_limits(Vec::new(), None, Some("plus"));

        assert!(matches!(limits, Err(ReadError::Unexpected(_))));
    }

    #[test]
    fn an_individual_limit_gives_the_pool_whatever_the_balance_says() {
        let limit =
            json!({"limit": 100000, "used": "7761", "resets_at": 1782864000});
        let pool = Credits {
            left: Some(92_239.0),
            used: Some(7761.0),
            pool: Some(100_000.0),
            renews_at_ms: Some(1_782_864_000_000),
            is_unlimited: false,
        };

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

        assert_eq!(credits, Some(no_numbers()));
    }

    #[test]
    fn the_three_shapes_of_credits() {
        let balance = codex_credits(
            Some(&json!(false)),
            Some(&json!(true)),
            Some(&json!("12.5")),
            None,
        );
        let unlimited =
            codex_credits(Some(&json!(true)), Some(&json!(true)), None, None);
        let none =
            codex_credits(None, Some(&json!(false)), Some(&json!("0")), None);

        assert_eq!(
            balance,
            Some(Credits {
                left: Some(12.5),
                ..no_numbers()
            })
        );
        assert_eq!(
            unlimited,
            Some(Credits {
                is_unlimited: true,
                ..no_numbers()
            })
        );
        assert_eq!(none, None);
    }

    #[test]
    fn a_time_in_seconds_is_kept_in_milliseconds() {
        assert_eq!(
            millis_of_seconds(&json!(1_794_826_800)),
            Some(1_794_826_800_000)
        );
        assert_eq!(millis_of_seconds(&json!(-5)), None);
        assert_eq!(millis_of_seconds(&json!("1794826800")), None);
    }
}
