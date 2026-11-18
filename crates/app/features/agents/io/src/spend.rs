//! The count of the tokens one profile spent. It reads the agent's session
//! files, which can be many, so it runs on a thread of its own.

use std::time::SystemTime;

use iced::Task;
use jiff::tz::TimeZone;
use otty_agents::{Agent, LocalNow, Profile};
use otty_app_agents::{AgentList, AgentsIntent, LoadedSpend};

use crate::load::on_thread;
use crate::time_zone::user_time_zone;

/// Count the tokens `profile` spent in the background, with the counter of
/// the agent of `agents` it belongs to, by the days of the user's time
/// zone. The answer comes back as [`AgentsIntent::SpendLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, Profile};
///
/// let profile = Profile::default_for(
///     AgentId::new("codex"),
///     "Codex",
///     PathBuf::from("/home/user/.codex"),
/// );
/// let count = otty_app_agents_io::load_spend(
///     Arc::from(otty_agents::known_agents()),
///     profile,
/// );
///
/// assert_eq!(count.units(), 1);
/// ```
pub fn load_spend(agents: AgentList, profile: Profile) -> Task<AgentsIntent> {
    let fallback = profile.clone();

    Task::perform(
        on_thread(move || {
            let time_zone = user_time_zone().unwrap_or(TimeZone::UTC);
            let now = LocalNow::new(SystemTime::now(), time_zone);
            count_spend(&agents, profile, &now)
        }),
        move |counted| {
            let counted = counted.unwrap_or_else(|| {
                log::warn!(
                    "counting the tokens of {} stopped",
                    fallback.name()
                );
                LoadedSpend {
                    profile: fallback,
                    spend: None,
                    counted_at: SystemTime::now(),
                }
            });
            AgentsIntent::SpendLoaded(counted)
        },
    )
}

/// The tokens `profile` spent up to `now`, with the time the count ended.
/// An agent OTTY does not know, or one with no counter, counts nothing.
fn count_spend(
    agents: &[Box<dyn Agent>],
    profile: Profile,
    now: &LocalNow,
) -> LoadedSpend {
    let counter = agents
        .iter()
        .find(|agent| agent.descriptor().id() == profile.agent())
        .and_then(|agent| agent.spend_counter());

    let spend = counter.map(|counter| counter.spend(&profile, now));

    LoadedSpend {
        profile,
        spend,
        counted_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use otty_agents::{AgentId, TokenCount, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::load::tests::run;

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;

    fn now() -> LocalNow {
        LocalNow::new(
            SystemTime::UNIX_EPOCH + Duration::from_secs(NOW),
            TimeZone::UTC,
        )
    }

    /// A Claude Code profile in a made-up folder with one session file of
    /// one answer written today.
    fn claude_with_one_answer(home: &TempDir) -> Profile {
        let config_dir = home.path().join(".claude-test");
        let session_file = config_dir.join("projects/-home-user-otty/s1.jsonl");
        std::fs::create_dir_all(
            session_file.parent().expect("the project folder"),
        )
        .expect("the project folder");
        let answer = r#"{"type":"assistant","cwd":"/home/user/otty","sessionId":"s1","requestId":"req_1","timestamp":"2026-11-16T09:00:00Z","message":{"id":"msg_1","model":"claude-sonnet-4-5","usage":{"input_tokens":10,"cache_read_input_tokens":100,"output_tokens":5}}}"#;
        std::fs::write(&session_file, answer).expect("the session file");
        std::fs::File::options()
            .write(true)
            .open(&session_file)
            .and_then(|file| file.set_modified(now().now()))
            .expect("the file time");

        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Test",
            config_dir,
        )
    }

    #[test]
    fn the_tokens_are_counted_by_the_agent_of_the_profile() {
        let home = TempDir::new("agents-spend");
        let profile = claude_with_one_answer(&home);
        let before = SystemTime::now();

        let counted = count_spend(&known_agents(), profile.clone(), &now());

        assert_eq!(counted.profile, profile);
        let spend = counted.spend.expect("Claude Code counts tokens");
        let spend = spend.expect("the files are counted");
        assert_eq!(spend.today(), TokenCount::new(10, 100, 5));
        assert!(counted.counted_at >= before);
    }

    #[test]
    fn a_profile_of_an_unknown_agent_counts_nothing() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let counted = count_spend(&known_agents(), profile, &now());

        assert_eq!(counted.spend, None);
    }

    #[test]
    fn the_count_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-spend-task");
        let profile = claude_with_one_answer(&home);
        let agents: AgentList = Arc::from(known_agents());

        let answers = run(load_spend(agents, profile.clone()));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::SpendLoaded(counted)]
                if counted.profile == profile
                    && matches!(counted.spend, Some(Ok(_)))
        ));
    }
}
