//! The limits Codex wrote last into the session files of a profile.
//!
//! After each answer of the model, Codex writes a `token_count` event into
//! the session file with the `rate_limits` the server sent along: the
//! `primary` and `secondary` windows, the `credits` and the `plan_type`.
//! The numbers are as old as that record, so the limits carry its time.

use otty_plugin_sdk::agents::ReadError;
use otty_plugin_sdk::files::{self, FileInfo};
use serde_json::Value;

use crate::individual_limit::individual_limit_in;
use crate::limits::Limits;
use crate::rate_limits::{
    WindowNumbers, codex_credits, codex_limits, millis_of_seconds,
};
use crate::sessions::{newest_first, session_files};
use crate::spend_file::Line;
use crate::timestamp::millis_of;

/// How many session files are looked at, newest first, for a record of the
/// limits. A session that just started has no record yet, but an older
/// file may be large, so the walk stops here.
const MAX_FILES: usize = 10;

/// What marks the lines that can hold a record of the limits.
const RECORD_MARKER: &[u8] = b"\"token_count\"";

/// A `rate_limits` record of a session file and when it was written.
#[derive(Debug, Clone, PartialEq)]
struct RateLimitsRecord {
    rate_limits: Value,
    recorded_at_ms: Option<i64>,
}

/// The limits of the newest record in the session files of the profile in
/// `folder`, as of the time of that record.
///
/// The files are looked at from the one that changed last, and the last
/// record of the first file that has one wins: an older file can only
/// have older numbers. A file that can not be read is skipped.
pub(crate) fn read_limits(folder: &str) -> Result<Limits, ReadError> {
    let found = session_files(folder).map_err(ReadError::Host)?;

    newest_limits(found, last_record)
}

/// The limits of the newest record of `found`, as `read_record` reads the
/// last record of a file. A record with no time has the time its file
/// last changed.
fn newest_limits(
    found: Vec<FileInfo>,
    read_record: impl Fn(&str) -> Option<RateLimitsRecord>,
) -> Result<Limits, ReadError> {
    if found.is_empty() {
        return Err(ReadError::Failed("no session files".to_string()));
    }

    let newest = newest_first(found);
    let record = newest
        .iter()
        .take(MAX_FILES)
        .find_map(|file| Some((read_record(&file.path)?, file.changed_ms)));
    let Some((record, changed_ms)) = record else {
        return Err(ReadError::Failed(format!(
            "no limits in the {MAX_FILES} newest session files"
        )));
    };

    let recorded_at_ms = record.recorded_at_ms.unwrap_or(changed_ms);
    let limits = limits_from(&record.rate_limits, recorded_at_ms)?;

    Ok(Limits {
        recorded_at_ms: Some(recorded_at_ms),
        ..limits
    })
}

/// The last `token_count` record with a `rate_limits` object in the session
/// file at `path`; `None` when it has none or can not be read.
///
/// The host hands out only the lines that name a `token_count`; one that
/// is not such a record, or not JSON at all, is skipped. A file that
/// breaks in the middle gives what was read before.
fn last_record(path: &str) -> Option<RateLimitsRecord> {
    let mut last = None;

    for (number, batch) in files::lines(path, &[RECORD_MARKER]).enumerate() {
        let batch = match batch {
            Ok(batch) => batch,
            Err(_) if number == 0 => return None,
            Err(_) => break,
        };
        for line in batch.lines() {
            if let Some(record) = record_on(line) {
                last = Some(record);
            }
        }
    }

    last
}

/// The record on `line`, when it is a `token_count` event with a
/// `rate_limits` object.
fn record_on(line: &[u8]) -> Option<RateLimitsRecord> {
    let line: Line = serde_json::from_slice(line).ok()?;
    let payload = line.payload()?;
    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return None;
    }
    let rate_limits = payload.get("rate_limits").filter(|v| v.is_object())?;

    Some(RateLimitsRecord {
        rate_limits: rate_limits.clone(),
        recorded_at_ms: line.timestamp().and_then(millis_of),
    })
}

