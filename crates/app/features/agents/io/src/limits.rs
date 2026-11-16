//! The read of the limits of one profile. Its sources may ask a server over
//! the network, so it runs on a thread of its own.

use std::time::SystemTime;

use iced::Task;
use otty_agents::{Agent, Profile};
use otty_app_agents::{AgentList, AgentsIntent, LoadedLimits};

use crate::load::on_thread;

/// Read the limits of `profile` in the background, from the sources of the
/// agent of `agents` it belongs to, best first. The answer comes back as
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
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_limits(agents: AgentList, profile: Profile) -> Task<AgentsIntent> {
    let fallback = profile.clone();

    Task::perform(
        on_thread(move || read_limits(&agents, profile)),
        move |loaded| {
            let loaded = loaded.unwrap_or_else(|| {
                log::warn!("reading the limits of {} stopped", fallback.name());
                LoadedLimits {
                    profile: fallback,
                    limits: Err(Vec::new()),
                    read_at: SystemTime::now(),
                }
            });
            AgentsIntent::LimitsLoaded(loaded)
        },
    )
}

/// The limits of `profile` from the first source of its agent that
/// answers, with the time the read ended. An agent OTTY does not know has
/// no sources.
fn read_limits(agents: &[Box<dyn Agent>], profile: Profile) -> LoadedLimits {
    let agent = agents
        .iter()
        .find(|agent| agent.descriptor().id() == profile.agent());

    let limits = match agent {
        Some(agent) => {
            otty_agents::read_limits(&agent.limit_sources(), &profile)
        },
        None => Err(Vec::new()),
    };

    LoadedLimits {
        profile,
        limits,
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

        let loaded = read_limits(&known_agents(), profile.clone());

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
        assert!(loaded.read_at >= before);
    }

    #[test]
    fn a_profile_of_an_unknown_agent_has_no_way_to_read_limits() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let loaded = read_limits(&known_agents(), profile);

        assert_eq!(loaded.limits, Err(Vec::new()));
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-limits-task");
        let profile = logged_out_claude(&home);
        let agents: AgentList = Arc::from(known_agents());

        let answers = run(load_limits(agents, profile.clone()));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::LimitsLoaded(loaded)]
                if loaded.profile == profile && loaded.limits.is_err()
        ));
    }
}
