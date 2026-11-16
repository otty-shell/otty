//! The tokens a Codex profile spent, counted from its session files in
//! `<config>/sessions/` and `<config>/archived_sessions/`.

use std::sync::Arc;

use super::sessions::session_files;
use super::spend_file::read_spend;
use crate::error::SourceError;
use crate::local_time::LocalNow;
use crate::profile::Profile;
use crate::spend::{Spend, SpendCounter};
use crate::spend_files::{SpendCache, count_spend};

/// Counts the tokens of a profile from every session file, plain or
/// compressed, in both session folders. The files of helper agents count
/// too: they are session files like the others.
pub(super) struct SessionSpend {
    cache: Arc<SpendCache>,
}

impl SessionSpend {
    /// A counter that remembers what it counted in `cache`.
    pub(super) fn new(cache: Arc<SpendCache>) -> Self {
        Self { cache }
    }
}

impl SpendCounter for SessionSpend {
    /// A missing session folder means nothing was spent there; a session
    /// folder that can not be read is "not available".
    fn spend(
        &self,
        profile: &Profile,
        now: LocalNow,
    ) -> Result<Spend, SourceError> {
        let paths = session_files(profile.config_dir())?;

        let spend = count_spend(
            &self.cache,
            profile.config_dir(),
            paths,
            now,
            read_spend,
        );

        Ok(spend)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    use super::*;
    use crate::testing::{
        TempDir, at, codex_profile, set_file_time, write_compressed, write_file,
    };
    use crate::token_count::TokenCount;

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;

    /// A made-up Codex config folder and a counter for it.
    struct ConfigDir {
        temp: TempDir,
        counter: SessionSpend,
    }

    impl ConfigDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join(".codex-work")
        }

        fn new() -> Self {
            let config = Self {
                temp: TempDir::new(),
                counter: SessionSpend::new(Arc::default()),
            };
            std::fs::create_dir_all(config.path().join("sessions"))
                .expect("create sessions folder");
            config
        }

        fn file(&self, folder: &str, id: &str, ext: &str) -> PathBuf {
            self.path()
                .join(folder)
                .join(format!("rollout-2026-11-16T09-00-00-{id}.{ext}"))
        }

        /// Writes a plain session file of the session `id` in `folder`.
        fn write(
            &self,
            folder: &str,
            id: &str,
            lines: &[String],
            changed: SystemTime,
        ) {
            let path = self.file(folder, id, "jsonl");
            write_file(&path, &with_meta(id, lines));
            set_file_time(&path, changed);
        }

