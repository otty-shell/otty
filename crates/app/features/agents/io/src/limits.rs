//! The read of the limits of one profile. Its sources may ask a server over
//! the network or start the agent's program, so it runs on a thread of its
//! own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{Agent, Profile, StartProgram};
use otty_app_agents::{AgentList, AgentsIntent, LoadedLimits};

use crate::agent_program::LoginShellPrograms;
use crate::load::on_thread;

/// Read the limits of `profile` in the background, from the sources of the
/// agent of `agents` it belongs to, best first. A source that asks the
/// agent's program starts it through the user's shell at `shell`, the way
/// the agent tab does. The answer comes back as
/// [`AgentsIntent::LimitsLoaded`].
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
/// let read = otty_app_agents_io::load_limits(
///     Arc::from(otty_agents::known_agents()),
///     profile,
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_limits(
    agents: AgentList,
    profile: Profile,
    shell: &str,
) -> Task<AgentsIntent> {
    let fallback = profile.clone();
    let shell = shell.to_string();

    Task::perform(
        on_thread(move || read_limits(&agents, profile, &shell)),
        move |loaded| {
            let loaded = loaded.unwrap_or_else(|| {
                log::warn!("reading the limits of {} stopped", fallback.name());
                LoadedLimits {
                    profile: fallback,
                    limits: Err(Vec::new()),
                    sources: Vec::new(),
                    read_at: SystemTime::now(),
                }
            });
            AgentsIntent::LimitsLoaded(loaded)
        },
    )
}

/// The limits of `profile` from the first source of its agent that
/// answers, with the names of all its sources and the time the read
/// ended. An agent OTTY does not know has no sources.
fn read_limits(
    agents: &[Box<dyn Agent>],
    profile: Profile,
    shell: &str,
) -> LoadedLimits {
    let agent = agents
        .iter()
        .find(|agent| agent.descriptor().id() == profile.agent());
    let programs: Arc<dyn StartProgram> =
        Arc::new(LoginShellPrograms::new(shell));

    let sources = match agent {
        Some(agent) => agent.limit_sources(programs),
        None => Vec::new(),
    };

    let limits = otty_agents::read_limits(&sources, &profile);

    LoadedLimits {
        profile,
        limits,
        sources: sources.iter().map(|source| source.name()).collect(),
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use otty_agents::{AgentId, SourceError, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::load::tests::run;

    /// A shell that does not exist: no test starts a real agent.
    const NO_SHELL: &str = "/no/such/shell";

    /// A Claude Code profile in a made-up folder with no token file: its
    /// source fails before it sends anything.
    fn logged_out_claude(home: &TempDir) -> Profile {
        let config_dir = home.path().join(".claude-test");
        std::fs::create_dir_all(&config_dir).expect("the config folder");

        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Test",
            config_dir,
        )
    }

    #[test]
    fn a_profile_with_no_login_has_the_failure_of_each_source_and_its_time() {
        let home = TempDir::new("agents-limits");
        let profile = logged_out_claude(&home);
        let before = SystemTime::now();

        let loaded = read_limits(&known_agents(), profile.clone(), NO_SHELL);

        assert_eq!(loaded.profile, profile);
        let failures = loaded.limits.expect_err("no source answers");
        let reasons: Vec<_> = failures
            .iter()
            .map(|failure| (failure.source(), failure.error().clone()))
            .collect();
        let no_token = SourceError::NotAvailable(String::from(
            "no login token in .credentials.json",
        ));
        assert_eq!(reasons, [("usage request", no_token)]);
        assert_eq!(loaded.sources, ["usage request"]);
        assert!(loaded.read_at >= before);
    }

    #[test]
    fn a_profile_of_an_unknown_agent_has_no_way_to_read_limits() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let loaded = read_limits(&known_agents(), profile, NO_SHELL);

        assert_eq!(loaded.limits, Err(Vec::new()));
        assert!(loaded.sources.is_empty());
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-limits-task");
        let profile = logged_out_claude(&home);
        let agents: AgentList = Arc::from(known_agents());

        let answers = run(load_limits(agents, profile.clone(), NO_SHELL));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::LimitsLoaded(loaded)]
                if loaded.profile == profile && loaded.limits.is_err()
        ));
    }
}
