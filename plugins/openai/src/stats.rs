//! The statistics of a Codex session and of a project folder, read from
//! the session files in `<config>/sessions/` and
//! `<config>/archived_sessions/`, plain or compressed.
//!
//! The answers and their tokens are read by the same code as the tokens
//! OTTY adds up (`spend_file.rs`). A session is the file whose first line
//! names its id; a project is every session file whose first line names
//! its folder. The sessions of helper agents are not listed, so they are
//! not counted either.

use std::collections::BTreeMap;

use otty_plugin_sdk::files::{self, FileInfo};
use otty_plugin_sdk::{
    Answer, HostError, ModelTokens, ProjectStats, ReadError, SessionStats,
    TokenCount,
};
use serde_json::Value;

use crate::session_file::read_session_meta;
use crate::sessions::{files_of_session, newest_first, session_files};
use crate::spend_file::{ANSWER_MARKERS, Line, SessionTokens, read_answers};
use crate::timestamp::millis_of;

/// What marks a message the user typed.
const USER_MESSAGE_MARKER: &[u8] = b"\"user_message\"";

/// What the lines of one session file told: the answers, the messages of
/// the user, and the first and the last time.
#[derive(Debug, Default)]
struct SessionLines {
    tokens: SessionTokens,
    messages: u64,
    first_ms: Option<i64>,
    last_ms: Option<i64>,
}

impl SessionLines {
    /// Takes what `line` tells: its time, a message of the user and the
    /// tokens of a `token_count`. A broken line is skipped.
    fn add_line(&mut self, line: &[u8]) {
        let Ok(line) = serde_json::from_slice::<Line>(line) else {
            return;
        };

        if let Some(at_ms) = line.timestamp().and_then(millis_of) {
            self.first_ms = earliest(self.first_ms, Some(at_ms));
            self.last_ms = latest(self.last_ms, Some(at_ms));
        }
        let is_user_message = line.kind() == Some("event_msg")
            && line
                .payload()
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str)
                == Some("user_message");
        if is_user_message {
            self.messages += 1;
        }

        self.tokens.add(&line);
    }
}

/// The statistics of the session `session` of the profile in `folder`.
/// Of the copies of a session, plain and compressed or archived, the one
/// that changed last is read. A session with no file is not found.
pub(crate) fn session_stats(
    folder: &str,
    session: &str,
) -> Result<SessionStats, ReadError> {
    let found = files_of_session(folder, session).map_err(ReadError::Host)?;

    let copies: Vec<FileInfo> = found
        .into_iter()
        .filter(|file| {
            read_session_meta(&file.path)
                .is_some_and(|meta| meta.id() == session)
        })
        .collect();
    let Some(newest) = newest_first(copies).into_iter().next() else {
        return Err(ReadError::Host(HostError::NotFound(format!(
            "the session file of {session}"
        ))));
    };

    let lines = read_lines(&newest.path)?;

    Ok(stats_of_session(lines))
}

/// The statistics of the project folder `working_folder` over the whole
/// history of the profile in `folder`. The same answer in two copies of a
/// session counts once.
pub(crate) fn project_stats(
    folder: &str,
    working_folder: &str,
) -> Result<ProjectStats, ReadError> {
    let found = session_files(folder).map_err(ReadError::Host)?;

    let answers: Vec<Answer> = found
        .iter()
        .filter(|file| {
            read_session_meta(&file.path)
                .is_some_and(|meta| meta.folder() == working_folder)
        })
        .filter_map(|file| read_answers(&file.path).ok())
        .flat_map(|read| read.answers)
        .collect();
    let (first_ms, last_ms, models) = history(&answers);

    Ok(ProjectStats {
        first_ms,
        last_ms,
        models,
    })
}

/// What the lines of the session file at `path` tell. A file that can
/// not be read fails; a file that breaks in the middle gives what was
/// read before.
fn read_lines(path: &str) -> Result<SessionLines, ReadError> {
    let markers: Vec<&[u8]> = ANSWER_MARKERS
        .iter()
        .copied()
        .chain([USER_MESSAGE_MARKER])
        .collect();

    let mut lines = SessionLines::default();
    for (number, batch) in files::lines(path, &markers).enumerate() {
        let batch = match batch {
            Ok(batch) => batch,
            Err(error) if number == 0 => return Err(ReadError::Host(error)),
            Err(_) => break,
        };
        for line in batch.lines() {
            lines.add_line(line);
        }
    }

    Ok(lines)
}

/// The statistics of a session from what the lines of its file told.
fn stats_of_session(lines: SessionLines) -> SessionStats {
    let answers = lines.tokens.into_answers().answers;
    let (_, _, models) = history(&answers);

    SessionStats {
        started_ms: lines.first_ms,
        last_active_ms: lines.last_ms,
        messages: lines.messages,
        models,
    }
}

