//! The read of the statistics of a session or of a project. It reads the
//! agents' session files, a project over its whole history, so it runs on a
//! thread of its own and never on the thread that draws the window.

use std::path::{Path, PathBuf};

use iced::Task;
use otty_agents::{
    Agent, Profile, ProjectStats, SessionId, SourceError, StatsSource,
};
use otty_app_agents::{
    AgentList, AgentsIntent, LoadedProjectStats, LoadedSessionStats,
};

use crate::load::on_thread;

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
///
/// let profile = Profile::default_for(
///     AgentId::new("codex"),
///     "Codex",
///     PathBuf::from("/home/user/.codex"),
/// );
/// let read = otty_app_agents_io::load_session_stats(
///     Arc::from(otty_agents::known_agents()),
///     profile,
///     SessionId::new("a1"),
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_session_stats(
    agents: AgentList,
    profile: Profile,
    session: SessionId,
) -> Task<AgentsIntent> {
    let fallback = (profile.clone(), session.clone());

    Task::perform(
        on_thread(move || read_session_stats(&agents, profile, session)),
        move |read| {
            let read = read.unwrap_or_else(|| {
                let (profile, session) = fallback;
                LoadedSessionStats {
                    profile,
                    session,
                    stats: Err(String::from("the read stopped")),
                }
            });
            AgentsIntent::SessionStatsLoaded(read)
        },
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
/// let read = otty_app_agents_io::load_project_stats(
///     Arc::from(otty_agents::known_agents()),
///     PathBuf::from("/work/otty"),
///     Vec::new(),
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_project_stats(
    agents: AgentList,
    folder: PathBuf,
    profiles: Vec<Profile>,
) -> Task<AgentsIntent> {
    let fallback = folder.clone();

    Task::perform(
        on_thread(move || read_project_stats(&agents, folder, &profiles)),
        move |read| {
            let read = read.unwrap_or_else(|| LoadedProjectStats {
                folder: fallback,
                parts: Err(String::from("the read stopped")),
            });
            AgentsIntent::ProjectStatsLoaded(read)
        },
    )
}

/// The statistics of `session` of `profile`, or why they could not be
/// read, ready to show.
fn read_session_stats(
    agents: &[Box<dyn Agent>],
    profile: Profile,
    session: SessionId,
) -> LoadedSessionStats {
    let stats = match stats_source(agents, &profile) {
        Some(source) => source
            .session_stats(&profile, &session)
            .map_err(|error| error.to_string()),
        None => Err(no_stats(&profile)),
    };

    LoadedSessionStats {
        profile,
        session,
        stats,
    }
}

/// What the files of each of `profiles` tell about `folder`, or why one
/// of them could not be read, ready to show: a part missing from the sum
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
) -> Result<ProjectStats, String> {
    let name = profile.name();

    let read = match stats_source(agents, profile) {
        Some(source) => source
            .project_stats(profile, folder)
            .map_err(|error| error.to_string()),
        None => Err(no_stats(profile)),
    };

    read.map_err(|reason| format!("{name}: {reason}"))
}

/// The statistics source of the agent `profile` belongs to; `None` for an
/// agent OTTY does not know or one that reads no statistics.
fn stats_source(
    agents: &[Box<dyn Agent>],
    profile: &Profile,
) -> Option<Box<dyn StatsSource>> {
    agents
        .iter()
        .find(|agent| agent.descriptor().id() == profile.agent())
        .and_then(|agent| agent.stats_source())
}

/// Why `profile` has no statistics: its agent reads none.
fn no_stats(profile: &Profile) -> String {
    let agent = profile.agent().as_str();

    SourceError::NotAvailable(format!("{agent} reads no statistics"))
        .to_string()
}

#[cfg(all(test, unix))]
mod tests {
    use std::sync::Arc;

    use otty_agents::{AgentId, TokenCount, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::load::tests::run;

    /// A made-up Claude Code profile in `home` with the session `c1` in
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

    /// A made-up Codex profile in `home` with the session `x1` in
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

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("the folder is made");
        std::fs::write(path, text).expect("the file is written");
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
            &known_agents(),
            profile.clone(),
            SessionId::new("c1"),
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
            &known_agents(),
            profile,
            SessionId::new("gone"),
        );
        let unknown = read_session_stats(
            &known_agents(),
            stranger(),
            SessionId::new("s1"),
        );

        assert!(
            gone.stats
                .as_ref()
                .is_err_and(|reason| reason.contains("gone")),
            "{:?}",
            gone.stats
        );
        assert!(
            unknown
                .stats
                .as_ref()
                .is_err_and(|reason| !reason.is_empty()),
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
            &known_agents(),
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
            &known_agents(),
            PathBuf::from("/work/otty"),
            &[claude, stranger()],
        );

        assert!(
            read.parts
                .as_ref()
                .is_err_and(|reason| reason.starts_with("Stranger: ")),
            "{:?}",
            read.parts
        );
    }

    #[test]
    fn the_reads_run_in_the_background_and_answer_with_an_intent() {
        let home = TempDir::new("agents-stats-task");
        let profile = claude_with_a_session(&home);
        let agents: AgentList = Arc::from(known_agents());

        let session = run(load_session_stats(
            Arc::clone(&agents),
            profile.clone(),
            SessionId::new("c1"),
        ));
        let project = run(load_project_stats(
            agents,
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
