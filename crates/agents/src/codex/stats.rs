//! The statistics of a Codex session and of a project folder, read from the
//! session files in `<config>/sessions/` and `<config>/archived_sessions/`,
//! plain or compressed.
//!
//! The lines are read by the same code as the tokens of the dashboard
//! (`spend_file.rs`) and the limits of the latest session
//! (`session_limits.rs`). Which file holds which session is taken from what
//! the session list read last, so a file whose first line was read already
//! is not opened again.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use serde_json::Value;

use super::session_file::{SessionMeta, open_session_file, read_session_meta};
use super::session_limits::{RateLimitsRecord, rate_limits_in};
use super::sessions::{newest_first, session_files};
use super::spend_file::{Line, SessionTokens, is_counted_line, read_spend};
use crate::billing::{Billing, LimitWindow};
use crate::error::SourceError;
use crate::file_memory::FileMemory;
use crate::file_stamp::FileStamp;
use crate::jsonl::{for_each_line, has_marker};
use crate::profile::Profile;
use crate::session::SessionId;
use crate::spend_files::FileSpend;
use crate::spend_tally::whole_history;
use crate::stats::{LimitChange, ProjectStats, SessionStats, StatsSource};
use crate::timestamp::{earliest, latest, parse_rfc3339};

/// What the lines of one session file told: the answers, the messages of
/// the user, the first and the last time, and the first and the last
/// record of the limits.
#[derive(Default)]
struct SessionLines {
    tokens: SessionTokens,
    messages: u64,
    first_at: Option<SystemTime>,
    last_at: Option<SystemTime>,
    first_limits: Option<RateLimitsRecord>,
    last_limits: Option<RateLimitsRecord>,
}

impl SessionLines {
    /// Takes what `line` tells: its time, a message of the user, the
    /// limits and the tokens of a `token_count`.
    fn add(&mut self, line: &Line) {
        if let Some(at) = line.timestamp().and_then(parse_rfc3339) {
            self.first_at = earliest(self.first_at, Some(at));
            self.last_at = latest(self.last_at, Some(at));
        }

        if line.kind() == Some("event_msg")
            && let Some(payload) = line.payload()
        {
            if payload.get("type").and_then(Value::as_str)
                == Some("user_message")
            {
                self.messages += 1;
            }
            if let Some(record) = rate_limits_in(line.timestamp(), payload) {
                if self.first_limits.is_none() {
                    self.first_limits = Some(record);
                } else {
                    self.last_limits = Some(record);
                }
            }
        }

        self.tokens.add(line);
    }
}

/// Reads the statistics from the session files.
///
/// A session is the file whose first line names its id; a project is
/// every session file whose first line names its folder. The sessions of
/// helper agents are not listed, so they are not counted either.
pub(super) struct SessionFilesStats {
    cache: Arc<FileMemory<Option<SessionMeta>>>,
}

impl SessionFilesStats {
    /// A source that finds the sessions in what the session list kept in
    /// `cache`.
    pub(super) fn new(cache: Arc<FileMemory<Option<SessionMeta>>>) -> Self {
        Self { cache }
    }

    /// The session files of `config_dir` whose session `wanted` picks.
    fn files_where(
        &self,
        config_dir: &Path,
        wanted: impl Fn(&SessionMeta) -> bool,
    ) -> Result<Vec<PathBuf>, SourceError> {
        let paths = session_files(config_dir)?;

        let files = paths
            .into_iter()
            .filter(|path| {
                let Ok(metadata) = std::fs::metadata(path) else {
                    return false;
                };
                let stamp = FileStamp::of(&metadata);
                let meta = self.cache.kept_or_read(
                    config_dir,
                    path,
                    stamp,
                    read_session_meta,
                );
                meta.as_ref().is_some_and(&wanted)
            })
            .collect();

        Ok(files)
    }
}

