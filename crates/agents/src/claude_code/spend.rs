//! The tokens a Claude Code profile spent, counted from its session files
//! in `<config>/projects/`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::project_folder::{fallback_project, project_dirs};
use super::spend_file::read_spend;
use crate::error::SourceError;
use crate::file_memory::FileMemory;
use crate::file_tree::files_in_tree;
use crate::local_time::LocalNow;
use crate::profile::Profile;
use crate::spend::{Spend, SpendCounter};
use crate::spend_files::{FileSpend, count_spend};

/// Counts the tokens of a profile from every `*.jsonl` file in the project
/// folders of `<config>/projects/`, at any depth, so the files of helper
/// agents in `<session id>/subagents/` count too, under the project of
/// their session.
pub(super) struct SessionSpend {
    cache: Arc<FileMemory<FileSpend>>,
}

impl SessionSpend {
    /// A counter that remembers what it counted in `cache`.
    pub(super) fn new(cache: Arc<FileMemory<FileSpend>>) -> Self {
        Self { cache }
    }
}

impl SpendCounter for SessionSpend {
    /// A missing `projects` folder means nothing was spent; a file in its
    /// place is "not available", and a folder that can not be read is
    /// "unreadable".
    fn spend(
        &self,
        profile: &Profile,
        now: &LocalNow,
    ) -> Result<Spend, SourceError> {
        let projects_dir = profile.config_dir().join("projects");

        let paths = project_dirs(profile.config_dir())?
            .iter()
            .flat_map(|project_dir| session_files_in(project_dir))
            .collect();

        let spend = count_spend(
            &self.cache,
            profile.config_dir(),
            paths,
            now,
            |path| read_spend(path, &fallback_project(&projects_dir, path)),
        );

        Ok(spend)
    }
}

