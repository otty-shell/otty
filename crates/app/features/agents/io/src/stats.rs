//! The read of the statistics of a session or of a project, which OTTY
//! sums from the answers of the agents' session files over the whole
//! history, so it runs on a thread of its own and never on the thread that
//! draws the window.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use jiff::tz::TimeZone;
use otty_agents::{
    Agent, ApiPrices, LocalNow, Profile, ProjectStats, SessionId,
};
use otty_app_agents::{
    AgentList, AgentsIntent, LoadedProjectStats, LoadedSessionStats,
    ProjectStatsFailure,
};

use crate::background::{on_own_thread, read_stopped};
use crate::time_zone::user_time_zone;

/// Read the statistics of the session `session` of `profile` in the
/// background, with the agent of `agents` the profile belongs to, whose
/// part prices the answers at `prices`. The answer comes back as
/// [`AgentsIntent::SessionStatsLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, ApiPrices, Profile, SessionId};
/// use otty_app_agents::AgentList;
///
/// let profile = Profile::default_for(
///     AgentId::new("coder"),
///     "Coder",
///     PathBuf::from("/home/user/.coder"),
/// );
/// let agents: AgentList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_session_stats(
///     &agents,
///     profile,
///     SessionId::new("a1"),
///     &Arc::new(ApiPrices::default()),
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_session_stats(
    agents: &AgentList,
    profile: Profile,
    session: SessionId,
    prices: &Arc<ApiPrices>,
) -> Task<AgentsIntent> {
    let task =
        format!("reading the statistics of a session of {}", profile.name());
    let agents = Arc::clone(agents);
    let prices = Arc::clone(prices);
    let fallback = (profile.clone(), session.clone());

    on_own_thread(
        task,
        move || {
            let time_zone = user_time_zone().unwrap_or(TimeZone::UTC);
            let now = LocalNow::new(SystemTime::now(), time_zone);
            read_session_stats(&agents, profile, session, &now, &prices)
        },
        move || {
            let (profile, session) = fallback;
            LoadedSessionStats {
                profile,
                session,
                stats: Err(read_stopped()),
            }
        },
        AgentsIntent::SessionStatsLoaded,
    )
}

/// Read the statistics of the project `folder` in the background, from
/// the files of each of `profiles` with the agent of `agents` it belongs
/// to, whose part prices the answers at `prices`. The answer comes back as
/// [`AgentsIntent::ProjectStatsLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::ApiPrices;
/// use otty_app_agents::AgentList;
///
/// let agents: AgentList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_project_stats(
///     &agents,
///     PathBuf::from("/work/otty"),
///     Vec::new(),
///     &Arc::new(ApiPrices::default()),
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_project_stats(
    agents: &AgentList,
    folder: PathBuf,
    profiles: Vec<Profile>,
    prices: &Arc<ApiPrices>,
) -> Task<AgentsIntent> {
    let task = format!("reading the statistics of {}", folder.display());
    let agents = Arc::clone(agents);
    let prices = Arc::clone(prices);
    let fallback = folder.clone();

    on_own_thread(
        task,
        move || read_project_stats(&agents, folder, &profiles, &prices),
        move || LoadedProjectStats {
            folder: fallback,
            parts: Err(ProjectStatsFailure {
                profile_name: None,
                error: read_stopped(),
            }),
        },
        AgentsIntent::ProjectStatsLoaded,
    )
}

/// The statistics of `session` of `profile`, its days counted up to
/// `now` and its answers priced at `prices`, or why they could not be
/// read.
fn read_session_stats(
    agents: &[Box<dyn Agent>],
    profile: Profile,
    session: SessionId,
    now: &LocalNow,
    prices: &Arc<ApiPrices>,
) -> LoadedSessionStats {
    let stats = otty_agents::read_session_stats(
        agents, &profile, &session, now, prices,
    );

    LoadedSessionStats {
        profile,
        session,
        stats,
    }
}

/// What the files of each of `profiles` tell about `folder`, or why one
/// of them could not be read: a part missing from the sum
/// would show a project smaller than it is.
fn read_project_stats(
    agents: &[Box<dyn Agent>],
    folder: PathBuf,
    profiles: &[Profile],
    prices: &Arc<ApiPrices>,
) -> LoadedProjectStats {
    let parts = profiles
        .iter()
        .map(|profile| {
            let part = part(agents, profile, &folder, prices)?;
            Ok((profile.clone(), part))
        })
        .collect();

    LoadedProjectStats { folder, parts }
}

/// What the files of `profile` tell about `folder`, or why they could not
/// be read, naming the profile.
fn part(
    agents: &[Box<dyn Agent>],
    profile: &Profile,
    folder: &Path,
    prices: &Arc<ApiPrices>,
) -> Result<ProjectStats, ProjectStatsFailure> {
    otty_agents::read_project_stats(agents, profile, folder, prices).map_err(
        |error| ProjectStatsFailure {
            profile_name: Some(profile.name().to_string()),
            error,
        },
    )
}

#[cfg(all(test, unix))]
mod tests {

    use otty_agents::{AgentId, SourceError, TokenCount, Unavailable};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{
        EXAMPLE_AGENT, agents_with_example, no_prices, run, write,
    };

    fn utc_now() -> LocalNow {
        LocalNow::new(SystemTime::UNIX_EPOCH, TimeZone::UTC)
    }

