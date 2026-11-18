//! The reading of the answer of Claude's usage request: its windows and its
//! extra usage.
//!
//! The fields follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift>,
//! and for the `limits` list `ClaudeScopedWeeklyLimitMapper.swift` and
//! `ClaudeWeb/ClaudeWebExtraRateWindowParser.swift` next to it. The answer
//! is not documented by Anthropic, so it is read tolerantly.

use serde_json::{Map, Value};

use super::window_id::{five_hours, model_week, week};
use crate::error::{SourceError, Unexpected};
use crate::limits::{
    Billing, ExtraUsage, LimitWindow, Limits, UsageWindows, WindowId,
};
use crate::timestamp::parse_rfc3339;

/// The start of the key of a weekly window, such as `seven_day_opus`.
const WEEK_KEY: &str = "seven_day_";
/// The weekly window of the apps signed in with Claude, read as the weekly
/// window when the answer has no `seven_day`, as CodexBar does.
const APPS_WEEK_KEY: &str = "seven_day_oauth_apps";
/// The models whose weekly key comes before the others, in this order, so
/// that the windows OTTY showed first keep their place.
const FIRST_MODELS: [&str; 2] = ["opus", "sonnet"];
/// The ends of `seven_day_` keys that are not the week of a model:
/// CodexBar reads the apps signed in with Claude as a fallback of the week,
/// and the routines and Cowork as a "Daily Routines" window, which OTTY
/// does not show.
const NOT_MODELS: [&str; 4] =
    ["oauth_apps", "routines", "claude_routines", "cowork"];
/// The kind of an entry of `limits` that is the week of one model.
const SCOPED_KIND: &str = "weekly_scoped";
/// The group of an entry of `limits` that is the week of one model.
const SCOPED_GROUP: &str = "weekly";
/// The scope of an entry of `limits` that counts every model, already
/// shown as the weekly window.
const ALL_MODELS: &str = "all-models";
/// Cents in one unit of the currency: the answer counts extra usage in
/// cents.
const CENTS: f64 = 100.0;

/// The limits in the usage answer: the five-hour window, the weekly one,
/// the weekly window of each model and the extra usage.
///
/// The week of a model comes from a `seven_day_<model>` key, Opus and
/// Sonnet first, or from a `weekly_scoped` entry of the `limits` list; a
/// model named by both is one window. A window that is missing, `null` or
/// of another shape is left out; an answer with no window and no extra
/// usage is unexpected, as in CodexBar.
pub(super) fn limits_from(
    answer: &Value,
    plan: Option<String>,
) -> Result<Limits, SourceError> {
    let no_fields = Map::new();
    let fields = answer.as_object().unwrap_or(&no_fields);

    let week = fields
        .get("seven_day")
        .and_then(|window| usage_window(window, week()))
        .or_else(|| usage_window(fields.get(APPS_WEEK_KEY)?, week()));
    let mut windows: Vec<LimitWindow> = fields
        .get("five_hour")
        .and_then(|window| usage_window(window, five_hours()))
        .into_iter()
        .chain(week)
        .collect();
    for window in model_weeks(fields).into_iter().chain(scoped_weeks(fields)) {
        let is_new = windows
            .iter()
            .all(|known| !is_same_window(known.id(), window.id()));
        if is_new {
            windows.push(window);
        }
    }
    let extra_usage = fields.get("extra_usage").and_then(extra_usage);

    if windows.is_empty() && extra_usage.is_none() {
        return Err(SourceError::UnexpectedAnswer(Unexpected::NoUsageWindow));
    }

    let usage = UsageWindows::new(windows, extra_usage);
    Ok(Limits::new(Billing::UsageWindows(usage), plan))
}

/// The weekly windows of the `seven_day_<model>` keys: Opus and Sonnet
/// first, then the other models in the order of their keys. The model is
/// the end of the key, its words split at `_`.
fn model_weeks(fields: &Map<String, Value>) -> Vec<LimitWindow> {
    let mut models: Vec<&str> = fields
        .keys()
        .filter_map(|key| key.strip_prefix(WEEK_KEY))
        .filter(|model| !model.is_empty() && !NOT_MODELS.contains(model))
        .collect();
    models.sort_by_key(|model| {
        let first = FIRST_MODELS.iter().position(|first| first == model);
        (first.unwrap_or(FIRST_MODELS.len()), *model)
    });

    models
        .into_iter()
        .filter_map(|model| {
            let window = fields.get(&format!("{WEEK_KEY}{model}"))?;
            usage_window(window, model_week(&model.replace('_', " ")))
        })
        .collect()
}