/// The limits in a `rate_limits` record written at `recorded_at_ms`.
fn limits_from(
    rate_limits: &Value,
    recorded_at_ms: i64,
) -> Result<Limits, ReadError> {
    let windows = ["primary", "secondary"]
        .iter()
        .filter_map(|field| window(rate_limits.get(*field)?, recorded_at_ms))
        .collect();
    let credits = rate_limits.get("credits");
    let credits = codex_credits(
        credits.and_then(|credits| credits.get("unlimited")),
        credits.and_then(|credits| credits.get("has_credits")),
        credits.and_then(|credits| credits.get("balance")),
        individual_limit_in(rate_limits),
    );
    let plan_type = rate_limits.get("plan_type").and_then(Value::as_str);

    codex_limits(windows, credits, plan_type)
}

/// One window: `used_percent` from 0 to 100, `window_minutes` its length in
/// minutes, and `resets_at`, a time in seconds since 1970. Older versions
/// of Codex wrote `resets_in_seconds` instead, counted from the record.
fn window(window: &Value, recorded_at_ms: i64) -> Option<WindowNumbers> {
    let percent_used = window.get("used_percent")?.as_f64()?;
    let minutes = window
        .get("window_minutes")
        .and_then(Value::as_u64)
        .and_then(|minutes| u32::try_from(minutes).ok());
    let resets_at_ms = match window.get("resets_at") {
        Some(seconds) if seconds.is_u64() => millis_of_seconds(seconds),
        _ => window
            .get("resets_in_seconds")
            .and_then(millis_of_seconds)
            .and_then(|millis| recorded_at_ms.checked_add(millis)),
    };

    Some(WindowNumbers {
        percent_used,
        minutes,
        resets_at_ms,
    })
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::Credits;

    use super::*;
    use crate::rate_limits::no_numbers;
    use crate::testing::{as_provider_in, file_info, host, path};

    /// 2026-11-16T10:00:00Z, the time of the made-up records.
    const NOW: i64 = 1_794_823_200_000;
    /// 2026-11-16T09:50:00Z, the time of the newest made-up record.
    const RECORDED: i64 = NOW - 600_000;

    const WINDOWS: &str = r#"{"limit_id":"codex","limit_name":null,"primary":{"used_percent":42.0,"window_minutes":300,"resets_at":1794826800},"secondary":{"used_percent":17.0,"window_minutes":10080,"resets_at":1794909600},"credits":{"has_credits":false,"unlimited":false,"balance":null},"plan_type":"plus"}"#;

    /// The made-up session files of `.codex-limits/`.
    const WITH_RECORDS: &str = ".codex-limits/sessions/2026/11/16/rollout-2026-11-16T09-00-00-019ac000-0000-7000-8000-00000000000a.jsonl";
    const COMPRESSED: &str = ".codex-limits/archived_sessions/rollout-2026-11-16T08-00-00-019ac000-0000-7000-8000-00000000000b.jsonl.zst";
    const NO_RECORD: &str = ".codex-limits/sessions/2026/11/16/rollout-2026-11-16T10-00-00-019ac000-0000-7000-8000-00000000000c.jsonl";
    const BROKEN: &str = ".codex-limits/sessions/2026/11/16/rollout-2026-11-16T10-30-00-019ac000-0000-7000-8000-00000000000d.jsonl.zst";

    /// A `token_count` record written at `timestamp` with `rate_limits`.
    fn token_count(timestamp: &str, rate_limits: &str) -> String {
        format!(
            r#"{{"timestamp":"{timestamp}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":100,"reasoning_output_tokens":0,"total_tokens":1100}},"model_context_window":272000}},"rate_limits":{rate_limits}}}}}"#
        )
    }

    /// The limits of a record of `rate_limits` written at 09:50.
    fn limits_of(rate_limits: &str) -> Result<Limits, ReadError> {
        let line = token_count("2026-11-16T09:50:00.000Z", rate_limits);
        let record = record_on(line.as_bytes()).expect("a record");

        newest_limits(vec![file_info("a", NOW)], |_| Some(record.clone()))
    }

    /// The limits the files `found` give, read through the host.
    fn limits_in(found: Vec<FileInfo>) -> Result<Limits, ReadError> {
        let host = host();

        as_provider_in(&host, ".codex-limits", || {
            newest_limits(found, last_record)
        })
    }

    /// The made-up file `relative`, changed at `changed_ms`.
    fn fixture(relative: &str, changed_ms: i64) -> FileInfo {
        file_info(&path(&host(), relative), changed_ms)
    }

    fn five_hour_and_weekly() -> Limits {
        Limits {
            windows: vec![
                otty_plugin_sdk::agents::LimitWindow {
                    length_minutes: Some(300),
                    model: None,
                    percent_used: 42.0,
                    resets_at_ms: Some(NOW + 3_600_000),
                    reset_text: None,
                },
                otty_plugin_sdk::agents::LimitWindow {
                    length_minutes: Some(10_080),
                    model: None,
                    percent_used: 17.0,
                    resets_at_ms: Some(NOW + 86_400_000),
                    reset_text: None,
                },
            ],
            credits: None,
            plan: Some("Plus".to_string()),
            recorded_at_ms: Some(RECORDED),
        }
    }

    fn credits_of(limits: Result<Limits, ReadError>) -> Option<Credits> {
        limits.expect("the limits are read").credits
    }

    #[test]
    fn the_last_record_gives_the_windows_the_plan_and_its_time() {
        let limits = limits_in(vec![fixture(WITH_RECORDS, NOW)]);

        assert_eq!(limits, Ok(five_hour_and_weekly()));
    }

    #[test]
    fn a_compressed_newest_file_is_read() {
        let limits = limits_in(vec![
            fixture(WITH_RECORDS, NOW - 60_000),
            fixture(COMPRESSED, NOW),
        ])
        .expect("the limits are read");

        assert_eq!(limits.windows, five_hour_and_weekly().windows);
        assert_eq!(limits.recorded_at_ms, Some(NOW - 4_200_000));
    }

    #[test]
    fn a_newest_file_with_no_record_falls_back_to_an_older_one() {
        let limits = limits_in(vec![
            fixture(WITH_RECORDS, NOW),
            fixture(NO_RECORD, NOW + 60_000),
        ]);

        assert_eq!(limits, Ok(five_hour_and_weekly()));
    }

    #[test]
    fn a_broken_or_missing_newest_file_is_skipped() {
        let limits = limits_in(vec![
            fixture(WITH_RECORDS, NOW),
            fixture(BROKEN, NOW + 60_000),
            fixture(".codex-limits/sessions/rollout-gone.jsonl", NOW + 120_000),
        ]);

        assert_eq!(limits, Ok(five_hour_and_weekly()));
    }

    #[test]
    fn a_record_older_than_the_ten_newest_files_is_not_read() {
        let mut found = vec![fixture(WITH_RECORDS, NOW - 60_000)];
        for number in 1..=10 {
            found.push(fixture(NO_RECORD, NOW + number));
        }

        let limits = limits_in(found);

        assert!(matches!(limits, Err(ReadError::Failed(_))), "{limits:?}");
    }

    #[test]
    fn no_session_files_has_no_limits() {
        let host = host();
        let empty = path(&host, ".codex-work-none");

        let limits =
            as_provider_in(&host, ".codex-work-none", || read_limits(&empty));

        assert_eq!(
            limits,
            Err(ReadError::Failed("no session files".to_string()))
        );
    }

    #[test]
    fn the_session_files_of_a_profile_are_read() {
        let host = host();
        let folder = path(&host, ".codex-limits");

        let limits =
            as_provider_in(&host, ".codex-limits", || read_limits(&folder))
                .expect("the limits");

        assert_eq!(limits.plan.as_deref(), Some("Plus"));
    }

    #[test]
    fn a_record_with_no_limits_and_other_lines_are_skipped() {
        let records = [
            token_count("2026-11-16T09:55:00.000Z", "null"),
            token_count("2026-11-16T09:56:00.000Z", "\"later\""),
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\""
                .to_string(),
            r#"{"type":"event_msg","payload":{"type":"agent_message","rate_limits":{"primary":{"used_percent":99}}}}"#.to_string(),
        ];

        for line in records {
            assert_eq!(record_on(line.as_bytes()), None, "{line}");
        }
    }

    #[test]
    fn a_reset_in_seconds_counts_from_the_time_of_the_record() {
        let older_shape = r#"{"primary":{"used_percent":5.0,"window_minutes":300,"resets_in_seconds":1800},"secondary":null}"#;

        let limits = limits_of(older_shape).expect("the limits are read");

        assert_eq!(limits.windows.len(), 1);
        assert_eq!(limits.windows[0].resets_at_ms, Some(RECORDED + 1_800_000));
        assert_eq!(limits.plan, None);
    }

    #[test]
    fn a_record_with_only_credits_is_credits() {
        let credits = r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":false,"balance":"3760"},"plan_type":"enterprise"}"#;

        let limits = limits_of(credits).expect("the limits are read");

        assert_eq!(
            limits.credits,
            Some(Credits {
                left: Some(3760.0),
                ..no_numbers()
            })
        );
        assert_eq!(limits.plan.as_deref(), Some("Enterprise"));
    }

    #[test]
    fn the_individual_limit_of_a_workspace_in_a_record_is_a_pool() {
        let workspace = r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":false,"balance":null},"individual_limit":{"limit":100000,"used":"7761","remaining_percent":92.239,"resets_at":1797465600},"plan_type":"enterprise_cbp_usage_based"}"#;

        assert_eq!(
            credits_of(limits_of(workspace)),
            Some(Credits {
                left: Some(92_239.0),
                used: Some(7761.0),
                pool: Some(100_000.0),
                renews_at_ms: Some(1_797_465_600_000),
                is_unlimited: false,
            })
        );
    }

    #[test]
    fn the_three_shapes_of_credits_in_a_record() {
        let available = r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":false,"balance":null},"plan_type":"enterprise_cbp_usage_based"}"#;
        let unlimited = r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":true},"plan_type":"enterprise"}"#;
        let none = r#"{"primary":null,"secondary":null,"credits":{"has_credits":false,"balance":"0"},"plan_type":"prolite"}"#;

        assert_eq!(credits_of(limits_of(available)), Some(no_numbers()));
        assert_eq!(
            credits_of(limits_of(unlimited)),
            Some(Credits {
                is_unlimited: true,
                ..no_numbers()
            })
        );
        assert!(matches!(limits_of(none), Err(ReadError::Unexpected(_))));
    }

    #[test]
    fn no_credits_next_to_windows_are_the_windows() {
        let prolite = WINDOWS
            .replace(
                r#""has_credits":false,"unlimited":false,"balance":null"#,
                r#""has_credits":false,"balance":"0""#,
            )
            .replace(r#""plan_type":"plus""#, r#""plan_type":"prolite""#);

        let limits = limits_of(&prolite).expect("the limits are read");

        assert_eq!(limits.windows, five_hour_and_weekly().windows);
        assert_eq!(limits.plan.as_deref(), Some("Pro 100"));
    }

    #[test]
    fn a_record_with_no_time_has_the_time_of_the_file() {
        let line = token_count("x", WINDOWS).replace(r#""timestamp":"x","#, "");
        let record = record_on(line.as_bytes()).expect("a record");

        let limits =
            newest_limits(vec![file_info("a", NOW)], |_| Some(record.clone()))
                .expect("the limits are read");

        assert_eq!(limits.recorded_at_ms, Some(NOW));
    }
}
