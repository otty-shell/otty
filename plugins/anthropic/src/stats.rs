//! The statistics of a Claude Code session and of a project folder, read
//! from the session files in `<config>/projects/`.
//!
//! The answers and their tokens are read by the same code as the tokens
//! OTTY adds up (`spend_file.rs`), and the prompts by the same code as the
//! session titles (`session_file.rs`). A session is the file
//! `projects/<folder>/<session id>.jsonl` with the files of its helper
//! agents in `projects/<folder>/<session id>/`, whose tokens count too. A
//! project is every session file whose working folder is the project
//! folder.

use std::collections::BTreeMap;

use otty_plugin_sdk::{
    Answer, HostError, ModelTokens, ProjectStats, ReadError, SessionStats,
    TokenCount, files,
};

use crate::glob::escaped;
use crate::project_folder::{project_folder_names, projects_folder};
use crate::session_file::{USER_MARKER, prompt_at_ms};
use crate::spend_file::{ANSWER_MARKERS, answer_on_line, read_answers};

/// What the main file of a session told: its answers, and how many
/// prompts the user typed between which times.
#[derive(Debug, Default)]
struct MainFile {
    answers: Vec<Answer>,
    prompts: u64,
    first_prompt_ms: Option<i64>,
    last_prompt_ms: Option<i64>,
}

impl MainFile {
    /// Takes the answer or the prompt on `line`.
    fn add_line(&mut self, line: &[u8]) {
        if let Some(answer) = answer_on_line(line) {
            self.answers.push(answer);
        } else if let Some(at_ms) = prompt_at_ms(line) {
            self.prompts += 1;
            self.first_prompt_ms = earliest(self.first_prompt_ms, Some(at_ms));
            self.last_prompt_ms = latest(self.last_prompt_ms, Some(at_ms));
        }
    }
}

/// The statistics of the session `session` of the profile in `folder`.
/// A session with no file is not found.
pub(crate) fn session_stats(
    folder: &str,
    session: &str,
) -> Result<SessionStats, ReadError> {
    let projects = escaped(&projects_folder(folder));
    let glob = format!("{projects}/*/{}.jsonl", escaped(session));
    let found = files::list(&glob).map_err(ReadError::Host)?;
    let Some(main_path) = found.first().map(|file| file.path.as_str()) else {
        return Err(ReadError::Host(HostError::NotFound(format!(
            "the session file {session}.jsonl"
        ))));
    };

    let main = read_main_file(main_path)?;
    // The files of the helper agents are in the folder named after the
    // session, next to its file.
    let helper_folder = main_path.strip_suffix(".jsonl").unwrap_or(main_path);
    let helpers =
        answers_in(folder, &format!("{}/**/*.jsonl", escaped(helper_folder)));

    Ok(stats_of_session(main, &helpers))
}

/// The statistics of the project folder `working_folder` over the whole
/// history of the profile in `folder`. Only the project folders named
/// after it are read; a missing `projects` folder has nothing.
pub(crate) fn project_stats(
    folder: &str,
    working_folder: &str,
) -> Result<ProjectStats, ReadError> {
    let projects = escaped(&projects_folder(folder));

    let answers: Vec<Answer> = project_folder_names(working_folder)
        .iter()
        .flat_map(|name| {
            answers_in(
                folder,
                &format!("{projects}/{}/**/*.jsonl", escaped(name)),
            )
        })
        .filter(|answer| answer.project.as_deref() == Some(working_folder))
        .collect();

    let (first_ms, last_ms, models) = history(&answers);

    Ok(ProjectStats {
        first_ms,
        last_ms,
        models,
    })
}