/// The time of the first and of the last answer of `answers`, and the
/// tokens and answers of each model, the most first. Answers with the
/// same identity count once: the one with the most output tokens.
fn history(answers: &[Answer]) -> (Option<i64>, Option<i64>, Vec<ModelTokens>) {
    let mut by_identity: BTreeMap<&str, &Answer> = BTreeMap::new();
    let mut counted: Vec<&Answer> = Vec::new();
    for answer in answers {
        let Some(identity) = answer.identity.as_deref() else {
            counted.push(answer);
            continue;
        };
        let is_better = by_identity
            .get(identity)
            .is_none_or(|kept| answer.tokens.output > kept.tokens.output);
        if is_better {
            by_identity.insert(identity, answer);
        }
    }
    counted.extend(by_identity.into_values());

    let mut by_model: BTreeMap<Option<&str>, ModelTokens> = BTreeMap::new();
    let mut first_ms = None;
    let mut last_ms = None;
    for answer in counted {
        let model =
            by_model.entry(answer.model.as_deref()).or_insert_with(|| {
                ModelTokens {
                    model: answer.model.clone().unwrap_or_default(),
                    tokens: TokenCount::default(),
                    answers: 0,
                }
            });
        add_tokens(&mut model.tokens, &answer.tokens);
        model.answers += 1;
        first_ms = earliest(first_ms, Some(answer.at_ms));
        last_ms = latest(last_ms, Some(answer.at_ms));
    }

    let mut models: Vec<ModelTokens> = by_model.into_values().collect();
    models.sort_by(|one, other| {
        total(&other.tokens)
            .cmp(&total(&one.tokens))
            .then_with(|| one.model.is_empty().cmp(&other.model.is_empty()))
            .then_with(|| one.model.cmp(&other.model))
    });

    (first_ms, last_ms, models)
}

/// Adds `more` to `tokens`.
fn add_tokens(tokens: &mut TokenCount, more: &TokenCount) {
    tokens.input = tokens.input.saturating_add(more.input);
    tokens.cached = tokens.cached.saturating_add(more.cached);
    tokens.cache_written =
        tokens.cache_written.saturating_add(more.cache_written);
    tokens.output = tokens.output.saturating_add(more.output);
    tokens.reasoning = tokens.reasoning.saturating_add(more.reasoning);
}

/// Every token of `tokens`: input, cached and output.
fn total(tokens: &TokenCount) -> u64 {
    tokens
        .input
        .saturating_add(tokens.cached)
        .saturating_add(tokens.output)
}

/// The earlier of two times; either may be missing.
fn earliest(one: Option<i64>, other: Option<i64>) -> Option<i64> {
    match (one, other) {
        (Some(one), Some(other)) => Some(one.min(other)),
        _ => one.or(other),
    }
}