impl StatsSource for SessionFilesStats {
    /// Of the copies of a session, plain and compressed or archived, the
    /// one that changed last is read. A session with no file is "not
    /// available".
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
    ) -> Result<SessionStats, SourceError> {
        let id = session.as_str();

        let files =
            self.files_where(profile.config_dir(), |meta| meta.id() == id)?;
        let Some((path, _)) = newest_first(files).into_iter().next() else {
            return Err(SourceError::NotAvailable(format!(
                "no session file of {id}"
            )));
        };
        let lines = read_lines(&path);

        let changes = limit_changes(&lines);
        let spend = lines.tokens.into_file_spend();
        let history = whole_history(std::iter::once(&spend));

        let stats = SessionStats::new(
            lines.first_at,
            lines.last_at,
            lines.messages,
            history.models().to_vec(),
        );
        Ok(stats.with_limit_changes(changes))
    }

    /// The same answer in two copies of a session counts once.
    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
    ) -> Result<ProjectStats, SourceError> {
        let files = self.files_where(profile.config_dir(), |meta| {
            meta.folder() == folder
        })?;

        let spends: Vec<FileSpend> =
            files.iter().map(|path| read_spend(path)).collect();

        Ok(whole_history(spends.iter()))
    }
}

/// What the lines of the session file at `path` tell; nothing for a file
/// that can not be opened. A broken line is skipped, and a file that breaks
/// in the middle gives what was read before.
fn read_lines(path: &Path) -> SessionLines {
    let Ok(reader) = open_session_file(path) else {
        return SessionLines::default();
    };

    let mut lines = SessionLines::default();
    for_each_line(reader, |line| {
        let is_wanted =
            is_counted_line(line) || has_marker(line, b"user_message");
        if is_wanted && let Ok(parsed) = serde_json::from_slice::<Line>(line) {
            lines.add(&parsed);
        }
    });

    lines
}

