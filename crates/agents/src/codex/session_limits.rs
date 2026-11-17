//! The third source of Codex limits: the newest record of the limits in
//! the session files of the profile.
//!
//! After each answer of the model, Codex writes a `token_count` event into
//! the session file with the `rate_limits` the server sent along: the
//! `primary` and `secondary` windows, the `credits` and the `plan_type`.
//! The numbers are as old as that record, so the limits carry its time.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::Value;

use super::individual_limit::individual_limit_in;
use super::rate_limits::{
    WindowNumbers, codex_credits, codex_limits, time_from_seconds,
};
use super::session_file::open_session_file;
use super::sessions::session_files;
use crate::error::SourceError;
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::source_id::SourceId;
use crate::timestamp::parse_rfc3339;

/// How many session files are looked at, newest first, for a record of the
/// limits. A session that just started has no record yet, but an older
/// file may be large, so the walk stops here.
const MAX_FILES: usize = 10;

/// The last `rate_limits` record of a session file and when it was written.
struct RateLimitsRecord {
    rate_limits: Value,
    recorded_at: Option<SystemTime>,
}

/// Reads the limits Codex wrote last into the session files.
pub(super) struct SessionRecord;

impl LimitSource for SessionRecord {
    fn id(&self) -> SourceId {
        SourceId::LatestCodexSession
    }

    /// The files are looked at from the one that changed last, and the
    /// last record of the first file that has one wins: an older file can
    /// only have older numbers. A file that can not be read is skipped. No
    /// file, or no record in the [`MAX_FILES`] newest ones, is "not
    /// available".
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError> {
        let files = newest_first(session_files(profile.config_dir())?);
        if files.is_empty() {
            return Err(SourceError::NotAvailable(String::from(
                "no session files",
            )));
        }

        let found =
            files.iter().take(MAX_FILES).find_map(|(path, changed_at)| {
                Some((last_record(path)?, *changed_at))
            });
        let Some((record, changed_at)) = found else {
            return Err(SourceError::NotAvailable(format!(
                "no limits in the {MAX_FILES} newest session files"
            )));
        };

        let recorded_at = record.recorded_at.unwrap_or(changed_at);
        let limits = limits_from(&record.rate_limits, recorded_at)?;

        Ok(limits.with_recorded_at(recorded_at))
    }
}

/// `files` with the time each last changed, the newest first. Of two files
/// that changed at the same time, the one with the later path comes first.
/// A file whose time can not be read is left out.
fn newest_first(files: Vec<PathBuf>) -> Vec<(PathBuf, SystemTime)> {
    let mut files: Vec<(PathBuf, SystemTime)> = files
        .into_iter()
        .filter_map(|path| {
            let changed_at = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((path, changed_at))
        })
        .collect();

    files.sort_by(|(a_path, a_time), (b_path, b_time)| {
        b_time.cmp(a_time).then_with(|| b_path.cmp(a_path))
    });
    files
}

/// The last `token_count` record with a `rate_limits` object in the session
/// file at `path`; `None` when it has none or can not be opened.
///
/// A line that is not such a record, or not JSON at all, is skipped. A file
/// that breaks in the middle gives what was read before.
fn last_record(path: &Path) -> Option<RateLimitsRecord> {
    let reader = open_session_file(path).ok()?;

    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    let mut last = None;
    while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
        if let Some(record) = rate_limits_record(&line) {
            last = Some(record);
        }
        line.clear();
    }

    last
}

/// The record on `line`, when it is a `token_count` event with a
/// `rate_limits` object.
fn rate_limits_record(line: &[u8]) -> Option<RateLimitsRecord> {
    // Most lines are messages: only the ones that name the event are
    // parsed.
    let marker = b"token_count";
    if !line.windows(marker.len()).any(|part| part == marker) {
        return None;
    }

    let record: Value = serde_json::from_slice(line).ok()?;
    let payload = record.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return None;
    }
    let rate_limits = payload.get("rate_limits").filter(|v| v.is_object())?;

    let recorded_at = record
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339);

    Some(RateLimitsRecord {
        rate_limits: rate_limits.clone(),
        recorded_at,
    })
}

