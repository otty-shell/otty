//! The read of the limits of one profile. Its parts may ask a server over
//! the network or start the agent's program, so it runs on a thread of its
//! own.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{
    Agent, AgentCommand, PluginProvider, Profile, ProgramError, RunningProgram,
    RunningTerminal, SourceFailure, SourceId, StartProgram, StartTerminal,
};
use otty_app_agents::{AgentList, AgentsIntent, LimitsRead, LoadedLimits};

use crate::agent_program::LoginShellPrograms;
use crate::background::{on_own_thread, read_stopped};
use crate::hidden_terminal::LoginShellTerminals;
use crate::plugin_parts::ProviderList;

/// The folder in OTTY's cache folder that an agent's program started in a
/// hidden terminal runs in, so it never shows up as a session of a real
/// project.
pub(crate) const PROBE_FOLDER: &str = "agent-probe";

/// Whether a read of the limits may start the agent's program: `codex
/// app-server`, or `claude` in a hidden terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProgramStarts {
    /// Agents mode is on screen: the program may start.
    Allowed,
    /// Agents mode is away: the program does not start.
    NotNow,
}

/// The programs of a read while Agents mode is away: none starts.
struct NoPrograms;

impl StartProgram for NoPrograms {
    fn start(
        &self,
        _command: &AgentCommand,
    ) -> Result<Box<dyn RunningProgram>, ProgramError> {
        Err(ProgramError::NotNow)
    }

    /// No: the plugin parts start nothing either.
    fn starts_now(&self) -> bool {
        false
    }
}

/// The hidden terminals of a read while Agents mode is away: none starts.
struct NoTerminals {
    probe_dir: PathBuf,
}

impl StartTerminal for NoTerminals {
    fn working_dir(&self) -> PathBuf {
        self.probe_dir.clone()
    }

    fn start(
        &self,
        _command: &AgentCommand,
        _columns: u16,
        _rows: u16,
    ) -> Result<Box<dyn RunningTerminal>, ProgramError> {
        Err(ProgramError::NotNow)
    }
}

