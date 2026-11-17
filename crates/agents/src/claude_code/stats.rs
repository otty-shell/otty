//! The statistics of a Claude Code session and of a project folder, read
//! from the session files in `<config>/projects/`.
//!
//! The answers and their tokens are read by the same code as the tokens of
//! the dashboard (`spend_file.rs`), and the prompts by the same code as the
//! session titles (`session_file.rs`).

use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::session_file::prompt_time_on_line;
use super::sessions::decoded_folder;
use super::spend::{add_session_files, fallback_project};
use super::spend_file::{answer_on_line, read_spend};
use crate::error::SourceError;
use crate::profile::Profile;
use crate::session::SessionId;
use crate::spend_files::FileSpend;
use crate::spend_tally::whole_history;
use crate::stats::{ProjectStats, SessionStats, StatsSource};

/// What the main file of a session told: its answers, and how many prompts
/// the user typed between which times.
#[derive(Debug, Default)]
struct MainFile {
    spend: FileSpend,
    prompts: u64,
    first_prompt_at: Option<SystemTime>,
    last_prompt_at: Option<SystemTime>,
}

/// Reads the statistics from the session files.
///
/// A session is the file `projects/<folder>/<session id>.jsonl`, with the
/// files of its helper agents in `projects/<folder>/<session id>/`, whose
/// tokens count too. A project is every session file whose folder is the
/// project folder.
pub(super) struct SessionFilesStats;

impl StatsSource for SessionFilesStats {
    /// A session with no file is "not available".
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
    ) -> Result<SessionStats, SourceError> {
        let projects_dir = profile.config_dir().join("projects");
        let id = session.as_str();

        let Some(path) = session_file(&projects_dir, id) else {
            return Err(SourceError::NotAvailable(format!(
                "no session file of {id}"
            )));
        };
        let main = read_main_file(&path);
        let helpers: Vec<FileSpend> = helper_files(&path)
            .iter()
            .map(|helper| {
                read_spend(helper, &fallback_project(&projects_dir, helper))
            })
            .collect();

        let history =
            whole_history(std::iter::once(&main.spend).chain(&helpers));
        let started_at = earliest(main.first_prompt_at, history.first_at());
        let last_active_at = latest(main.last_prompt_at, history.last_at());

        Ok(SessionStats::new(
            started_at,
            last_active_at,
            main.prompts,
            history.models().to_vec(),
        ))
    }

    /// Only the folders of `projects` named after `folder` are read. A
    /// missing `projects` folder has nothing; one that can not be read is
    /// "unreadable".
    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
    ) -> Result<ProjectStats, SourceError> {
        let projects_dir = profile.config_dir().join("projects");

        let entries = match std::fs::read_dir(&projects_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(ProjectStats::default());
            },
            Err(error) => {
                let shown = projects_dir.display();
                return Err(SourceError::Unreadable(format!(
                    "{shown}: {error}"
                )));
            },
        };
        let mut paths = Vec::new();
        for project_dir in entries.flatten() {
            let path = project_dir.path();
            if !project_dir.file_type().is_ok_and(|kind| kind.is_dir())
                || !is_folder_of(&path, folder)
            {
                continue;
            }
            if let Ok(inner) = std::fs::read_dir(&path) {
                add_session_files(inner, &mut paths);
            }
        }

        let files: Vec<FileSpend> = paths
            .iter()
            .map(|path| {
                read_spend(path, &fallback_project(&projects_dir, path))
            })
            .filter(|file| file.project() == Some(folder))
            .collect();

        Ok(whole_history(files.iter()))
    }
}

/// The file of the session `id`: `<id>.jsonl` in one of the project
/// folders of `projects_dir`.
fn session_file(projects_dir: &Path, id: &str) -> Option<PathBuf> {
    let file_name = format!("{id}.jsonl");

    std::fs::read_dir(projects_dir)
        .ok()?
        .flatten()
        .map(|project_dir| project_dir.path().join(&file_name))
        .find(|path| path.is_file())
}