/// The limits in a `rate_limits` record written at `recorded_at`.
fn limits_from(
    rate_limits: &Value,
    recorded_at: SystemTime,
) -> Result<Limits, SourceError> {
    let windows = ["primary", "secondary"]
        .iter()
        .filter_map(|field| window(rate_limits.get(*field)?, recorded_at))
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
fn window(window: &Value, recorded_at: SystemTime) -> Option<WindowNumbers> {
    let percent_used = window.get("used_percent")?.as_f64()?;
    let seconds = window
        .get("window_minutes")
        .and_then(Value::as_u64)
        .and_then(|minutes| minutes.checked_mul(60));
    let resets_at = match window.get("resets_at").and_then(Value::as_u64) {
        Some(seconds) => time_from_seconds(seconds),
        None => window
            .get("resets_in_seconds")
            .and_then(Value::as_u64)
            .and_then(|seconds| {
                recorded_at.checked_add(Duration::from_secs(seconds))
            }),
    };

    Some(WindowNumbers {
        percent_used,
        seconds,
        resets_at,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::billing::{Billing, Credits, LimitWindow, UsageWindows};
    use crate::testing::{
        TempDir, at, codex_profile, set_file_time, write_compressed, write_file,
    };

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;
    const ID: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";
    const META: &str = r#"{"timestamp":"2026-11-16T09:00:00.000Z","type":"session_meta","payload":{"id":"019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b","cwd":"/home/user/otty","source":"cli"}}"#;
    const MESSAGE: &str = r#"{"timestamp":"2026-11-16T09:00:05.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Fix the tabs"}]}}"#;
    const WINDOWS: &str = r#"{"limit_id":"codex","limit_name":null,"primary":{"used_percent":42.0,"window_minutes":300,"resets_at":1794826800},"secondary":{"used_percent":17.0,"window_minutes":10080,"resets_at":1794909600},"credits":{"has_credits":false,"unlimited":false,"balance":null},"plan_type":"plus"}"#;

    /// A `token_count` record written at `timestamp` with `rate_limits`.
    fn token_count(timestamp: &str, rate_limits: &str) -> String {
        format!(
            r#"{{"timestamp":"{timestamp}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":100,"reasoning_output_tokens":0,"total_tokens":1100}},"model_context_window":272000}},"rate_limits":{rate_limits}}}}}"#
        )
    }

    /// A made-up Codex config folder.
    struct ConfigDir {
        temp: TempDir,
    }

    impl ConfigDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join(".codex-work")
        }

        fn new() -> Self {
            Self {
                temp: TempDir::new(),
            }
        }

        fn session_file(&self, folder: &str, name: &str) -> PathBuf {
            self.path()
                .join(folder)
                .join(format!("rollout-{ID}-{name}"))
        }

        /// Writes a plain session file of `lines` that last changed at
        /// [`NOW`].
        fn write_plain(&self, folder: &str, name: &str, lines: &[&str]) {
            let path = self.session_file(folder, &format!("{name}.jsonl"));
            write_file(&path, &lines.join("\n"));
            set_file_time(&path, at(NOW));
        }

        fn limits(&self) -> Result<Limits, SourceError> {
            SessionRecord.limits(&codex_profile(&self.path()))
        }
    }

    fn five_hour_and_weekly() -> Billing {
        Billing::UsageWindows(UsageWindows::new(
            vec![
                LimitWindow::new("5 hours", 42.0, Some(at(NOW + 3600))),
                LimitWindow::new("7 days", 17.0, Some(at(NOW + 86_400))),
            ],
            None,
        ))
    }

    fn not_available(text: &str) -> Result<Limits, SourceError> {
        Err(SourceError::NotAvailable(String::from(text)))
    }

    #[test]
    fn the_last_record_gives_the_windows_the_plan_and_its_time() {
        let config = ConfigDir::new();
        let older = WINDOWS.replace("42.0", "3.0");
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[
                META,
                MESSAGE,
                &token_count("2026-11-16T09:30:00.000Z", &older),
                MESSAGE,
                &token_count("2026-11-16T09:50:00.000Z", WINDOWS),
                MESSAGE,
            ],
        );

        let limits = config.limits();

        assert_eq!(
            limits,
            Ok(
                Limits::new(five_hour_and_weekly(), Some(String::from("Plus")))
                    .with_recorded_at(at(NOW - 600))
            )
        );
    }

    #[test]
    fn the_newest_file_with_a_record_wins() {
        let config = ConfigDir::new();
        let newest = config.session_file("archived_sessions", "b.jsonl");
        let older = WINDOWS.replace("42.0", "3.0");
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[META, &token_count("2026-11-16T09:59:00.000Z", &older)],
        );
        config.write_plain(
            "archived_sessions",
            "b",
            &[META, &token_count("2026-11-16T09:50:00.000Z", WINDOWS)],
        );
        set_file_time(&newest, at(NOW + 60));

        let limits = config.limits().expect("the limits are read");

        assert_eq!(limits.billing(), &five_hour_and_weekly());
    }

    #[test]
    fn a_compressed_newest_file_is_read() {
        let config = ConfigDir::new();
        let path = config.session_file("sessions/2026/11/16", "a.jsonl.zst");
        let mut content = format!("{META}\n");
        for _ in 0..500 {
            content.push_str(MESSAGE);
            content.push('\n');
        }
        content.push_str(&token_count("2026-11-16T09:50:00.000Z", WINDOWS));
        write_compressed(&path, &content);

        let limits = config.limits().expect("the limits are read");

        assert_eq!(limits.billing(), &five_hour_and_weekly());
        assert_eq!(limits.recorded_at(), Some(at(NOW - 600)));
    }

    #[test]
    fn a_record_with_no_limits_and_broken_lines_are_skipped() {
        let config = ConfigDir::new();
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[
                META,
                &token_count("2026-11-16T09:50:00.000Z", WINDOWS),
                &token_count("2026-11-16T09:55:00.000Z", "null"),
                &token_count("2026-11-16T09:56:00.000Z", "\"later\""),
                "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\"",
                r#"{"type":"event_msg","payload":{"type":"agent_message","rate_limits":{"primary":{"used_percent":99}}}}"#,
            ],
        );

        let limits = config.limits().expect("the limits are read");

        assert_eq!(limits.recorded_at(), Some(at(NOW - 600)));
    }

    #[test]
    fn a_reset_in_seconds_counts_from_the_time_of_the_record() {
        let config = ConfigDir::new();
        let older_shape = r#"{"primary":{"used_percent":5.0,"window_minutes":300,"resets_in_seconds":1800},"secondary":null}"#;
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[META, &token_count("2026-11-16T09:50:00.000Z", older_shape)],
        );

        let limits = config.limits().expect("the limits are read");

        assert_eq!(
            limits.billing(),
            &Billing::UsageWindows(UsageWindows::new(
                vec![LimitWindow::new("5 hours", 5.0, Some(at(NOW + 1200)))],
                None,
            ))
        );
        assert_eq!(limits.plan(), None);
    }

    #[test]
    fn a_record_with_only_credits_is_credits() {
        let config = ConfigDir::new();
        let credits = r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":false,"balance":"3760"},"plan_type":"enterprise"}"#;
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[META, &token_count("2026-11-16T09:50:00.000Z", credits)],
        );

        let limits = config.limits().expect("the limits are read");

        assert_eq!(
            limits.billing(),
            &Billing::Credits(Credits::new(Some(3760.0), None, None, None))
        );
        assert_eq!(limits.plan(), Some("Enterprise"));
    }

    #[test]
    fn the_individual_limit_of_a_workspace_in_a_record_is_a_pool() {
        let config = ConfigDir::new();
        let workspace = r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":false,"balance":null},"individual_limit":{"limit":100000,"used":"7761","remaining_percent":92.239,"resets_at":1797465600},"plan_type":"enterprise_cbp_usage_based"}"#;
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[META, &token_count("2026-11-16T09:50:00.000Z", workspace)],
        );

        let limits = config.limits().expect("the limits are read");

        assert_eq!(
            limits.billing(),
            &Billing::Credits(Credits::new(
                Some(92_239.0),
                Some(7761.0),
                Some(100_000.0),
                Some(at(1_797_465_600)),
            ))
        );
    }

    #[test]
    fn the_three_shapes_of_credits_in_a_record() {
        let shapes = [
            r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":false,"balance":null},"plan_type":"enterprise_cbp_usage_based"}"#,
            r#"{"primary":null,"secondary":null,"credits":{"has_credits":true,"unlimited":true},"plan_type":"enterprise"}"#,
            r#"{"primary":null,"secondary":null,"credits":{"has_credits":false,"balance":"0"},"plan_type":"prolite"}"#,
        ];
        let mut found = Vec::new();

        for shape in shapes {
            let config = ConfigDir::new();
            config.write_plain(
                "sessions/2026/11/16",
                "a",
                &[META, &token_count("2026-11-16T09:50:00.000Z", shape)],
            );
            found.push(config.limits());
        }

        let at_record = |billing: Billing, plan: &str| {
            Ok(Limits::new(billing, Some(String::from(plan)))
                .with_recorded_at(at(NOW - 600)))
        };
        assert_eq!(
            found,
            [
                at_record(Billing::Credits(Credits::available()), "Enterprise"),
                at_record(Billing::Credits(Credits::unlimited()), "Enterprise"),
                Err(SourceError::UnexpectedAnswer(String::from(
                    "no usage window and no credits in the answer"
                ))),
            ]
        );
    }

    #[test]
    fn no_credits_next_to_windows_are_the_windows() {
        let config = ConfigDir::new();
        let prolite = WINDOWS
            .replace(
                r#""has_credits":false,"unlimited":false,"balance":null"#,
                r#""has_credits":false,"balance":"0""#,
            )
            .replace(r#""plan_type":"plus""#, r#""plan_type":"prolite""#);
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[META, &token_count("2026-11-16T09:50:00.000Z", &prolite)],
        );

        let limits = config.limits().expect("the limits are read");

        assert_eq!(limits.billing(), &five_hour_and_weekly());
        assert_eq!(limits.plan(), Some("Prolite"));
    }

    #[test]
    fn a_record_with_no_time_has_the_time_of_the_file() {
        let config = ConfigDir::new();
        let no_time =
            token_count("x", WINDOWS).replace(r#""timestamp":"x","#, "");
        config.write_plain("sessions/2026/11/16", "a", &[META, &no_time]);

        let limits = config.limits().expect("the limits are read");

        assert_eq!(limits.recorded_at(), Some(at(NOW)));
    }

    #[test]
    fn a_newest_file_with_no_record_falls_back_to_an_older_one() {
        let config = ConfigDir::new();
        let path = config.session_file("sessions/2026/11/16", "b.jsonl");
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[META, &token_count("2026-11-16T09:50:00.000Z", WINDOWS)],
        );
        config.write_plain("sessions/2026/11/16", "b", &[META, MESSAGE]);
        set_file_time(&path, at(NOW + 60));

        let limits = config.limits().expect("the older record is read");

        assert_eq!(limits.billing(), &five_hour_and_weekly());
        assert_eq!(limits.recorded_at(), Some(at(NOW - 600)));
    }

    #[test]
    fn a_record_older_than_the_ten_newest_files_is_not_read() {
        let config = ConfigDir::new();
        config.write_plain(
            "sessions/2026/11/15",
            "old",
            &[META, &token_count("2026-11-15T09:50:00.000Z", WINDOWS)],
        );
        for number in 1..=10_u64 {
            let name = format!("new{number}");
            let path = config
                .session_file("sessions/2026/11/16", &format!("{name}.jsonl"));
            config.write_plain("sessions/2026/11/16", &name, &[META, MESSAGE]);
            set_file_time(&path, at(NOW + number));
        }

        assert_eq!(
            config.limits(),
            not_available("no limits in the 10 newest session files")
        );
    }

    #[test]
    fn no_session_files_is_not_available() {
        let config = ConfigDir::new();
        let empty_folder = config.path().join("sessions");
        let missing = config.limits();
        std::fs::create_dir_all(&empty_folder).expect("create the folder");
        write_file(&empty_folder.join("notes.txt"), "not a session");
        let empty = config.limits();

        assert_eq!(missing, not_available("no session files"));
        assert_eq!(empty, not_available("no session files"));
    }

    #[test]
    fn a_broken_newest_file_is_skipped() {
        let config = ConfigDir::new();
        let broken = config.session_file("sessions/2026/11/16", "b.jsonl.zst");
        config.write_plain(
            "sessions/2026/11/16",
            "a",
            &[META, &token_count("2026-11-16T09:50:00.000Z", WINDOWS)],
        );
        write_file(&broken, "not compressed at all");
        set_file_time(&broken, at(NOW + 60));

        let limits = config.limits().expect("the older record is read");

        assert_eq!(limits.billing(), &five_hour_and_weekly());
    }
}