    /// A made-up profile of the example agent in `home/<folder>`, named
    /// `name`, with the session `session` in `/work/otty`: one answer with
    /// `tokens` (input, cached, output).
    fn example_with_a_session(
        home: &TempDir,
        folder: &str,
        name: &str,
        session: &str,
        tokens: (u64, u64, u64),
    ) -> Profile {
        let config_dir = home.path().join(folder);
        let (input, cached, output) = tokens;
        let lines = [
            r#"{"type":"meta","version":"1.4.2","cwd":"/work/otty","title":"Fix the tabs"}"#.to_string(),
            format!(
                r#"{{"type":"answer","id":"{session}-a1","at_ms":1794819660000,"model":"example-large","tokens":{{"input":{input},"cached":{cached},"output":{output}}}}}"#
            ),
        ];
        write(
            &config_dir.join(format!("sessions/{session}.jsonl")),
            &lines.join("\n"),
        );

        Profile::added(AgentId::new(EXAMPLE_AGENT), name, config_dir)
    }

    /// The profile `Example (Test)` with the session `c1`.
    fn example_test(home: &TempDir) -> Profile {
        example_with_a_session(
            home,
            ".example-agent-test",
            "Example (Test)",
            "c1",
            (10, 100, 5),
        )
    }

    /// The profile `Example (Work)` with the session `x1`.
    fn example_work(home: &TempDir) -> Profile {
        example_with_a_session(
            home,
            ".example-agent-work",
            "Example (Work)",
            "x1",
            (30, 0, 3),
        )
    }

    fn stranger() -> Profile {
        Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        )
    }

    #[test]
    fn a_session_is_read_by_the_agent_of_its_profile() {
        let home = TempDir::new("agents-stats-session");
        let profile = example_test(&home);

        let read = read_session_stats(
            &agents_with_example(home.path()),
            profile.clone(),
            SessionId::new("c1"),
            &utc_now(),
            &no_prices(),
        );

        assert_eq!(read.profile, profile);
        assert_eq!(read.session, SessionId::new("c1"));
        let stats = read.stats.expect("the session is read");
        assert_eq!(stats.answers(), 1);
        assert_eq!(stats.tokens(), TokenCount::new(10, 100, 5));
    }

    /// Issue 22: OTTY sums a session from its answers, so a session no
    /// answer names has none; an agent OTTY does not know reads nothing.
    #[test]
    fn a_session_with_no_answers_is_empty_and_an_unknown_agent_fails() {
        let home = TempDir::new("agents-stats-gone");
        let profile = example_test(&home);

        let gone = read_session_stats(
            &agents_with_example(home.path()),
            profile,
            SessionId::new("gone"),
            &utc_now(),
            &no_prices(),
        );
        let unknown = read_session_stats(
            &[],
            stranger(),
            SessionId::new("s1"),
            &utc_now(),
            &no_prices(),
        );

        let gone = gone.stats.expect("a session with no answers");
        assert_eq!(gone.answers(), 0);
        assert_eq!(gone.started_at(), None);
        assert!(
            unknown.stats.as_ref().is_err_and(|error| matches!(
                error,
                SourceError::NotAvailable(Unavailable::NoStatistics(_))
            )),
            "{:?}",
            unknown.stats
        );
    }

    #[test]
    fn a_project_is_read_from_each_profile() {
        let home = TempDir::new("agents-stats-project");
        let test = example_test(&home);
        let work = example_work(&home);

        let read = read_project_stats(
            &agents_with_example(home.path()),
            PathBuf::from("/work/otty"),
            &[test.clone(), work.clone()],
            &no_prices(),
        );

        assert_eq!(read.folder, PathBuf::from("/work/otty"));
        let parts = read.parts.expect("both profiles are read");
        let tokens: Vec<(&Profile, TokenCount)> = parts
            .iter()
            .map(|(profile, part)| (profile, part.tokens()))
            .collect();
        assert_eq!(
            tokens,
            [
                (&test, TokenCount::new(10, 100, 5)),
                (&work, TokenCount::new(30, 0, 3)),
            ]
        );
    }

    #[test]
    fn a_project_fails_with_the_reason_of_the_profile_that_failed() {
        let home = TempDir::new("agents-stats-project-failed");
        let test = example_test(&home);

        let read = read_project_stats(
            &agents_with_example(home.path()),
            PathBuf::from("/work/otty"),
            &[test, stranger()],
            &no_prices(),
        );

        assert!(
            read.parts.as_ref().is_err_and(|failure| {
                failure.profile_name.as_deref() == Some(stranger().name())
            }),
            "{:?}",
            read.parts
        );
    }

    #[test]
    fn the_reads_run_in_the_background_and_answer_with_an_intent() {
        let home = TempDir::new("agents-stats-task");
        let profile = example_test(&home);
        let agents = agents_with_example(home.path());

        let session = run(load_session_stats(
            &agents,
            profile.clone(),
            SessionId::new("c1"),
            &no_prices(),
        ));
        let project = run(load_project_stats(
            &agents,
            PathBuf::from("/work/otty"),
            vec![profile],
            &no_prices(),
        ));

        assert!(matches!(
            session.as_slice(),
            [AgentsIntent::SessionStatsLoaded(read)] if read.stats.is_ok()
        ));
        assert!(matches!(
            project.as_slice(),
            [AgentsIntent::ProjectStatsLoaded(read)] if read.parts.is_ok()
        ));
    }
}
