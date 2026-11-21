//! The individual limit of a Codex team or enterprise workspace: the
//! credits a member may spend in a month, how many are used and when the
//! month starts again.
//!
//! `codex app-server` and the session records tell it in the same shape,
//! so both read it here and give the same numbers. The field names, the
//! numbers sent as text and the reckoning follow CodexBar (MIT licence):
//! `SpendControlLimitSnapshot` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>,
//! `RPCSpendControlLimitSnapshot` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/UsageFetcher.swift>
//! and
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexSpendControlLimitMapping.swift>.
//! CodexBar treats these numbers as credits, not money, and so does OTTY.

use otty_plugin_sdk::agents::Credits;
use serde_json::Value;

use crate::rate_limits::codex_number;

/// The individual limit inside `part`: its `individual_limit`, or its
/// `individualLimit` as `codex app-server` spells it. Only an object
/// counts; `null` or any other value is no limit.
pub(crate) fn individual_limit_in(part: &Value) -> Option<&Value> {
    ["individual_limit", "individualLimit"]
        .iter()
        .filter_map(|field| part.get(*field))
        .find(|limit| limit.is_object())
}

/// The credits of an individual limit: the pool is its `limit`, the used
/// part its `used` and what is left the rest of the pool; the pool renews
/// at `resets_at`. `None` when the limit is missing or not above zero.
///
/// With no `used`, the used part is counted from `remaining_percent`, and
/// with neither it is zero, as CodexBar does. Every number may come as
/// text, and each field may be spelled in camel case.
pub(crate) fn pool_credits(limit: &Value) -> Option<Credits> {
    let pool = limit.get("limit").and_then(codex_number)?;
    if pool <= 0.0 {
        return None;
    }

    let used = limit.get("used").and_then(codex_number);
    let percent_left =
        first_field(limit, &["remaining_percent", "remainingPercent"]);
    let used = match (used, percent_left) {
        (Some(used), _) => used,
        (None, Some(percent_left)) => {
            pool * (100.0 - percent_left).clamp(0.0, 100.0) / 100.0
        },
        (None, None) => 0.0,
    };
    let renews_at_ms =
        first_field(limit, &["resets_at", "resetsAt", "reset_at"])
            .filter(|seconds| *seconds > 0.0)
            .map(|seconds| seconds.trunc() as i64 * 1000);

    Some(Credits {
        left: Some((pool - used).max(0.0)),
        used: Some(used),
        pool: Some(pool),
        renews_at_ms,
        is_unlimited: false,
    })
}

/// The number of the first of `fields` that `limit` has as a number.
fn first_field(limit: &Value, fields: &[&str]) -> Option<f64> {
    fields
        .iter()
        .find_map(|field| limit.get(*field).and_then(codex_number))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn credits(
        left: f64,
        used: f64,
        pool: f64,
        renews_at_ms: Option<i64>,
    ) -> Credits {
        Credits {
            left: Some(left),
            used: Some(used),
            pool: Some(pool),
            renews_at_ms,
            is_unlimited: false,
        }
    }

    #[test]
    fn a_limit_with_its_numbers_is_a_pool_with_a_used_part() {
        let limit = json!({
            "limit": 100000,
            "used": 7761,
            "remaining_percent": 92.239,
            "resets_at": 1782864000
        });

        assert_eq!(
            pool_credits(&limit),
            Some(credits(
                92_239.0,
                7761.0,
                100_000.0,
                Some(1_782_864_000_000)
            ))
        );
    }

    #[test]
    fn numbers_may_come_as_text_and_the_reset_as_reset_at() {
        let limit = json!({
            "source": "workspace_spend_controls",
            "limit": "1000",
            "used": " 36.5 ",
            "remaining": "963.5",
            "remaining_percent": 96,
            "reset_at": "1788220800"
        });

        assert_eq!(
            pool_credits(&limit),
            Some(credits(963.5, 36.5, 1000.0, Some(1_788_220_800_000)))
        );
    }

    #[test]
    fn the_camel_case_fields_of_app_server_are_read() {
        let limit = json!({
            "limit": 500,
            "used": 100,
            "remainingPercent": 80,
            "resetsAt": 1782864000
        });

        assert_eq!(
            pool_credits(&limit),
            Some(credits(400.0, 100.0, 500.0, Some(1_782_864_000_000)))
        );
    }

    #[test]
    fn with_no_used_part_it_is_counted_from_the_percent_left() {
        let from_percent = json!({"limit": 1000, "remaining_percent": 75});
        let neither = json!({"limit": 1000});

        assert_eq!(
            pool_credits(&from_percent),
            Some(credits(750.0, 250.0, 1000.0, None))
        );
        assert_eq!(
            pool_credits(&neither),
            Some(credits(1000.0, 0.0, 1000.0, None))
        );
    }

    #[test]
    fn more_used_than_the_pool_leaves_nothing() {
        let limit = json!({"limit": 1000, "used": 1200, "resets_at": 0});

        assert_eq!(
            pool_credits(&limit),
            Some(credits(0.0, 1200.0, 1000.0, None))
        );
    }

    #[test]
    fn a_missing_zero_or_unreadable_limit_is_no_pool() {
        for limit in [
            json!({"used": 10}),
            json!({"limit": 0, "used": 10}),
            json!({"limit": "a lot"}),
            json!({"limit": "NaN"}),
            json!({"limit": null}),
        ] {
            assert_eq!(pool_credits(&limit), None, "{limit}");
        }
    }

    #[test]
    fn only_an_object_is_an_individual_limit_in_either_spelling() {
        let snake = json!({"individual_limit": {"limit": 1}});
        let camel = json!({"individualLimit": {"limit": 2}});
        let null_then_camel =
            json!({"individual_limit": null, "individualLimit": {"limit": 3}});
        let not_an_object = json!({"individual_limit": 7});

        assert_eq!(individual_limit_in(&snake), Some(&json!({"limit": 1})));
        assert_eq!(individual_limit_in(&camel), Some(&json!({"limit": 2})));
        assert_eq!(
            individual_limit_in(&null_then_camel),
            Some(&json!({"limit": 3}))
        );
        assert_eq!(individual_limit_in(&not_an_object), None);
        assert_eq!(individual_limit_in(&json!(null)), None);
    }
}