/// Read the limits of `profile` in the background: the provider part of
/// `providers` that serves the profile's subscription login first, then
/// the agent part of the agent of `agents` it belongs to, which reads them
/// from wherever it reads them. A read that asks the agent's program
/// starts it through the user's shell at `shell`, the way the agent tab
/// does; one that reads the program's screen starts it in a hidden
/// terminal in a folder of its own inside OTTY's cache folder
/// `cache_dir`. The answer comes back as [`AgentsIntent::LimitsLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, Profile};
/// use otty_app_agents::AgentList;
///
/// let profile = Profile::default_for(
///     AgentId::new("coder"),
///     "Coder",
///     PathBuf::from("/home/user/.coder"),
/// );
/// let agents: AgentList = Arc::from(Vec::new());
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_limits(
///     &agents,
///     &providers,
///     profile,
///     "/bin/zsh",
///     &PathBuf::from("/home/user/.cache/otty"),
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_limits(
    agents: &AgentList,
    providers: &ProviderList,
    profile: Profile,
    shell: &str,
    cache_dir: &Path,
) -> Task<AgentsIntent> {
    let readers = (agents, providers);

    read_task(readers, profile, shell, cache_dir, ProgramStarts::Allowed)
}

/// Read the limits of `profile` in the background while Agents mode is not
/// on screen, as [`load_limits`] does, except that the agent part starts
/// no program, neither `codex app-server` nor `claude` in a hidden
/// terminal, and a read on screen starts it (spec Decision 10).
pub fn load_limits_in_background(
    agents: &AgentList,
    providers: &ProviderList,
    profile: Profile,
    shell: &str,
    cache_dir: &Path,
) -> Task<AgentsIntent> {
    let readers = (agents, providers);

    read_task(readers, profile, shell, cache_dir, ProgramStarts::NotNow)
}

/// The read of the limits of `profile` with the agents and the provider
/// parts of `readers` on a thread of its own, which may start the agent's
/// program as `starts` says.
fn read_task(
    (agents, providers): (&AgentList, &ProviderList),
    profile: Profile,
    shell: &str,
    cache_dir: &Path,
    starts: ProgramStarts,
) -> Task<AgentsIntent> {
    let is_known = otty_agents::agent_of(agents, profile.agent()).is_some();
    let task = format!("reading the limits of {}", profile.name());
    let agents = Arc::clone(agents);
    let providers = Arc::clone(providers);
    let shell = shell.to_string();
    let probe_dir = cache_dir.join(PROBE_FOLDER);
    let fallback = profile.clone();

    on_own_thread(
        task,
        move || {
            let readers = (agents.as_ref(), providers.as_ref());
            read_limits(readers, profile, &shell, probe_dir, starts)
        },
        move || stopped_read(fallback, is_known),
        AgentsIntent::LimitsLoaded,
    )
}

/// The limits of `profile` from the first part that answers, the provider
/// part of the profile's subscription login first, then the agent part,
/// with the time the read ended. The agent's program starts only as
/// `starts` says; in a hidden terminal it runs in `probe_dir`. Nothing
/// reads the limits of an agent OTTY does not know.
fn read_limits(
    (agents, providers): (&[Box<dyn Agent>], &[PluginProvider]),
    profile: Profile,
    shell: &str,
    probe_dir: PathBuf,
    starts: ProgramStarts,
) -> LoadedLimits {
    let agent = otty_agents::agent_of(agents, profile.agent());
    let (programs, terminals): (Arc<dyn StartProgram>, Arc<dyn StartTerminal>) =
        match starts {
            ProgramStarts::Allowed => (
                Arc::new(LoginShellPrograms::new(shell)),
                Arc::new(LoginShellTerminals::new(shell, &probe_dir)),
            ),
            ProgramStarts::NotNow => {
                (Arc::new(NoPrograms), Arc::new(NoTerminals { probe_dir }))
            },
        };

    let Some(agent) = agent else {
        return LoadedLimits {
            profile,
            limits: LimitsRead::from_walk(Err(Vec::new())),
            read_at: SystemTime::now(),
        };
    };

    let agent_limits = agent.limit_source(programs, terminals);
    let walked = otty_agents::read_profile_limits(
        agent,
        providers,
        agent_limits.as_deref(),
        &profile,
    );

    LoadedLimits {
        profile,
        limits: LimitsRead::from_walk(walked),
        read_at: SystemTime::now(),
    }
}

/// What a read of the limits of `profile` that stopped before it answered
/// gives: the agent part failed because the read stopped, so the last good
/// answer stays, greyed, and the user sees why. Nothing reads the limits
/// of an agent OTTY does not know, `is_known` false.
fn stopped_read(profile: Profile, is_known: bool) -> LoadedLimits {
    let failures = if is_known {
        vec![SourceFailure::new(SourceId::AgentPart, read_stopped())]
    } else {
        Vec::new()
    };

    LoadedLimits {
        profile,
        limits: LimitsRead::from_walk(Err(failures)),
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;

    use otty_agents::AgentId;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::background::read_stopped;
    use crate::testing::{
        agents_with_plugins, agents_with_plugins_and_shell, executable_script,
        run, write,
    };

    /// A shell that does not exist: no test starts a real agent.
    const NO_SHELL: &str = "/no/such/shell";

    /// No provider part: no test sends a request to a provider.
    const NO_PROVIDERS: &[PluginProvider] = &[];

    /// No provider part, shared by the background reads.
    fn no_providers() -> ProviderList {
        Arc::from(Vec::new())
    }

    /// A Coder profile in a made-up folder with no login: the
    /// Anthropic plugin reads no limits for it and starts nothing.
    fn logged_out_claude(home: &TempDir) -> Profile {
        let config_dir = home.path().join(".claude-test");
        write(&config_dir.join("settings.json"), "{}");

        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Test",
            config_dir,
        )
    }

    /// A Coder profile in a made-up folder with a made-up login
    /// token, whose limits the plugin reads from the `/usage` panel.
    fn logged_in_claude(home: &TempDir) -> Profile {
        let config_dir = home.path().join(".claude-test");
        write(
            &config_dir.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"made-up-token"}}"#,
        );

        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Test",
            config_dir,
        )
    }

    #[test]
    fn a_profile_with_no_login_has_the_failure_of_its_agent_part_and_time() {
        let home = TempDir::new("agents-limits");
        let profile = logged_out_claude(&home);
        let probe_dir = home.path().join("agent-probe");
        let before = SystemTime::now();

        let loaded = read_limits(
            (&agents_with_plugins(home.path()), NO_PROVIDERS),
            profile.clone(),
            NO_SHELL,
            probe_dir,
            ProgramStarts::Allowed,
        );

        assert_eq!(loaded.profile, profile);
        assert_eq!(
            loaded.limits,
            LimitsRead::Failed(vec![SourceFailure::new(
                SourceId::AgentPart,
                otty_agents::SourceError::NotLoggedIn
            )])
        );
        assert!(loaded.read_at >= before);
    }

    /// While Agents mode is away the plugin's hidden terminal is not
    /// started: the plugin engine starts nothing for the read, and the
    /// next read on screen starts it. The shell here only counts its
    /// starts, so no real `claude` ever runs.
    #[test]
    fn a_read_in_the_background_starts_no_hidden_terminal() {
        let home = TempDir::new("agents-limits-background");
        let profile = logged_in_claude(&home);
        let starts = home.path().join("starts");
        let shell = executable_script(
            home.path(),
            "counting-shell",
            &format!("echo started >> '{}'\nexit 1", starts.display()),
        );
        let agents = agents_with_plugins_and_shell(
            home.path(),
            &shell.display().to_string(),
        );

        let loaded = read_limits(
            (&agents, NO_PROVIDERS),
            profile,
            NO_SHELL,
            home.path().join("agent-probe"),
            ProgramStarts::NotNow,
        );

        assert!(matches!(loaded.limits, LimitsRead::Failed(_)));
        assert!(!starts.exists(), "the shell was started");
    }

    /// While Agents mode is away `codex app-server` is not started either:
    /// the OpenAI plugin starts nothing for the read, falls back on its
    /// session files, and they hold no record of the limits. The shell
    /// here only counts its starts, so no real `codex` ever runs.
    #[test]
    fn a_read_in_the_background_starts_no_codex_app_server() {
        let home = TempDir::new("agents-limits-background-codex");
        let config_dir = home.path().join(".codex-test");
        write(
            &config_dir.join("auth.json"),
            r#"{"auth_mode":"chatgpt","tokens":{"access_token":"made-up-token","account_id":"00000000-0000-4000-8000-0000000000c1"}}"#,
        );
        let profile =
            Profile::added(AgentId::new("codex"), "Codex · Test", config_dir);
        let starts = home.path().join("starts");
        let shell = executable_script(
            home.path(),
            "counting-shell",
            &format!("echo started >> '{}'\nexit 1", starts.display()),
        );
        let agents = agents_with_plugins_and_shell(
            home.path(),
            &shell.display().to_string(),
        );

        let loaded = read_limits(
            (&agents, NO_PROVIDERS),
            profile,
            NO_SHELL,
            home.path().join("agent-probe"),
            ProgramStarts::NotNow,
        );

        assert!(matches!(loaded.limits, LimitsRead::Failed(_)));
        assert!(!starts.exists(), "the shell was started");
    }

    #[test]
    fn a_profile_of_an_unknown_agent_has_no_way_to_read_limits() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let loaded = read_limits(
            (&[], NO_PROVIDERS),
            profile,
            NO_SHELL,
            PathBuf::from("/no/such/cache/agent-probe"),
            ProgramStarts::Allowed,
        );

        assert_eq!(loaded.limits, LimitsRead::NoSources);
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-limits-task");
        let profile = logged_out_claude(&home);
        let agents = agents_with_plugins(home.path());

        let answers = run(load_limits(
            &agents,
            &no_providers(),
            profile.clone(),
            NO_SHELL,
            home.path(),
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::LimitsLoaded(loaded)]
                if loaded.profile == profile
                    && matches!(loaded.limits, LimitsRead::Failed(_))
        ));
    }

    #[test]
    fn the_read_in_the_background_answers_with_an_intent() {
        let home = TempDir::new("agents-limits-background-task");
        let profile = logged_out_claude(&home);
        let agents = agents_with_plugins(home.path());

        let answers = run(load_limits_in_background(
            &agents,
            &no_providers(),
            profile.clone(),
            NO_SHELL,
            home.path(),
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::LimitsLoaded(loaded)]
                if loaded.profile == profile
                    && matches!(loaded.limits, LimitsRead::Failed(_))
        ));
    }

    /// A read that stopped before it answered fails at the agent part with
    /// why, so the last good limits stay, greyed, and the user is told; it
    /// never says nothing reads the limits.
    #[test]
    fn a_read_that_stopped_fails_at_the_agent_part_because_it_stopped() {
        let home = TempDir::new("agents-limits-stopped");
        let profile = logged_out_claude(&home);

        let loaded = stopped_read(profile.clone(), true);

        assert_eq!(loaded.profile, profile);
        assert_eq!(
            loaded.limits,
            LimitsRead::Failed(vec![SourceFailure::new(
                SourceId::AgentPart,
                read_stopped()
            )])
        );
    }

    #[test]
    fn a_stopped_read_of_an_unknown_agent_has_no_way_to_read() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let loaded = stopped_read(profile, false);

        assert_eq!(loaded.limits, LimitsRead::NoSources);
    }
}
