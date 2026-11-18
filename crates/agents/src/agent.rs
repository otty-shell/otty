//! A coding agent OTTY knows, and how it is told apart from the others.

use std::path::PathBuf;
use std::sync::Arc;

use crate::bill::BillSource;
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::hidden_terminal::StartTerminal;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::session::{SessionId, SessionSource};
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The id of an agent, such as `claude-code` or `codex`.
///
/// It is the name the agent is saved under, so it never changes once an
/// agent is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(&'static str);

impl AgentId {
    /// The id as text.
    pub fn as_str(&self) -> &'static str {
        self.0
    }

    /// An agent id from its text.
    ///
    /// ```
    /// use otty_agents::AgentId;
    ///
    /// assert_eq!(AgentId::new("codex").as_str(), "codex");
    /// ```
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }
}

/// What OTTY knows of an agent without asking it anything: its id, its
/// name and icon, its program, its config folder and the variable that
/// points the program at another one, the provider of its models, and
/// whether it reads a bill and statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentDescriptor {
    id: AgentId,
    name: &'static str,
    icon: &'static str,
    program: &'static str,
    config_dir_name: &'static str,
    config_dir_variable: &'static str,
    provider: &'static str,
    has_bill: bool,
    has_stats: bool,
}

impl AgentDescriptor {
    /// The id of the agent.
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// The name shown to the user, such as "Claude Code".
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The name of the agent's icon among the app's icons.
    pub fn icon(&self) -> &'static str {
        self.icon
    }

    /// The name of the agent's program, looked up on `PATH`, such as
    /// `claude`.
    pub fn program(&self) -> &'static str {
        self.program
    }

    /// The name of the agent's default config folder in the home folder,
    /// such as `.claude`. The folder proposed for a new profile starts with
    /// it, such as `.claude-work`.
    pub fn config_dir_name(&self) -> &'static str {
        self.config_dir_name
    }

    /// The environment variable that points the agent's program at a
    /// config folder, such as `CLAUDE_CONFIG_DIR`. When the app found it
    /// set, its folder is the one of the default profile.
    pub fn config_dir_variable(&self) -> &'static str {
        self.config_dir_variable
    }

    /// The company whose models the agent uses and whose API an API key
    /// pays, such as "Anthropic". The profile dialog shows it next to the
    /// agent's name.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// Whether the agent reads the real bill of an API key account with an
    /// admin key: its [`Agent::bill_source`] gives one (spec Decision 9).
    pub fn has_bill(&self) -> bool {
        self.has_bill
    }

    /// Whether the agent reads the statistics of a session and of a
    /// project: its [`Agent::stats_source`] gives one (spec Decision 14).
    pub fn has_stats(&self) -> bool {
        self.has_stats
    }

    /// A descriptor from its parts, of an agent that reads no bill and no
    /// statistics.
    ///
    /// ```
    /// use otty_agents::{AgentDescriptor, AgentId};
    ///
    /// const CODEX: AgentDescriptor = AgentDescriptor::new(
    ///     AgentId::new("codex"),
    ///     "Codex",
    ///     "codex",
    ///     "codex",
    ///     ".codex",
    ///     "CODEX_HOME",
    ///     "OpenAI",
    /// );
    ///
    /// assert_eq!(CODEX.name(), "Codex");
    /// assert_eq!(CODEX.config_dir_variable(), "CODEX_HOME");
    /// assert!(!CODEX.has_stats());
    /// ```
    pub const fn new(
        id: AgentId,
        name: &'static str,
        icon: &'static str,
        program: &'static str,
        config_dir_name: &'static str,
        config_dir_variable: &'static str,
        provider: &'static str,
    ) -> Self {
        Self {
            id,
            name,
            icon,
            program,
            config_dir_name,
            config_dir_variable,
            provider,
            has_bill: false,
            has_stats: false,
        }
    }

    /// The same descriptor of an agent that reads a bill.
    pub const fn with_bill(self) -> Self {
        Self {
            has_bill: true,
            ..self
        }
    }

    /// The same descriptor of an agent that reads statistics.
    ///
    /// ```
    /// use otty_agents::{AgentDescriptor, AgentId};
    ///
    /// const CODEX: AgentDescriptor = AgentDescriptor::new(
    ///     AgentId::new("codex"),
    ///     "Codex",
    ///     "codex",
    ///     "codex",
    ///     ".codex",
    ///     "CODEX_HOME",
    ///     "OpenAI",
    /// )
    /// .with_stats();
    ///
    /// assert!(CODEX.has_stats());
    /// assert!(!CODEX.has_bill());
    /// ```
    pub const fn with_stats(self) -> Self {
        Self {
            has_stats: true,
            ..self
        }
    }
}

/// A coding agent OTTY knows (spec Decision 4).
///
/// It is a trait because there are several real agents, each with its own
/// files, programs and servers. What an agent can not do, it does not
/// return: an empty list of sources or no spend counter means the mode does
/// not show that part for its profiles.
///
/// All methods are plain blocking calls; the app runs them on a background
/// thread, so an agent can be shared between threads.
pub trait Agent: Send + Sync {
    /// Id, name and icon.
    fn descriptor(&self) -> &AgentDescriptor;

    /// Config folder of the default profile.
    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf;

    /// Whether the profile is installed and logged in.
    fn detect(&self, profile: &Profile) -> ProfileStatus;

    /// Program, arguments and variables that start a new session, or
    /// continue `session` when it is given.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand;

    /// Program, arguments and variables that log the profile in.
    fn login(&self, profile: &Profile) -> AgentCommand;

    /// Ways to list sessions, best first. Empty: not supported.
    fn session_sources(&self) -> Vec<Box<dyn SessionSource>>;

    /// Ways to read the limits, best first. Empty: not supported.
    ///
    /// A source that asks the agent's own program starts it with
    /// `programs`, and a source that reads the program's screen starts it
    /// in a hidden terminal with `terminals`.
    fn limit_sources(
        &self,
        programs: Arc<dyn StartProgram>,
        terminals: Arc<dyn StartTerminal>,
    ) -> Vec<Box<dyn LimitSource>>;

    /// Counts the tokens spent from local files. `None`: not supported.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;

    /// Reads the real bill of an API key account with an admin key (spec
    /// Decision 9). `None`: the agent's provider has no bill OTTY can read,
    /// and its API key profiles show only the estimate. It gives one
    /// exactly when [`AgentDescriptor::has_bill`] says so.
    fn bill_source(&self) -> Option<Box<dyn BillSource>>;

    /// Reads the statistics of a session and of a project folder from
    /// local files (spec Decision 14). `None`: not supported, and Show
    /// statistics is off for the agent's sessions. It gives one exactly
    /// when [`AgentDescriptor::has_stats`] says so.
    fn stats_source(&self) -> Option<Box<dyn StatsSource>>;
}

/// The agent of `agents` with the id `id`; `None` when it is not one of
/// them, such as the agent of a profile saved by a newer OTTY.
///
/// ```
/// use otty_agents::{AgentId, agent_of, known_agents};
///
/// let agents = known_agents();
///
/// let codex = agent_of(&agents, AgentId::new("codex"));
///
/// assert_eq!(codex.map(|agent| agent.descriptor().name()), Some("Codex"));
/// assert!(agent_of(&agents, AgentId::new("cursor")).is_none());
/// ```
pub fn agent_of(agents: &[Box<dyn Agent>], id: AgentId) -> Option<&dyn Agent> {
    agents
        .iter()
        .find(|agent| agent.descriptor().id() == id)
        .map(|agent| agent.as_ref())
}
