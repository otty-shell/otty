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
    use crate::testing::{agents_with_plugins, no_prices, run, write};

    fn utc_now() -> LocalNow {
        LocalNow::new(SystemTime::UNIX_EPOCH, TimeZone::UTC)
    }

    /// A made-up Coder profile in `home` with the session `c1` in
    /// `/work/otty`: one prompt and one answer.
    fn claude_with_a_session(home: &TempDir) -> Profile {
        let config_dir = home.path().join(".claude-test");
        let lines = [
            r#"{"type":"user","cwd":"/work/otty","sessionId":"c1","timestamp":"2026-11-16T09:00:00Z","message":{"role":"user","content":"Fix the tabs"}}"#,
            r#"{"type":"assistant","cwd":"/work/otty","sessionId":"c1","requestId":"req_1","timestamp":"2026-11-16T09:01:00Z","message":{"id":"msg_1","model":"claude-sonnet-4-5","usage":{"input_tokens":10,"cache_read_input_tokens":100,"output_tokens":5}}}"#,
        ];
        write(
            &config_dir.join("projects/-work-otty/c1.jsonl"),
            &lines.join("\n"),
        );

        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Test",
            config_dir,
        )
    }

    /// A made-up Coder profile in `home` with the session `x1` in
    /// `/work/otty`: one answer of `gpt-5-codex`.
    fn codex_with_a_session(home: &TempDir) -> Profile {
        let config_dir = home.path().join(".codex-test");
        let lines = [
            r#"{"timestamp":"2026-11-16T09:00:00.000Z","type":"session_meta","payload":{"id":"x1","cwd":"/work/otty","source":"cli"}}"#,
            r#"{"timestamp":"2026-11-16T09:00:01.000Z","type":"turn_context","payload":{"cwd":"/work/otty","model":"gpt-5-codex"}}"#,
            r#"{"timestamp":"2026-11-16T09:02:00.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":30,"cached_input_tokens":0,"output_tokens":3},"last_token_usage":{"input_tokens":30,"cached_input_tokens":0,"output_tokens":3}},"rate_limits":null}}"#,
        ];
        write(
            &config_dir.join(
                "sessions/2026/11/16/rollout-2026-11-16T09-00-00-x1.jsonl",
            ),
            &lines.join("\n"),
        );

        Profile::added(AgentId::new("codex"), "Codex · Test", config_dir)
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
        let profile = claude_with_a_session(&home);

        let read = read_session_stats(
            &agents_with_plugins(home.path()),
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
        let profile = claude_with_a_session(&home);

        let gone = read_session_stats(
            &agents_with_plugins(home.path()),
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
    fn a_project_is_read_from_each_profile_of_each_agent() {
        let home = TempDir::new("agents-stats-project");
        let claude = claude_with_a_session(&home);
        let codex = codex_with_a_session(&home);

        let read = read_project_stats(
            &agents_with_plugins(home.path()),
            PathBuf::from("/work/otty"),
            &[claude.clone(), codex.clone()],
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
                (&claude, TokenCount::new(10, 100, 5)),
                (&codex, TokenCount::new(30, 0, 3)),
            ]
        );
    }

    #[test]
    fn a_project_fails_with_the_reason_of_the_profile_that_failed() {
        let home = TempDir::new("agents-stats-project-failed");
        let claude = claude_with_a_session(&home);

        let read = read_project_stats(
            &agents_with_plugins(home.path()),
            PathBuf::from("/work/otty"),
            &[claude, stranger()],
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
        let profile = claude_with_a_session(&home);
        let agents = agents_with_plugins(home.path());

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
    /// Issue 22: how long the first session panel, the first project
    /// panel and a second panel take from the real `~/.codex`, and the
    /// memory of the process after them. It only reads the folder. Run it
    /// by hand: `cargo test -p otty-app-agents-io --release
    /// measure_the_panels -- --ignored --nocapture`, one test at a time.
    #[test]
    #[ignore = "reads the real ~/.codex; run by hand"]
    fn measure_the_panels_of_the_real_codex() {
        measure_panels(AgentId::new("codex"), "Codex", ".codex");
    }

    /// [`measure_the_panels_of_the_real_codex`] for the real `~/.claude`.
    #[test]
    #[ignore = "reads the real ~/.claude; run by hand"]
    fn measure_the_panels_of_the_real_claude() {
        measure_panels(AgentId::new("claude-code"), "Claude Code", ".claude");
    }

    /// Issue 22: how long a count of the real OpenCode database takes now
    /// that its whole history is read; the count after a change of the
    /// database reads it all again, as the first one does.
    #[test]
    #[ignore = "reads the real OpenCode data; run by hand"]
    fn measure_the_answers_of_the_real_opencode() {
        let home = std::env::var_os("HOME").map(PathBuf::from).expect("HOME");
        let profile = Profile::default_for(
            AgentId::new("opencode"),
            "OpenCode",
            home.join(".local/share/opencode"),
        );
        let agents = crate::testing::agents_with_opencode(&home);
        let agent =
            otty_agents::agent_of(&agents, profile.agent()).expect("the agent");
        let counter = agent.spend_counter().expect("a spend counter");

        for read in ["first", "unchanged"] {
            let started = std::time::Instant::now();
            let spend = counter.spend(&profile, &utc_now(), &no_prices());
            let took = started.elapsed();

            let today = spend.map(|spend| spend.last_30_days().total());
            println!("OpenCode: {read} count: {today:?} tokens in {took:?}");
        }
        println!("OpenCode: after: {}", memory());
    }

    /// Prints how long the panels of the newest sessions of the profile
    /// in `~/<folder>` take, each first one with agents that read nothing
    /// yet, and the memory of the process after them.
    fn measure_panels(agent: AgentId, name: &str, folder: &str) {
        let home = std::env::var_os("HOME").map(PathBuf::from).expect("HOME");
        let profile = Profile::default_for(agent, name, home.join(folder));
        println!("{name}: before: {}", memory());

        // The sessions are read at the start of Agents mode, before any
        // panel can open.
        let agents = agents_with_plugins(&home);
        let started = std::time::Instant::now();
        let sessions = newest_sessions(&agents, &profile);
        println!(
            "{name}: sessions read: {} sessions in {:?}",
            sessions.len(),
            started.elapsed()
        );
        let (first, second) = (sessions[0].clone(), sessions[1].clone());

        let started = std::time::Instant::now();
        let session = otty_agents::read_session_stats(
            &agents,
            &profile,
            first.id(),
            &utc_now(),
            &no_prices(),
        )
        .expect("the session");
        println!(
            "{name}: (a) first session panel: {} answers in {:?}",
            session.answers(),
            started.elapsed()
        );
        drop(agents);

        let agents = agents_with_plugins(&home);
        let _ = newest_sessions(&agents, &profile);
        let started = std::time::Instant::now();
        let project = otty_agents::read_project_stats(
            &agents,
            &profile,
            first.folder(),
            &no_prices(),
        )
        .expect("the project");
        println!(
            "{name}: (b) first project panel: {} tokens in {:?}",
            project.tokens().total(),
            started.elapsed()
        );

        let started = std::time::Instant::now();
        let session = otty_agents::read_session_stats(
            &agents,
            &profile,
            second.id(),
            &utc_now(),
            &no_prices(),
        )
        .expect("the second session");
        println!(
            "{name}: (c) second panel: {} answers in {:?}",
            session.answers(),
            started.elapsed()
        );
        println!("{name}: after: {}", memory());
    }

    /// The sessions of `profile`, the newest first.
    fn newest_sessions(
        agents: &[Box<dyn Agent>],
        profile: &Profile,
    ) -> Vec<otty_agents::AgentSession> {
        let agent =
            otty_agents::agent_of(agents, profile.agent()).expect("the agent");
        let mut sessions =
            otty_agents::read_sessions(&agent.session_sources(), profile)
                .expect("the sessions")
                .into_value();
        sessions.sort_by_key(|session| std::cmp::Reverse(session.changed_at()));
        sessions
    }

    /// The peak and the resident memory of this process.
    fn memory() -> String {
        let status =
            std::fs::read_to_string("/proc/self/status").unwrap_or_default();

        status
            .lines()
            .filter(|line| {
                line.starts_with("VmHWM") || line.starts_with("VmRSS")
            })
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