/// How each usage window moved from the first record of the limits to the
/// last one, shorter windows first; none with fewer than two records. A
/// window only one of them has is left out.
fn limit_changes(lines: &SessionLines) -> Vec<LimitChange> {
    let (Some(first), Some(last)) = (&lines.first_limits, &lines.last_limits)
    else {
        return Vec::new();
    };
    let fallback = lines.last_at.unwrap_or(SystemTime::UNIX_EPOCH);

    let windows_of = |record: &RateLimitsRecord| -> Vec<LimitWindow> {
        let recorded_at = record.recorded_at().unwrap_or(fallback);
        match record
            .limits(recorded_at)
            .map(|limits| limits.billing().clone())
        {
            Ok(Billing::UsageWindows(windows)) => windows.windows().to_vec(),
            _ => Vec::new(),
        }
    };
    let at_start = windows_of(first);

    windows_of(last)
        .iter()
        .filter_map(|end| {
            let start =
                at_start.iter().find(|start| start.name() == end.name())?;
            Some(LimitChange::new(
                end.name(),
                start.percent_used(),
                end.percent_used(),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spend::ModelTokens;
    use crate::testing::{
        NOW, TempDir, at, codex_profile, set_file_time, write_compressed,
        write_file,
    };
    use crate::token_count::TokenCount;

    fn meta(id: &str, folder: &str, time: &str) -> String {
        format!(
            r#"{{"timestamp":"{time}","type":"session_meta","payload":{{"id":"{id}","cwd":"{folder}","source":"cli"}}}}"#
        )
    }

    fn turn(model: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-11-16T10:00:01.000Z","type":"turn_context","payload":{{"cwd":"/home/user/otty","model":"{model}"}}}}"#
        )
    }

    fn user(time: &str, text: &str) -> String {
        format!(
            r#"{{"timestamp":"{time}","type":"event_msg","payload":{{"type":"user_message","message":"{text}","kind":"plain"}}}}"#
        )
    }

    /// A `token_count` at `time` with this answer's `last` and the
    /// session's `total`, each as input, cached input and output, and the
    /// 5 hours and 7 days windows used `five_hours` and `week` percent.
    fn count(
        time: &str,
        last: (u64, u64, u64),
        total: (u64, u64, u64),
        limits: Option<(f64, f64)>,
    ) -> String {
        let usage = |(input, cached, output): (u64, u64, u64)| {
            format!(
                r#"{{"input_tokens":{input},"cached_input_tokens":{cached},"output_tokens":{output}}}"#
            )
        };
        let (last, total) = (usage(last), usage(total));
        let rate_limits = match limits {
            Some((five_hours, week)) => format!(
                r#"{{"primary":{{"used_percent":{five_hours},"window_minutes":300,"resets_at":1794826800}},"secondary":{{"used_percent":{week},"window_minutes":10080,"resets_at":1794909600}},"plan_type":"plus"}}"#
            ),
            None => String::from("null"),
        };
        format!(
            r#"{{"timestamp":"{time}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{total},"last_token_usage":{last}}},"rate_limits":{rate_limits}}}}}"#
        )
    }

    /// A made-up Codex config folder and a source for it.
    struct ConfigDir {
        temp: TempDir,
        source: SessionFilesStats,
    }

    impl ConfigDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join(".codex-work")
        }

        fn new() -> Self {
            Self {
                temp: TempDir::new(),
                source: SessionFilesStats::new(Arc::default()),
            }
        }

        /// Writes `lines` to the session file `name` in `folder` of the
        /// config folder, changed at `changed`.
        fn write(
            &self,
            folder: &str,
            name: &str,
            lines: &[String],
            changed: u64,
        ) {
            let path = self.path().join(folder).join(name);
            write_file(&path, &lines.join("\n"));
            set_file_time(&path, at(changed));
        }

        fn session(&self, id: &str) -> Result<SessionStats, SourceError> {
            self.source.session_stats(
                &codex_profile(&self.path()),
                &SessionId::new(id),
            )
        }

        fn project(&self, folder: &str) -> Result<ProjectStats, SourceError> {
            self.source
                .project_stats(&codex_profile(&self.path()), Path::new(folder))
        }
    }

    #[test]
    fn a_session_gives_its_times_messages_tokens_and_limits() {
        let config = ConfigDir::new();
        config.write(
            "sessions/2026/11/16",
            "rollout-2026-11-16T10-00-00-a1.jsonl",
            &[
                meta("a1", "/home/user/otty", "2026-11-16T10:00:00.000Z"),
                turn("gpt-5-codex"),
                user("2026-11-16T10:00:02.000Z", "Fix the tabs"),
                count(
                    "2026-11-16T10:01:00.000Z",
                    (1000, 800, 50),
                    (1000, 800, 50),
                    Some((12.0, 40.0)),
                ),
                String::from("{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\""),
                user("2026-11-16T10:20:00.000Z", "And the docs"),
                count(
                    "2026-11-16T10:30:00.000Z",
                    (300, 100, 20),
                    (1300, 900, 70),
                    Some((31.0, 42.5)),
                ),
            ],
            NOW,
        );

        let stats = config.session("a1").expect("the session is read");

        assert_eq!(stats.started_at(), Some(at(NOW)));
        assert_eq!(stats.last_active_at(), Some(at(NOW + 1800)));
        assert_eq!(stats.messages(), 2);
        assert_eq!(stats.answers(), 2);
        assert_eq!(
            stats.models(),
            [ModelTokens::new(
                Some("gpt-5-codex"),
                TokenCount::new(400, 900, 70),
                2
            )]
        );
        assert_eq!(
            stats.limit_changes(),
            [
                LimitChange::new("5 hours", 12.0, 31.0),
                LimitChange::new("7 days", 40.0, 42.5),
            ]
        );
    }

    #[test]
    fn a_session_with_one_record_of_the_limits_has_no_change() {
        let config = ConfigDir::new();
        config.write(
            "sessions",
            "rollout-a2.jsonl",
            &[
                meta("a2", "/home/user/otty", "2026-11-16T10:00:00.000Z"),
                count(
                    "2026-11-16T10:01:00.000Z",
                    (10, 0, 5),
                    (10, 0, 5),
                    Some((12.0, 40.0)),
                ),
            ],
            NOW,
        );

        let stats = config.session("a2").expect("the session is read");

        assert!(stats.limit_changes().is_empty());
    }

    #[test]
    fn a_session_with_no_token_lines_has_no_tokens_and_no_limits() {
        let config = ConfigDir::new();
        config.write(
            "sessions",
            "rollout-a3.jsonl",
            &[
                meta("a3", "/home/user/otty", "2026-11-16T10:00:00.000Z"),
                user("2026-11-16T10:00:30.000Z", "Hello"),
            ],
            NOW,
        );

        let stats = config.session("a3").expect("the session is read");

        assert_eq!(stats.messages(), 1);
        assert!(stats.models().is_empty());
        assert!(stats.limit_changes().is_empty());
        assert_eq!(stats.length(), Some(std::time::Duration::from_secs(30)));
    }

    #[test]
    fn of_two_copies_of_a_session_the_newer_is_read() {
        let config = ConfigDir::new();
        let start = meta("a4", "/home/user/otty", "2026-11-16T10:00:00.000Z");
        config.write(
            "archived_sessions",
            "rollout-a4.jsonl",
            &[start.clone(), user("2026-11-16T10:00:01.000Z", "Old")],
            NOW - 60,
        );
        let path = config.path().join("sessions/rollout-a4.jsonl.zst");
        let newer = [
            start,
            user("2026-11-16T10:00:01.000Z", "Old"),
            user("2026-11-16T10:00:02.000Z", "New"),
        ];
        write_compressed(&path, &newer.join("\n"));
        set_file_time(&path, at(NOW));

        let stats = config.session("a4").expect("the session is read");

        assert_eq!(stats.messages(), 2);
    }

    #[test]
    fn a_session_with_no_file_is_not_available() {
        let config = ConfigDir::new();

        let stats = config.session("gone");

        assert!(matches!(stats, Err(SourceError::NotAvailable(_))));
    }

    #[test]
    fn a_project_counts_its_sessions_over_the_whole_history() {
        let config = ConfigDir::new();
        let answer = |time: &str, output: u64| {
            count(time, (10, 0, output), (10, 0, output), None)
        };
        config.write(
            "archived_sessions",
            "rollout-old.jsonl",
            &[
                meta("old", "/home/user/otty", "2025-01-02T09:00:00.000Z"),
                turn("o3"),
                answer("2025-01-02T09:01:00.000Z", 1),
            ],
            NOW - 400 * 24 * 3600,
        );
        config.write(
            "sessions",
            "rollout-new.jsonl",
            &[
                meta("new", "/home/user/otty", "2026-11-16T10:00:00.000Z"),
                turn("gpt-5-codex"),
                answer("2026-11-16T10:01:00.000Z", 2),
            ],
            NOW,
        );
        config.write(
            "sessions",
            "rollout-docs.jsonl",
            &[
                meta("docs", "/home/user/docs", "2026-11-16T10:00:00.000Z"),
                turn("gpt-5-codex"),
                answer("2026-11-16T10:02:00.000Z", 100),
            ],
            NOW,
        );

        let stats = config
            .project("/home/user/otty")
            .expect("the project is read");

        assert_eq!(stats.first_at(), parse_rfc3339("2025-01-02T09:01:00.000Z"));
        assert_eq!(stats.last_at(), Some(at(NOW + 60)));
        assert_eq!(
            stats.models(),
            [
                ModelTokens::new(
                    Some("gpt-5-codex"),
                    TokenCount::new(10, 0, 2),
                    1
                ),
                ModelTokens::new(Some("o3"), TokenCount::new(10, 0, 1), 1),
            ]
        );
    }
}