/// The files of the helper agents of the session kept at `path`: every
/// `*.jsonl` file in the folder named after the session, next to it.
fn helper_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path.with_extension("")) {
        add_session_files(entries, &mut files);
    }

    files
}

/// The answers and the prompts of the session file at `path`; nothing for
/// a file that can not be opened. A broken line is skipped.
fn read_main_file(path: &Path) -> MainFile {
    let Ok(file) = File::open(path) else {
        return MainFile::default();
    };

    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut main = MainFile::default();
    let mut answers = Vec::new();
    while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
        if let Some(answer) = answer_on_line(&line) {
            answers.push(answer);
        } else if let Some(at) = prompt_time_on_line(&line) {
            main.prompts += 1;
            main.first_prompt_at = earliest(main.first_prompt_at, Some(at));
            main.last_prompt_at = latest(main.last_prompt_at, Some(at));
        }
        line.clear();
    }

    main.spend = FileSpend::new(None, answers);
    main
}

/// Whether the folder `project_dir` of `projects` holds the sessions of
/// `folder`: Claude Code names it after the path with every character
/// that is not a letter or a digit turned into `-`. A name the session
/// list would turn back into `folder` counts too.
fn is_folder_of(project_dir: &Path, folder: &Path) -> bool {
    let name = project_dir
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let encoded: String = folder
        .to_string_lossy()
        .chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() {
                letter
            } else {
                '-'
            }
        })
        .collect();

    name == encoded || decoded_folder(project_dir) == folder
}

/// The earlier of two times; either may be missing.
fn earliest(
    a: Option<SystemTime>,
    b: Option<SystemTime>,
) -> Option<SystemTime> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        _ => a.or(b),
    }
}