/// The later of two times; either may be missing.
fn latest(one: Option<i64>, other: Option<i64>) -> Option<i64> {
    one.max(other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spend_file::testing::tokens;
    use crate::testing::{
        BLOG_SESSION, SHOP_SESSION, TEN, as_part, host, path,
    };

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

    fn model(name: &str, tokens: TokenCount, answers: u64) -> ModelTokens {
        ModelTokens {
            model: name.to_string(),
            tokens,
            answers,
        }
    }

    /// What a session file of `lines` tells, as the host hands out its
    /// lines.
    fn session_lines(lines: &[String]) -> SessionLines {
        let mut read = SessionLines::default();
        for line in lines {
            read.add_line(line.as_bytes());
        }
        read
    }

    /// The answers of a session file of `lines`.
    fn answers_of(lines: &[String]) -> Vec<Answer> {
        session_lines(lines).tokens.into_answers().answers
    }

    #[test]
    fn a_session_gives_its_times_messages_and_tokens() {
        let lines = session_lines(&[
            meta("a1", "/home/user/otty", "2026-11-16T10:00:00.000Z"),
            turn("gpt-5-codex"),
            user("2026-11-16T10:00:02.000Z", "Fix the tabs"),
            count(
                "2026-11-16T10:01:00.000Z",
                (1000, 800, 50),
                (1000, 800, 50),
                Some((12.0, 40.0)),
            ),
            String::from(
                "{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\"",
            ),
            user("2026-11-16T10:20:00.000Z", "And the docs"),
            count(
                "2026-11-16T10:30:00.000Z",
                (300, 100, 20),
                (1300, 900, 70),
                Some((31.0, 42.5)),
            ),
        ]);

        let stats = stats_of_session(lines);

        assert_eq!(stats.started_ms, Some(TEN));
        assert_eq!(stats.last_active_ms, Some(TEN + 1_800_000));
        assert_eq!(stats.messages, 2);
        assert_eq!(
            stats.models,
            [model("gpt-5-codex", tokens(400, 900, 70), 2)]
        );
    }

    #[test]
    fn a_session_with_no_token_lines_has_no_tokens() {
        let lines = session_lines(&[
            meta("a3", "/home/user/otty", "2026-11-16T10:00:00.000Z"),
            user("2026-11-16T10:00:30.000Z", "Hello"),
        ]);

        let stats = stats_of_session(lines);

        assert_eq!(stats.messages, 1);
        assert!(stats.models.is_empty());
        assert_eq!(stats.started_ms, Some(TEN));
        assert_eq!(stats.last_active_ms, Some(TEN + 30_000));
    }

    #[test]
    fn a_session_file_is_read_with_its_messages_and_models() {
        let host = host();
        let folder = path(&host, ".codex");

        let stats = as_part(&host, || session_stats(&folder, SHOP_SESSION))
            .expect("the statistics");

        assert_eq!(stats.started_ms, Some(TEN));
        assert_eq!(stats.last_active_ms, Some(TEN + 1_800_000));
        assert_eq!(stats.messages, 2);
        assert_eq!(
            stats.models,
            [
                model(
                    "gpt-5-codex",
                    TokenCount {
                        reasoning: 20,
                        ..tokens(200, 800, 50)
                    },
                    1
                ),
                model(
                    "gpt-5.1-codex",
                    TokenCount {
                        reasoning: 5,
                        ..tokens(200, 100, 20)
                    },
                    1
                ),
            ]
        );
    }

    /// The compressed file and its archived copy hold the same lines;
    /// one of them is read.
    #[test]
    fn of_two_copies_of_a_session_one_is_read() {
        let host = host();
        let folder = path(&host, ".codex");

        let stats = as_part(&host, || session_stats(&folder, BLOG_SESSION))
            .expect("the statistics");

        assert_eq!(stats.messages, 1);
        assert_eq!(stats.models, [model("o3", tokens(600, 300, 65), 2)]);
    }

    #[test]
    fn a_session_with_no_file_is_not_found() {
        let host = host();
        let folder = path(&host, ".codex");

        let stats = as_part(&host, || session_stats(&folder, "gone"));

        assert!(matches!(
            stats,
            Err(ReadError::Host(HostError::NotFound(_)))
        ));
    }

    #[test]
    fn a_project_counts_its_sessions_over_the_whole_history() {
        let answer = |time: &str, output: u64| {
            count(time, (10, 0, output), (10, 0, output), None)
        };
        let old = answers_of(&[
            meta("old", "/home/user/otty", "2025-01-02T09:00:00.000Z"),
            turn("o3"),
            answer("2025-01-02T09:01:00.000Z", 1),
        ]);
        let new = answers_of(&[
            meta("new", "/home/user/otty", "2026-11-16T10:00:00.000Z"),
            turn("gpt-5-codex"),
            answer("2026-11-16T10:01:00.000Z", 2),
        ]);
        let answers: Vec<Answer> = old.into_iter().chain(new).collect();

        let (first_ms, last_ms, models) = history(&answers);

        assert_eq!(first_ms, millis_of("2025-01-02T09:01:00.000Z"));
        assert_eq!(last_ms, Some(TEN + 60_000));
        assert_eq!(
            models,
            [
                model("gpt-5-codex", tokens(10, 0, 2), 1),
                model("o3", tokens(10, 0, 1), 1),
            ]
        );
    }

    /// The helper agent's session in the same folder is not one of the
    /// project's sessions.
    #[test]
    fn a_project_reads_only_the_sessions_of_its_folder() {
        let host = host();
        let folder = path(&host, ".codex");

        let stats = as_part(&host, || project_stats(&folder, "/home/dev/shop"))
            .expect("the statistics");

        assert_eq!(stats.first_ms, Some(TEN + 60_000));
        assert_eq!(stats.last_ms, Some(TEN + 1_800_000));
        let names: Vec<&str> = stats
            .models
            .iter()
            .map(|model| model.model.as_str())
            .collect();
        assert_eq!(names, ["gpt-5-codex", "gpt-5.1-codex"]);
    }

    /// The compressed file and its archived copy hold the same answers;
    /// they count once.
    #[test]
    fn the_same_answer_in_two_copies_counts_once() {
        let host = host();
        let folder = path(&host, ".codex");

        let stats = as_part(&host, || project_stats(&folder, "/home/dev/blog"))
            .expect("the statistics");

        assert_eq!(stats.models, [model("o3", tokens(600, 300, 65), 2)]);
    }

    #[test]
    fn a_profile_with_no_session_folder_has_an_empty_project() {
        let host = host();
        let folder = path(&host, ".codex-nomode");

        let stats = as_part(&host, || project_stats(&folder, "/home/dev/shop"))
            .expect("the statistics");

        assert_eq!(
            stats,
            ProjectStats {
                first_ms: None,
                last_ms: None,
                models: Vec::new(),
            }
        );
    }
}