/// The weekly windows of single models in the `limits` list: an entry of
/// the kind `weekly_scoped` in the group `weekly`, with a percent and the
/// model's `display_name`. An entry that counts every model is left out.
/// Each entry is read on its own, so a field of another shape skips only
/// that entry.
fn scoped_weeks(fields: &Map<String, Value>) -> Vec<LimitWindow> {
    let no_entries = Vec::new();
    let entries = fields
        .get("limits")
        .and_then(Value::as_array)
        .unwrap_or(&no_entries);

    entries.iter().filter_map(scoped_week).collect()
}

/// The window of one entry of the `limits` list, when it is the week of
/// one model.
fn scoped_week(entry: &Value) -> Option<LimitWindow> {
    let text = |field: &str| entry.get(field).and_then(Value::as_str);
    if text("kind") != Some(SCOPED_KIND) || text("group") != Some(SCOPED_GROUP)
    {
        return None;
    }

    let percent_used = entry.get("percent")?.as_f64()?;
    let model = entry.get("scope")?.get("model")?;
    let name = model.get("display_name")?.as_str()?.trim();
    let id = model.get("id").and_then(Value::as_str).unwrap_or_default();

    let name_slug = slug(name);
    let id = slug(id);
    let counts_every_model = name_slug == ALL_MODELS
        || id == ALL_MODELS
        || id.ends_with(&format!("-{ALL_MODELS}"));
    if name_slug.is_empty() || counts_every_model {
        return None;
    }

    let resets_at = text("resets_at").and_then(parse_rfc3339);
    Some(LimitWindow::new(model_week(name), percent_used, resets_at))
}

