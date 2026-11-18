//! The read of the limits of one profile. Its sources may ask a server over
//! the network or start the agent's program, so it runs on a thread of its
//! own.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{
    Agent, AgentCommand, Profile, ProgramError, RunningProgram,
    RunningTerminal, StartProgram, StartTerminal,
};
use otty_app_agents::{AgentList, AgentsIntent, LimitsRead, LoadedLimits};

use crate::agent_program::LoginShellPrograms;
use crate::hidden_terminal::LoginShellTerminals;
use crate::load::on_thread;

/// The folder in OTTY's cache folder that an agent's program started in a
/// hidden terminal runs in, so it never shows up as a session of a real
/// project.
const PROBE_FOLDER: &str = "agent-probe";

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

/// Read the limits of `profile` in the background, from the sources of the
/// agent of `agents` it belongs to, best first. A source that asks the
/// agent's program starts it through the user's shell at `shell`, the way
/// the agent tab does; a source that reads the program's screen starts it
/// in a hidden terminal in a folder of its own inside OTTY's cache folder
/// `cache_dir`. The answer comes back as [`AgentsIntent::LimitsLoaded`].
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
///     &PathBuf::from("/home/user/.cache/otty"),
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_limits(
    agents: AgentList,
    profile: Profile,
    shell: &str,
    cache_dir: &Path,
) -> Task<AgentsIntent> {
    read_task(agents, profile, shell, cache_dir, ProgramStarts::Allowed)
}

/// Read the limits of `profile` in the background while Agents mode is not
/// on screen, as [`load_limits`] does, except that no source starts the
/// agent's program, neither `codex app-server` nor `claude` in a hidden
/// terminal: that source says why, and a read on screen starts it (spec
/// Decision 10).
pub fn load_limits_in_background(
    agents: AgentList,
    profile: Profile,
    shell: &str,
    cache_dir: &Path,
) -> Task<AgentsIntent> {
    read_task(agents, profile, shell, cache_dir, ProgramStarts::NotNow)
}

/// The read of the limits of `profile` on a thread of its own, whose
/// sources may start the agent's program as `starts` says.
fn read_task(
    agents: AgentList,
    profile: Profile,
    shell: &str,
    cache_dir: &Path,
    starts: ProgramStarts,
) -> Task<AgentsIntent> {
    let fallback = profile.clone();
    let shell = shell.to_string();
    let probe_dir = cache_dir.join(PROBE_FOLDER);

    Task::perform(
        on_thread(move || {
            read_limits(&agents, profile, &shell, probe_dir, starts)
        }),
        move |loaded| {
            let loaded = loaded.unwrap_or_else(|| {
                log::warn!("reading the limits of {} stopped", fallback.name());
                LoadedLimits {
                    profile: fallback,
                    limits: LimitsRead::NoSources,
                    sources: Vec::new(),
                    read_at: SystemTime::now(),
                }
            });
            AgentsIntent::LimitsLoaded(loaded)
        },
    )
}