/// The later of two times; either may be missing.
fn latest(a: Option<SystemTime>, b: Option<SystemTime>) -> Option<SystemTime> {
    a.max(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spend::ModelTokens;
    use crate::testing::{TempDir, claude_profile, write_file};
    use crate::token_count::TokenCount;

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;

    /// A made-up Claude Code config folder.
    struct ConfigDir {
        temp: TempDir,
    }

    impl ConfigDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join(".claude-work")
        }

        fn new() -> Self {
            Self {
                temp: TempDir::new(),
            }
        }

        /// Writes `lines` to `projects/<file>`.
        fn write(&self, file: &str, lines: &[String]) {
            write_file(
                &self.path().join("projects").join(file),
                &lines.join("\n"),
            );
        }

        fn session(&self, id: &str) -> Result<SessionStats, SourceError> {
            SessionFilesStats.session_stats(
                &claude_profile(&self.path()),
                &SessionId::new(id),
            )
        }

        fn project(&self, folder: &str) -> Result<ProjectStats, SourceError> {
            SessionFilesStats
                .project_stats(&claude_profile(&self.path()), Path::new(folder))
        }
    }

    fn prompt(time: &str, text: &str) -> String {
        format!(
            r#"{{"type":"user","cwd":"/home/user/otty","sessionId":"s1","timestamp":"{time}","message":{{"role":"user","content":"{text}"}}}}"#
        )
    }

    fn answer(id: &str, time: &str, model: &str, output: u64) -> String {
        format!(
            r#"{{"type":"assistant","cwd":"/home/user/otty","sessionId":"s1","requestId":"req_{id}","timestamp":"{time}","message":{{"id":"msg_{id}","model":"{model}","usage":{{"input_tokens":10,"cache_read_input_tokens":100,"output_tokens":{output}}}}}}}"#
        )
    }

    fn time(seconds_after_now: u64) -> SystemTime {
        crate::testing::at(NOW + seconds_after_now)
    }

    #[test]
    fn a_session_gives_its_times_prompts_answers_and_tokens_by_model() {
        let config = ConfigDir::new();
        let tool_result = r#"{"type":"user","timestamp":"2026-11-16T10:00:30Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}"#;
        let meta = r#"{"type":"user","isMeta":true,"timestamp":"2026-11-16T10:00:31Z","message":{"role":"user","content":"Caveat"}}"#;
        let command = r#"{"type":"user","timestamp":"2026-11-16T10:00:32Z","message":{"role":"user","content":"<command-name>/clear</command-name>"}}"#;
        config.write(
            "-home-user-otty/s1.jsonl",
            &[
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
            ],
        );
        config.write(
            "-home-user-otty/s1/subagents/agent-a1.jsonl",
            &[answer("h", "2026-11-16T10:35:00Z", "claude-haiku-4-5", 5)],
        );

        let stats = config.session("s1").expect("the session is read");

        assert_eq!(stats.started_at(), Some(time(0)));
        assert_eq!(stats.last_active_at(), Some(time(2400)));
        assert_eq!(stats.messages(), 2);
        assert_eq!(stats.answers(), 3);
        assert_eq!(
            stats.models(),
            [
                ModelTokens::new(
                    Some("claude-haiku-4-5"),
                    TokenCount::new(20, 200, 8),
                    2
                ),
                ModelTokens::new(
                    Some("claude-opus-4-5"),
                    TokenCount::new(10, 100, 7),
                    1
                ),
            ]
        );
        assert!(stats.limit_changes().is_empty());
    }

    #[test]
    fn a_session_with_no_token_lines_has_its_prompts_and_no_tokens() {
        let config = ConfigDir::new();
        config.write(
            "-home-user-otty/s2.jsonl",
            &[
                prompt("2026-11-16T10:00:00Z", "Hello"),
                prompt("2026-11-16T10:05:00Z", "Anyone?"),
            ],
        );

        let stats = config.session("s2").expect("the session is read");

        assert_eq!(stats.messages(), 2);
        assert_eq!(stats.answers(), 0);
        assert!(stats.models().is_empty());
        assert_eq!(stats.length(), Some(std::time::Duration::from_secs(300)));
    }

    #[test]
    fn a_session_with_no_file_is_not_available() {
        let config = ConfigDir::new();
        config.write(
            "-home-user-otty/s1.jsonl",
            &[prompt("2026-11-16T10:00:00Z", "Hi")],
        );

        let stats = config.session("gone");

        assert!(matches!(stats, Err(SourceError::NotAvailable(_))));
    }

    #[test]
    fn a_project_counts_the_whole_history_of_its_folder_only() {
        let config = ConfigDir::new();
        config.write(
            "-home-user-otty/s1.jsonl",
            &[
                answer("1", "2025-01-02T09:00:00Z", "claude-opus-4-5", 1),
                answer("2", "2026-11-16T10:00:00Z", "claude-opus-4-5", 2),
            ],
        );
        config.write(
            "-home-user-otty/s1-resumed.jsonl",
            &[
                answer("2", "2026-11-16T10:00:00Z", "claude-opus-4-5", 2),
                answer("3", "2026-11-16T11:00:00Z", "claude-sonnet-4-5", 4),
            ],
        );
        config.write(
            "-home-user-docs/d1.jsonl",
            &[answer("9", "2026-11-16T10:00:00Z", "claude-opus-4-5", 100)],
        );

        let stats = config
            .project("/home/user/otty")
            .expect("the project is read");

        assert_eq!(
            stats.first_at(),
            crate::timestamp::parse_rfc3339("2025-01-02T09:00:00Z")
        );
        assert_eq!(stats.last_at(), Some(time(3600)));
        assert_eq!(
            stats.models(),
            [
                ModelTokens::new(
                    Some("claude-opus-4-5"),
                    TokenCount::new(20, 200, 3),
                    2
                ),
                ModelTokens::new(
                    Some("claude-sonnet-4-5"),
                    TokenCount::new(10, 100, 4),
                    1
                ),
            ]
        );
    }

    #[test]
    fn a_profile_with_no_projects_folder_has_an_empty_project() {
        let config = ConfigDir::new();

        let stats = config.project("/home/user/otty");

        assert_eq!(stats, Ok(ProjectStats::default()));
    }
}