/// `text` in lower case with each run of other characters than letters
/// and digits turned into one `-`, and no `-` at its ends, as CodexBar
/// compares the scopes.
fn slug(text: &str) -> String {
    let mut slug = String::new();

    for letter in text.to_lowercase().chars() {
        if letter.is_alphanumeric() {
            slug.push(letter);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_string()
}

/// One window: `utilization` is the percent used, from 0 to 100, and
/// `resets_at` an RFC 3339 time.
fn usage_window(window: &Value, id: WindowId) -> Option<LimitWindow> {
    let percent_used = window.get("utilization")?.as_f64()?;
    let resets_at = window
        .get("resets_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339);

    Some(LimitWindow::new(id, percent_used, resets_at))
}

/// Whether `a` and `b` are the same window: the same length and the same
/// model, its case not counted.
fn is_same_window(a: &WindowId, b: &WindowId) -> bool {
    let same_model = match (a.model(), b.model()) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        (a, b) => a == b,
    };

    a.length() == b.length() && same_model
}

/// The extra usage the account may buy, while it is turned on: how much
/// was spent this month and the monthly limit. The answer counts both in
/// cents; they are kept in whole units of the currency.
fn extra_usage(extra: &Value) -> Option<ExtraUsage> {
    if extra.get("is_enabled").and_then(Value::as_bool) != Some(true) {
        return None;
    }

    let amount = |field: &str| {
        let cents = extra.get(field)?.as_f64()?;
        Some(cents / CENTS)
    };
    let currency = extra
        .get("currency")
        .and_then(Value::as_str)
        .map(String::from);

    Some(ExtraUsage::new(
        amount("used_credits"),
        amount("monthly_limit"),
        currency,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{NOW, at};

    fn limits_of(answer: &str) -> Result<Limits, SourceError> {
        let answer: Value =
            serde_json::from_str(answer).expect("the made-up answer is JSON");

        limits_from(&answer, None)
    }

    /// The usage windows of `limits`; any other billing fails the test.
    fn usage(limits: &Limits) -> &UsageWindows {
        match limits.billing() {
            Billing::UsageWindows(usage) => usage,
            other => panic!("expected usage windows, got {other:?}"),
        }
    }

    fn ids(answer: &str) -> Vec<WindowId> {
        let limits = limits_of(answer).expect("the limits are read");

        usage(&limits)
            .windows()
            .iter()
            .map(|window| window.id().clone())
            .collect()
    }

    #[test]
    fn a_missing_window_is_left_out() {
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": null},
            "seven_day_opus": null,
            "seven_day_sonnet": {"utilization": null, "resets_at": null}
        }"#;

        assert_eq!(ids(answer), [WindowId::hours(5)]);
    }

    #[test]
    fn unknown_fields_and_a_window_of_another_shape_are_skipped() {
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": "soon", "kind": "session"},
            "seven_day": "full",
            "iguana_necktie": {"anything": [1, 2, 3]},
            "limits": [{"scope": {"model": {"display_name": "Fable"}}}]
        }"#;

        let limits = limits_of(answer).expect("the limits are read");

        assert_eq!(
            usage(&limits).windows(),
            [LimitWindow::new(WindowId::hours(5), 42.0, None)]
        );
    }

    #[test]
    fn the_extra_usage_is_kept_in_money_next_to_the_windows() {
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": null},
            "extra_usage": {"is_enabled": true, "monthly_limit": 5000, "used_credits": 1250, "utilization": 25.0, "currency": "USD"}
        }"#;

        let limits = limits_of(answer).expect("the limits are read");

        assert_eq!(
            limits.billing(),
            &Billing::UsageWindows(UsageWindows::new(
                vec![LimitWindow::new(WindowId::hours(5), 42.0, None)],
                Some(ExtraUsage::new(
                    Some(12.5),
                    Some(50.0),
                    Some(String::from("USD"))
                )),
            ))
        );
    }

    #[test]
    fn an_account_with_only_extra_usage_has_no_windows() {
        let answer = r#"{
            "five_hour": null,
            "seven_day": null,
            "extra_usage": {"is_enabled": true, "monthly_limit": null, "used_credits": 830, "utilization": null}
        }"#;

        let limits = limits_of(answer).expect("the limits are read");

        assert!(usage(&limits).windows().is_empty());
        assert_eq!(
            usage(&limits).extra_usage(),
            Some(&ExtraUsage::new(Some(8.3), None, None))
        );
    }

    #[test]
    fn extra_usage_that_is_turned_off_is_left_out() {
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": null},
            "extra_usage": {"is_enabled": false, "monthly_limit": 5000, "used_credits": 0}
        }"#;

        let limits = limits_of(answer).expect("the limits are read");

        assert_eq!(usage(&limits).extra_usage(), None);
    }

    #[test]
    fn an_answer_with_no_window_at_all_is_unexpected() {
        let no_window =
            r#"{"five_hour": null, "extra_usage": {"is_enabled": false}}"#;

        assert_eq!(
            limits_of(no_window),
            Err(SourceError::UnexpectedAnswer(Unexpected::NoUsageWindow))
        );
        assert_eq!(
            limits_of("[1, 2]"),
            Err(SourceError::UnexpectedAnswer(Unexpected::NoUsageWindow))
        );
    }

    /// Owner, issue 27: every model window of the answer is a bar, also
    /// one of a model OTTY does not know by name.
    #[test]
    fn opus_sonnet_and_a_third_model_key_each_give_a_window() {
        let answer = r#"{
            "five_hour": {"utilization": 12.0, "resets_at": null},
            "seven_day": {"utilization": 30.0, "resets_at": null},
            "seven_day_fable": {"utilization": 64.0, "resets_at": "2026-11-20T10:00:00Z"},
            "seven_day_opus": {"utilization": 3.5, "resets_at": null},
            "seven_day_sonnet": {"utilization": 0.0, "resets_at": null}
        }"#;

        let limits = limits_of(answer).expect("the limits are read");

        assert_eq!(
            usage(&limits).windows(),
            [
                LimitWindow::new(WindowId::hours(5), 12.0, None),
                LimitWindow::new(WindowId::days(7), 30.0, None),
                LimitWindow::new(
                    WindowId::days(7).for_model("Opus"),
                    3.5,
                    None
                ),
                LimitWindow::new(
                    WindowId::days(7).for_model("Sonnet"),
                    0.0,
                    None
                ),
                LimitWindow::new(
                    WindowId::days(7).for_model("Fable"),
                    64.0,
                    Some(at(NOW + 4 * 24 * 3600))
                ),
            ]
        );
    }

    #[test]
    fn a_model_key_of_several_words_is_named_by_its_words() {
        let answer = r#"{
            "seven_day_example_model": {"utilization": 5.0, "resets_at": null}
        }"#;

        assert_eq!(ids(answer), [WindowId::days(7).for_model("Example model")]);
    }

    /// The newer shape of the answer: the weekly windows of single models
    /// are entries of `limits`, named by `scope.model.display_name`.
    #[test]
    fn weekly_scoped_entries_of_limits_each_give_a_window() {
        let answer = r#"{
            "five_hour": {"utilization": 12.0, "resets_at": null},
            "limits": [
                {"kind": "weekly_scoped", "group": "weekly", "percent": 64.0,
                 "resets_at": "2026-11-20T10:00:00Z", "is_active": false,
                 "scope": {"model": {"id": "made-up-fable-id", "display_name": "Fable"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 9,
                 "resets_at": null,
                 "scope": {"model": {"display_name": "Example Model"}}}
            ]
        }"#;

        let limits = limits_of(answer).expect("the limits are read");

        assert_eq!(
            usage(&limits).windows(),
            [
                LimitWindow::new(WindowId::hours(5), 12.0, None),
                LimitWindow::new(
                    WindowId::days(7).for_model("Fable"),
                    64.0,
                    Some(at(NOW + 4 * 24 * 3600))
                ),
                LimitWindow::new(
                    WindowId::days(7).for_model("Example Model"),
                    9.0,
                    None
                ),
            ]
        );
    }

    #[test]
    fn entries_of_limits_that_are_not_a_model_week_are_skipped() {
        let answer = r#"{
            "five_hour": {"utilization": 12.0, "resets_at": null},
            "limits": [
                {"kind": "weekly", "group": "weekly", "percent": 30.0,
                 "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "weekly_scoped", "group": "session", "percent": 30.0,
                 "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 30.0,
                 "scope": {"model": {"id": "made-up-all-models", "display_name": "Everything"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 30.0,
                 "scope": {"model": {"display_name": "All models"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": null,
                 "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": true,
                 "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 30.0,
                 "scope": {"model": {"display_name": " "}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 30.0},
                "not an entry"
            ]
        }"#;

        assert_eq!(ids(answer), [WindowId::hours(5)]);
    }

    #[test]
    fn a_model_named_by_a_key_and_by_limits_is_one_window() {
        let answer = r#"{
            "seven_day_fable": {"utilization": 64.0, "resets_at": null},
            "seven_day_opus": {"utilization": 3.0, "resets_at": null},
            "limits": [
                {"kind": "weekly_scoped", "group": "weekly", "percent": 65.0,
                 "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 4.0,
                 "scope": {"model": {"display_name": "opus"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 20.0,
                 "scope": {"model": {"display_name": "Haiku"}}},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 21.0,
                 "scope": {"model": {"display_name": "Haiku"}}}
            ]
        }"#;

        let limits = limits_of(answer).expect("the limits are read");

        assert_eq!(
            usage(&limits).windows(),
            [
                LimitWindow::new(
                    WindowId::days(7).for_model("Opus"),
                    3.0,
                    None
                ),
                LimitWindow::new(
                    WindowId::days(7).for_model("Fable"),
                    64.0,
                    None
                ),
                LimitWindow::new(
                    WindowId::days(7).for_model("Haiku"),
                    20.0,
                    None
                ),
            ]
        );
    }

    /// The keys CodexBar reads as something else than a model window:
    /// the routines window, the cloud credits and the reset status. They
    /// are skipped.
    #[test]
    fn keys_that_are_not_a_model_window_are_skipped() {
        let answer = r#"{
            "five_hour": {"utilization": 12.0, "resets_at": null},
            "seven_day_routines": {"utilization": 1.0, "resets_at": null},
            "seven_day_claude_routines": {"utilization": 2.0, "resets_at": null},
            "seven_day_cowork": {"utilization": 3.0, "resets_at": null},
            "iguana_necktie": {"utilization": 4.0, "resets_at": null},
            "cedar_ember": {"utilization": 5.0, "resets_at": null},
            "extra_usage": {"is_enabled": false, "utilization": 6.0}
        }"#;

        assert_eq!(ids(answer), [WindowId::hours(5)]);
    }

    /// CodexBar reads the weekly window of the apps signed in with Claude
    /// when the answer has no `seven_day`.
    #[test]
    fn the_oauth_apps_week_stands_in_for_a_missing_week() {
        let without_week = r#"{
            "five_hour": {"utilization": 12.0, "resets_at": null},
            "seven_day_oauth_apps": {"utilization": 80.0, "resets_at": null}
        }"#;
        let with_week = r#"{
            "seven_day": {"utilization": 30.0, "resets_at": null},
            "seven_day_oauth_apps": {"utilization": 80.0, "resets_at": null}
        }"#;

        let without_week =
            limits_of(without_week).expect("the limits are read");
        let with_week = limits_of(with_week).expect("the limits are read");

        assert_eq!(
            usage(&without_week).windows(),
            [
                LimitWindow::new(WindowId::hours(5), 12.0, None),
                LimitWindow::new(WindowId::days(7), 80.0, None),
            ]
        );
        assert_eq!(
            usage(&with_week).windows(),
            [LimitWindow::new(WindowId::days(7), 30.0, None)]
        );
    }
}