/// The `*.jsonl` files in `folder` and in the folders inside it, at any
/// depth; none when `folder` can not be read.
pub(super) fn session_files_in(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };

    files_in_tree(entries, |path| {
        path.extension().is_some_and(|ext| ext == "jsonl")
    })
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;
    use crate::testing::{
        NOW, TempDir, at, claude_profile, set_file_time, utc_now, write_file,
    };
    use crate::token_count::TokenCount;

    /// A made-up Claude Code config folder and a counter for it.
    struct ConfigDir {
        temp: TempDir,
        counter: SessionSpend,
    }

    impl ConfigDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join(".claude-work")
        }

        fn new() -> Self {
            let config = Self {
                temp: TempDir::new(),
                counter: SessionSpend::new(Arc::default()),
            };
            std::fs::create_dir_all(config.path().join("projects"))
                .expect("create projects folder");
            config
        }

        /// Writes `lines` to `projects/<file>` and sets its change time to
        /// `changed`.
        fn write(&self, file: &str, lines: &[String], changed: SystemTime) {
            let path = self.path().join("projects").join(file);
            write_file(&path, &lines.join("\n"));
            set_file_time(&path, changed);
        }

        fn spend_at(&self, now: &LocalNow) -> Spend {
            self.counter
                .spend(&claude_profile(&self.path()), now)
                .expect("the session files are counted")
        }

        fn spend(&self) -> Spend {
            self.spend_at(&utc_now())
        }
    }

    fn answer(id: &str, time: &str, output: u64) -> String {
        format!(
            r#"{{"type":"assistant","cwd":"/home/user/otty","sessionId":"s1","requestId":"req_{id}","timestamp":"{time}","message":{{"id":"msg_{id}","model":"claude-sonnet-4-5","usage":{{"input_tokens":10,"cache_read_input_tokens":100,"output_tokens":{output}}}}}}}"#
        )
    }

    fn output(count: u64) -> TokenCount {
        TokenCount::new(10, 100, count)
    }

    #[test]
    fn a_session_with_several_answers_is_added_up() {
        let config = ConfigDir::new();
        config.write(
            "-home-user-otty/s1.jsonl",
            &[
                answer("1", "2026-11-16T09:00:00Z", 1),
                answer("2", "2026-11-16T09:30:00Z", 2),
            ],
            at(NOW),
        );

        let spend = config.spend();

        assert_eq!(spend.today(), output(1) + output(2));
        let projects = spend.projects();
        assert_eq!(projects[0].folder(), Path::new("/home/user/otty"));
        let models = spend.this_month_by_model();
        assert_eq!(models[0].model(), Some("claude-sonnet-4-5"));
    }

    #[test]
    fn the_same_answer_written_twice_is_counted_once() {
        let config = ConfigDir::new();
        let time = "2026-11-16T09:00:00Z";
        config.write(
            "-home-user-otty/s1.jsonl",
            &[answer("1", time, 1), answer("1", time, 7)],
            at(NOW),
        );
        config.write(
            "-home-user-otty/s1-resumed.jsonl",
            &[answer("1", time, 7), answer("2", time, 2)],
            at(NOW),
        );

        let spend = config.spend();

        assert_eq!(spend.today(), output(7) + output(2));
    }

    #[test]
    fn records_count_on_their_side_of_midnight_and_of_the_30_days() {
        let config = ConfigDir::new();
        config.write(
            "-home-user-otty/s1.jsonl",
            &[
                answer("1", "2026-10-17T23:59:59Z", 1000),
                answer("2", "2026-10-18T00:00:00Z", 1),
                answer("3", "2026-11-15T23:59:59Z", 2),
                answer("4", "2026-11-16T00:00:00Z", 3),
            ],
            at(NOW),
        );

        let spend = config.spend();

        assert_eq!(spend.today(), output(3));
        assert_eq!(spend.last_7_days(), output(2) + output(3));
        assert_eq!(spend.last_30_days(), output(1) + output(2) + output(3));
    }

    #[test]
    fn a_helper_agents_file_counts_under_its_project() {
        let config = ConfigDir::new();
        let helper = r#"{"type":"assistant","timestamp":"2026-11-16T09:00:00Z","message":{"id":"msg_h","usage":{"output_tokens":5}}}"#;
        config.write(
            "-home-user-otty/s1.jsonl",
            &[answer("1", "2026-11-16T09:00:00Z", 1)],
            at(NOW),
        );
        config.write(
            "-home-user-otty/s1/subagents/agent-a1.jsonl",
            &[String::from(helper)],
            at(NOW),
        );

        let spend = config.spend();

        assert_eq!(spend.today(), output(1) + TokenCount::new(0, 0, 5));
        assert_eq!(spend.projects().len(), 1);
        assert_eq!(spend.projects()[0].tokens(), spend.today());
    }

    #[test]
    fn a_file_that_did_not_change_is_not_counted_again() {
        let config = ConfigDir::new();
        let file = "-home-user-otty/s1.jsonl";
        config.write(file, &[answer("1", "2026-11-16T09:00:00Z", 1)], at(NOW));

        let first = config.spend();
        config.write(file, &[answer("1", "2026-11-16T09:00:00Z", 9)], at(NOW));
        let second = config.spend();
        config.write(
            file,
            &[answer("1", "2026-11-16T09:00:00Z", 9)],
            at(NOW + 1),
        );
        let third = config.spend();

        assert_eq!(first.today(), output(1));
        assert_eq!(second.today(), output(1));
        assert_eq!(third.today(), output(9));
    }

    #[test]
    fn a_file_not_changed_in_30_days_is_not_counted() {
        let config = ConfigDir::new();
        config.write(
            "-home-user-otty/s1.jsonl",
            &[answer("1", "2026-11-16T09:00:00Z", 1)],
            at(NOW - 40 * 24 * 3600),
        );

        assert!(config.spend().today().is_zero());
    }

    #[test]
    fn other_files_are_not_counted() {
        let config = ConfigDir::new();
        let line = answer("1", "2026-11-16T09:00:00Z", 1);
        config.write(
            "-home-user-otty/notes.txt",
            std::slice::from_ref(&line),
            at(NOW),
        );
        write_file(&config.path().join("history.jsonl"), &line);
        config.write("stray.jsonl", std::slice::from_ref(&line), at(NOW));

        assert!(config.spend().today().is_zero());
    }

    #[test]
    fn a_missing_projects_folder_spent_nothing() {
        let temp = TempDir::new();
        let counter = SessionSpend::new(Arc::default());
        let profile = claude_profile(&temp.path().join(".claude-gone"));

        let spend = counter.spend(&profile, &utc_now());

        let spend = spend.expect("nothing to count is not a failure");
        assert!(spend.last_30_days().is_zero());
        assert_eq!(spend.last_30_days_by_day().len(), 30);
    }

    #[test]
    fn a_projects_path_that_is_not_a_folder_is_not_available() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-work");
        write_file(&config_dir.join("projects"), "not a folder");
        let counter = SessionSpend::new(Arc::default());

        let spend = counter.spend(&claude_profile(&config_dir), &utc_now());

        assert!(matches!(spend, Err(SourceError::NotAvailable(_))));
    }
}
