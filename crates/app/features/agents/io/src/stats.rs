//! The read of the statistics of a session or of a project. It reads the
//! agents' session files, a project over its whole history, so it runs on a
//! thread of its own and never on the thread that draws the window.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use jiff::tz::TimeZone;
use otty_agents::{Agent, LocalNow, Profile, ProjectStats, SessionId};
use otty_app_agents::{
    AgentList, AgentsIntent, LoadedProjectStats, LoadedSessionStats,
    ProjectStatsFailure,
};

use crate::background::{on_own_thread, read_stopped};
use crate::time_zone::user_time_zone;

/// Read the statistics of the session `session` of `profile` in the
/// background, with the agent of `agents` the profile belongs to. The
/// answer comes back as [`AgentsIntent::SessionStatsLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, Profile, SessionId};
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
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_session_stats(
    agents: &AgentList,
    profile: Profile,
    session: SessionId,
) -> Task<AgentsIntent> {
    let task =
        format!("reading the statistics of a session of {}", profile.name());
    let agents = Arc::clone(agents);
    let fallback = (profile.clone(), session.clone());

    on_own_thread(
        task,
        move || {
            let time_zone = user_time_zone().unwrap_or(TimeZone::UTC);
            let now = LocalNow::new(SystemTime::now(), time_zone);
            read_session_stats(&agents, profile, session, &now)
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
/// to. The answer comes back as [`AgentsIntent::ProjectStatsLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_app_agents::AgentList;
///
/// let agents: AgentList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_project_stats(
///     &agents,
///     PathBuf::from("/work/otty"),
///     Vec::new(),
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_project_stats(
    agents: &AgentList,
    folder: PathBuf,
    profiles: Vec<Profile>,
) -> Task<AgentsIntent> {
    let task = format!("reading the statistics of {}", folder.display());
    let agents = Arc::clone(agents);
    let fallback = folder.clone();

    on_own_thread(
        task,
        move || read_project_stats(&agents, folder, &profiles),
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
/// `now`, or why they could not be read.
fn read_session_stats(
    agents: &[Box<dyn Agent>],
    profile: Profile,
    session: SessionId,
    now: &LocalNow,
) -> LoadedSessionStats {
    let stats =
        otty_agents::read_session_stats(agents, &profile, &session, now);

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
) -> LoadedProjectStats {
    let parts = profiles
        .iter()
        .map(|profile| {
            let part = part(agents, profile, &folder)?;
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
) -> Result<ProjectStats, ProjectStatsFailure> {
    otty_agents::read_project_stats(agents, profile, folder).map_err(|error| {
        ProjectStatsFailure {
            profile_name: Some(profile.name().to_string()),
            error,
        }
    })
}

#[cfg(all(test, unix))]
mod tests {

    use otty_agents::{AgentId, SourceError, TokenCount, Unavailable};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{agents_with_plugins, run, write};

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
        );

        assert_eq!(read.profile, profile);
        assert_eq!(read.session, SessionId::new("c1"));
        let stats = read.stats.expect("the session is read");
        assert_eq!(stats.messages(), 1);
        assert_eq!(stats.tokens(), TokenCount::new(10, 100, 5));
    }

    #[test]
    fn a_session_with_no_file_or_of_an_unknown_agent_fails_with_a_reason() {
        let home = TempDir::new("agents-stats-gone");
        let profile = claude_with_a_session(&home);

        let gone = read_session_stats(
            &agents_with_plugins(home.path()),
            profile,
            SessionId::new("gone"),
            &utc_now(),
        );
        let unknown = read_session_stats(
            &[],
            stranger(),
            SessionId::new("s1"),
            &utc_now(),
        );

        assert!(gone.stats.is_err(), "{:?}", gone.stats);
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
        ));
        let project = run(load_project_stats(
            &agents,
            PathBuf::from("/work/otty"),
            vec![profile],
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