        fn spend(&self) -> Spend {
            self.counter
                .spend(&codex_profile(&self.path()), LocalNow::new(at(NOW), 0))
                .expect("the session files are counted")
        }
    }

    /// The content of a session file of `id`: its first line, a turn of
    /// `gpt-5-codex` and `lines`.
    fn with_meta(id: &str, lines: &[String]) -> String {
        let meta = format!(
            r#"{{"timestamp":"2026-11-16T09:00:00.000Z","type":"session_meta","payload":{{"id":"{id}","cwd":"/home/user/otty","source":"cli"}}}}"#
        );
        let turn = r#"{"timestamp":"2026-11-16T09:00:01.000Z","type":"turn_context","payload":{"model":"gpt-5-codex"}}"#;

        let mut content = format!("{meta}\n{turn}\n");
        for line in lines {
            content.push_str(line);
            content.push('\n');
        }
        content
    }

    /// A `token_count` record at `time` of an answer with `output` output
    /// tokens, after `before` output tokens in the session.
    fn answer(time: &str, before: u64, output: u64) -> String {
        let total = before + output;
        format!(
            r#"{{"timestamp":"{time}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{total},"cached_input_tokens":0,"output_tokens":{total}}},"last_token_usage":{{"input_tokens":{output},"cached_input_tokens":0,"output_tokens":{output}}}}}}}}}"#
        )
    }

    fn tokens(count: u64) -> TokenCount {
        TokenCount::new(count, 0, count)
    }

    #[test]
    fn a_session_with_several_answers_is_added_up() {
        let config = ConfigDir::new();
        config.write(
            "sessions/2026/11/16",
            "a1",
            &[
                answer("2026-11-16T09:10:00Z", 0, 1),
                answer("2026-11-16T09:20:00Z", 1, 2),
            ],
            at(NOW),
        );

        let spend = config.spend();

        assert_eq!(spend.today(), tokens(3));
        assert_eq!(spend.projects()[0].folder(), Path::new("/home/user/otty"));
        let models = spend.this_month_by_model();
        assert_eq!(models[0].model(), Some("gpt-5-codex"));
    }

    #[test]
    fn records_count_on_their_side_of_midnight_and_of_the_30_days() {
        let config = ConfigDir::new();
        config.write(
            "sessions/2026/10/17",
            "a1",
            &[
                answer("2026-10-17T23:59:59Z", 0, 1000),
                answer("2026-10-18T00:00:00Z", 1000, 1),
                answer("2026-11-15T23:59:59Z", 1001, 2),
                answer("2026-11-16T00:00:00Z", 1003, 3),
            ],
            at(NOW),
        );

        let spend = config.spend();

        assert_eq!(spend.today(), tokens(3));
        assert_eq!(spend.last_7_days(), tokens(5));
        assert_eq!(spend.last_30_days(), tokens(6));
    }

    #[test]
    fn a_helper_agents_file_counts_too() {
        let config = ConfigDir::new();
        let helper = format!(
            "{}\n{}\n",
            r#"{"type":"session_meta","payload":{"id":"h1","cwd":"/home/user/otty","source":{"subagent":{"thread_spawn":{"depth":1}}}}}"#,
            answer("2026-11-16T09:30:00Z", 0, 4)
        );
        config.write(
            "sessions/2026/11/16",
            "a1",
            &[answer("2026-11-16T09:10:00Z", 0, 1)],
            at(NOW),
        );
        let helper_path = config.file("sessions/2026/11/16", "h1", "jsonl");
        write_file(&helper_path, &helper);
        set_file_time(&helper_path, at(NOW));

        assert_eq!(config.spend().today(), tokens(5));
    }

    #[test]
    fn a_compressed_file_and_its_copies_count_once() {
        let config = ConfigDir::new();
        let lines = [answer("2026-11-16T09:10:00Z", 0, 1)];
        let compressed = config.file("sessions/2025/01/02", "a1", "jsonl.zst");
        write_compressed(&compressed, &with_meta("a1", &lines));
        set_file_time(&compressed, at(NOW));
        config.write("sessions/2026/11/16", "a1", &lines, at(NOW));
        config.write("archived_sessions", "a1", &lines, at(NOW));
        let other = config.file("sessions/2025/01/02", "b1", "jsonl.zst");
        write_compressed(
            &other,
            &with_meta("b1", &[answer("2026-11-16T09:40:00Z", 0, 2)]),
        );
        set_file_time(&other, at(NOW));

        assert_eq!(config.spend().today(), tokens(3));
    }

    #[test]
    fn broken_lines_are_skipped() {
        let config = ConfigDir::new();
        config.write(
            "sessions/2026/11/16",
            "a1",
            &[
                String::from("{\"type\":\"event_msg\",\"payload\":"),
                String::from("token_count but not json"),
                answer("2026-11-16T09:10:00Z", 0, 1),
            ],
            at(NOW),
        );
        let broken = config.file("sessions/2026/11/16", "x1", "jsonl.zst");
        write_file(&broken, "not compressed at all");
        set_file_time(&broken, at(NOW));

        assert_eq!(config.spend().today(), tokens(1));
    }

    #[test]
    fn a_file_that_did_not_change_is_not_counted_again() {
        let config = ConfigDir::new();
        let folder = "sessions/2026/11/16";
        config.write(
            folder,
            "a1",
            &[answer("2026-11-16T09:10:00Z", 0, 1)],
            at(NOW),
        );

        let first = config.spend();
        config.write(
            folder,
            "a1",
            &[answer("2026-11-16T09:10:00Z", 0, 9)],
            at(NOW),
        );
        let second = config.spend();
        config.write(
            folder,
            "a1",
            &[answer("2026-11-16T09:10:00Z", 0, 9)],
            at(NOW + 1),
        );
        let third = config.spend();

        assert_eq!(first.today(), tokens(1));
        assert_eq!(second.today(), tokens(1));
        assert_eq!(third.today(), tokens(9));
    }

    #[test]
    fn a_file_not_changed_in_30_days_is_not_counted() {
        let config = ConfigDir::new();
        config.write(
            "sessions/2026/11/16",
            "a1",
            &[answer("2026-11-16T09:10:00Z", 0, 1)],
            at(NOW - 40 * 24 * 3600),
        );

        assert!(config.spend().today().is_zero());
    }

    #[test]
    fn a_sessions_path_that_is_not_a_folder_is_not_available() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".codex-work");
        write_file(&config_dir.join("archived_sessions"), "not a folder");
        let counter = SessionSpend::new(Arc::default());

        let spend = counter
            .spend(&codex_profile(&config_dir), LocalNow::new(at(NOW), 0));

        assert!(matches!(spend, Err(SourceError::NotAvailable(_))));
    }

    #[test]
    fn a_missing_config_folder_spent_nothing() {
        let temp = TempDir::new();
        let counter = SessionSpend::new(Arc::default());
        let profile = codex_profile(&temp.path().join(".codex-gone"));

        let spend = counter.spend(&profile, LocalNow::new(at(NOW), 0));

        let spend = spend.expect("nothing to count is not a failure");
        assert!(spend.last_30_days().is_zero());
    }
}