/// The limits of `profile` from the first source of its agent that
/// answers, with the ids of all its sources and the time the read
/// ended. The agent's program starts only as `starts` says; in a hidden
/// terminal it runs in `probe_dir`. An agent OTTY does not know has no
/// sources.
fn read_limits(
    agents: &[Box<dyn Agent>],
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

    let sources = match agent {
        Some(agent) => agent.limit_sources(programs, terminals),
        None => Vec::new(),
    };

    let limits =
        LimitsRead::from_walk(otty_agents::read_limits(&sources, &profile));

    LoadedLimits {
        profile,
        limits,
        sources: sources.iter().map(|source| source.id()).collect(),
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use otty_agents::{
        AgentId, ProgramError, SourceError, SourceId, Unavailable, known_agents,
    };
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
        let probe_dir = home.path().join("agent-probe");
        let before = SystemTime::now();

        let loaded = read_limits(
            &known_agents(),
            profile.clone(),
            NO_SHELL,
            probe_dir,
            ProgramStarts::Allowed,
        );

        assert_eq!(loaded.profile, profile);
        let LimitsRead::Failed(failures) = loaded.limits else {
            panic!("no source answers");
        };
        let reasons: Vec<_> = failures
            .iter()
            .map(|failure| (failure.source(), failure.error().clone()))
            .collect();
        let no_token = SourceError::NotAvailable(Unavailable::NoLoginToken);
        let no_shell = SourceError::NotAvailable(Unavailable::NotStarted {
            program: String::from("claude"),
            reason: ProgramError::ShellNotFound(String::from(NO_SHELL)),
        });
        assert_eq!(
            reasons,
            [
                (SourceId::AnthropicUsageApi, no_token),
                (SourceId::ClaudeUsageCommand, no_shell)
            ]
        );
        assert_eq!(
            loaded.sources,
            [SourceId::AnthropicUsageApi, SourceId::ClaudeUsageCommand]
        );
        assert!(loaded.read_at >= before);
    }

    /// While Agents mode is away the hidden terminal is not started: its
    /// source says why, and the next read on screen starts it.
    #[test]
    fn a_read_in_the_background_starts_no_hidden_terminal() {
        let home = TempDir::new("agents-limits-background");
        let profile = logged_out_claude(&home);
        let probe_dir = home.path().join("agent-probe");

        let loaded = read_limits(
            &known_agents(),
            profile,
            NO_SHELL,
            probe_dir,
            ProgramStarts::NotNow,
        );

        let LimitsRead::Failed(failures) = loaded.limits else {
            panic!("no source answers");
        };
        let probe = failures
            .iter()
            .find(|failure| failure.source() == SourceId::ClaudeUsageCommand)
            .map(|failure| failure.error().clone());
        assert_eq!(
            probe,
            Some(SourceError::NotAvailable(Unavailable::NotStarted {
                program: String::from("claude"),
                reason: ProgramError::NotNow,
            }))
        );
    }

    /// While Agents mode is away `codex app-server` is not started either:
    /// its source says why, and the next read on screen starts it.
    #[test]
    fn a_read_in_the_background_starts_no_codex_app_server() {
        let home = TempDir::new("agents-limits-background-codex");
        let config_dir = home.path().join(".codex-test");
        std::fs::create_dir_all(&config_dir).expect("the config folder");
        let profile =
            Profile::added(AgentId::new("codex"), "Codex · Test", config_dir);

        let loaded = read_limits(
            &known_agents(),
            profile,
            NO_SHELL,
            home.path().join("agent-probe"),
            ProgramStarts::NotNow,
        );

        let LimitsRead::Failed(failures) = loaded.limits else {
            panic!("no source answers");
        };
        let app_server = failures
            .iter()
            .find(|failure| failure.source() == SourceId::CodexAppServer)
            .map(|failure| failure.error().clone());
        assert_eq!(
            app_server,
            Some(SourceError::NotAvailable(Unavailable::NotStarted {
                program: String::from("codex"),
                reason: ProgramError::NotNow,
            }))
        );
    }

    #[test]
    fn a_profile_of_an_unknown_agent_has_no_way_to_read_limits() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let loaded = read_limits(
            &known_agents(),
            profile,
            NO_SHELL,
            PathBuf::from("/no/such/cache/agent-probe"),
            ProgramStarts::Allowed,
        );

        assert_eq!(loaded.limits, LimitsRead::NoSources);
        assert!(loaded.sources.is_empty());
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-limits-task");
        let profile = logged_out_claude(&home);
        let agents: AgentList = Arc::from(known_agents());

        let answers =
            run(load_limits(agents, profile.clone(), NO_SHELL, home.path()));

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
        let agents: AgentList = Arc::from(known_agents());

        let answers = run(load_limits_in_background(
            agents,
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
}