/// The statistics of a session from its main file and the answers of its
/// helper agents.
fn stats_of_session(main: MainFile, helpers: &[Answer]) -> SessionStats {
    let answers: Vec<Answer> = main
        .answers
        .into_iter()
        .chain(helpers.iter().cloned())
        .collect();
    let (first_ms, last_ms, models) = history(&answers);

    SessionStats {
        started_ms: earliest(main.first_prompt_ms, first_ms),
        last_active_ms: latest(main.last_prompt_ms, last_ms),
        messages: main.prompts,
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

/// The main file of a session at `path`: its answers and its prompts.
fn read_main_file(path: &str) -> Result<MainFile, ReadError> {
    let markers: Vec<&[u8]> = ANSWER_MARKERS
        .iter()
        .copied()
        .chain([USER_MARKER])
        .collect();

    let mut main = MainFile::default();
    for batch in files::lines(path, &markers) {
        let batch = batch.map_err(ReadError::Host)?;
        for line in batch.lines() {
            main.add_line(line);
        }
    }

    Ok(main)
}

/// The answers of the session files of the profile in `folder` that
/// `glob` lists; a file that can not be read has none.
fn answers_in(folder: &str, glob: &str) -> Vec<Answer> {
    let found = files::list(glob).unwrap_or_default();

    found
        .iter()
        .filter_map(|file| read_answers(folder, &file.path, None).ok())
        .flat_map(|read| read.answers)
        .collect()
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
    use crate::spend_file::SpendFile;
    use crate::testing::{as_part, host, path};

    /// 2026-11-16T10:00:00Z.
    const TEN: i64 = 1_794_823_200_000;

    fn prompt(time: &str, text: &str) -> String {
        format!(
            r#"{{"type":"user","cwd":"/home/dev/otty","sessionId":"s1","timestamp":"{time}","message":{{"role":"user","content":"{text}"}}}}"#
        )
    }

    fn answer(id: &str, time: &str, model: &str, output: u64) -> String {
        format!(
            r#"{{"type":"assistant","cwd":"/home/dev/otty","sessionId":"s1","requestId":"req_{id}","timestamp":"{time}","message":{{"id":"msg_{id}","model":"{model}","usage":{{"input_tokens":10,"cache_read_input_tokens":100,"output_tokens":{output}}}}}}}"#
        )
    }

    fn tokens(input: u64, cached: u64, output: u64) -> TokenCount {
        TokenCount {
            input,
            cached,
            output,
            ..TokenCount::default()
        }
    }

    fn model(name: &str, tokens: TokenCount, answers: u64) -> ModelTokens {
        ModelTokens {
            model: name.to_string(),
            tokens,
            answers,
        }
    }

    fn main_file(lines: &[String]) -> MainFile {
        let mut main = MainFile::default();
        for line in lines {
            main.add_line(line.as_bytes());
        }
        main
    }

    fn answers_of(lines: &[String]) -> Vec<Answer> {
        let mut file = SpendFile::default();
        for line in lines {
            file.add_line(line.as_bytes());
        }

        file.into_answers("/fallback", None).answers
    }

    #[test]
    fn a_session_gives_its_times_prompts_answers_and_tokens_by_model() {
        let tool_result = r#"{"type":"user","timestamp":"2026-11-16T10:00:30Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}"#;
        let meta = r#"{"type":"user","isMeta":true,"timestamp":"2026-11-16T10:00:31Z","message":{"role":"user","content":"Caveat"}}"#;
        let command = r#"{"type":"user","timestamp":"2026-11-16T10:00:32Z","message":{"role":"user","content":"<command-name>/clear</command-name>"}}"#;
        let main = main_file(&[
            String::from(r#"{"type":"summary","summary":"Fix the tabs"}"#),
            prompt("2026-11-16T10:00:00Z", "Fix the tabs"),
            answer("1", "2026-11-16T10:00:20Z", "claude-opus-4-5", 1),
            answer("1", "2026-11-16T10:00:20Z", "claude-opus-4-5", 7),
            String::from(tool_result),
            String::from(meta),
            String::from(command),
            String::from("{\"type\":\"user\",\"message\":"),
            prompt("2026-11-16T10:30:00Z", "Now the docs"),
            answer("2", "2026-11-16T10:40:00Z", "claude-haiku-4-5", 3),
        ]);
        let helpers = answers_of(&[answer(
            "h",
            "2026-11-16T10:35:00Z",
            "claude-haiku-4-5",
            5,
        )]);

        let stats = stats_of_session(main, &helpers);

        assert_eq!(stats.started_ms, Some(TEN));
        assert_eq!(stats.last_active_ms, Some(TEN + 2_400_000));
        assert_eq!(stats.messages, 2);
        assert_eq!(
            stats.models,
            [
                model("claude-haiku-4-5", tokens(20, 200, 8), 2),
                model("claude-opus-4-5", tokens(10, 100, 7), 1),
            ]
        );
    }

    #[test]
    fn a_session_with_no_token_lines_has_its_prompts_and_no_tokens() {
        let main = main_file(&[
            prompt("2026-11-16T10:00:00Z", "Hello"),
            prompt("2026-11-16T10:05:00Z", "Anyone?"),
        ]);

        let stats = stats_of_session(main, &[]);

        assert_eq!(stats.messages, 2);
        assert!(stats.models.is_empty());
        assert_eq!(stats.started_ms, Some(TEN));
        assert_eq!(stats.last_active_ms, Some(TEN + 300_000));
    }

    #[test]
    fn a_session_is_read_with_its_helper_agents() {
        let host = host();
        let folder = path(&host, ".claude");

        let stats = as_part(&host, || {
            session_stats(&folder, "1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b")
        })
        .expect("the statistics");

        assert_eq!(stats.started_ms, Some(TEN));
        assert_eq!(stats.last_active_ms, Some(TEN + 2_400_000));
        assert_eq!(stats.messages, 2);
        assert_eq!(
            stats.models,
            [
                model(
                    "claude-opus-4-5",
                    TokenCount {
                        input: 112,
                        cached: 2000,
                        cache_written: 100,
                        output: 30,
                        reasoning: 0,
                    },
                    1
                ),
                model("claude-haiku-4-5", tokens(10, 100, 5), 1),
                model("claude-sonnet-4-5", tokens(10, 100, 3), 1),
            ]
        );
    }

    #[test]
    fn a_session_with_no_file_is_not_found() {
        let host = host();
        let folder = path(&host, ".claude");

        let stats = as_part(&host, || session_stats(&folder, "gone"));

        assert!(matches!(
            stats,
            Err(ReadError::Host(HostError::NotFound(_)))
        ));
    }

    /// A resumed session copies the answers before it into its own file;
    /// they count once.
    #[test]
    fn a_project_counts_the_whole_history_of_its_folder_only() {
        let host = host();
        let folder = path(&host, ".claude");

        let stats = as_part(&host, || project_stats(&folder, "/home/dev/shop"))
            .expect("the statistics");

        assert_eq!(stats.first_ms, Some(TEN + 20_000));
        assert_eq!(stats.last_ms, Some(TEN + 3_630_000));
        assert_eq!(
            stats.models,
            [
                model(
                    "claude-opus-4-5",
                    TokenCount {
                        input: 112,
                        cached: 2000,
                        cache_written: 100,
                        output: 30,
                        reasoning: 0,
                    },
                    1
                ),
                model("claude-sonnet-4-5", tokens(20, 200, 7), 2),
                model("claude-haiku-4-5", tokens(10, 100, 5), 1),
            ]
        );
    }

    #[test]
    fn a_profile_with_no_projects_folder_has_an_empty_project() {
        let host = host();
        let folder = path(&host, ".claude-token");

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
