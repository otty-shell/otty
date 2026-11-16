//! A coding agent OTTY knows, and how it is told apart from the others.

use std::path::{Path, PathBuf};

use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::session::{SessionId, SessionSource};
use crate::spend::SpendCounter;

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

/// What the user sees of an agent: its id, its name, its icon and the name
/// of its config folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentDescriptor {
    id: AgentId,
    name: &'static str,
    icon: &'static str,
    config_dir_name: &'static str,
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

    /// The name of the agent's default config folder in the home folder,
    /// such as `.claude`. The folder proposed for a new profile starts with
    /// it, such as `.claude-work`.
    pub fn config_dir_name(&self) -> &'static str {
        self.config_dir_name
    }

    /// A descriptor from its parts.
    ///
    /// ```
    /// use otty_agents::{AgentDescriptor, AgentId};
    ///
    /// const CODEX: AgentDescriptor =
    ///     AgentDescriptor::new(AgentId::new("codex"), "Codex", "codex", ".codex");
    ///
    /// assert_eq!(CODEX.name(), "Codex");
    /// assert_eq!(CODEX.config_dir_name(), ".codex");
    /// ```
    pub const fn new(
        id: AgentId,
        name: &'static str,
        icon: &'static str,
        config_dir_name: &'static str,
    ) -> Self {
        Self {
            id,
            name,
            icon,
            config_dir_name,
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

    /// Whether the folder `dir` looks like a config folder of this agent,
    /// so the dialog that adds a profile can offer it.
    fn looks_like_config_dir(&self, dir: &Path) -> bool;

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
    fn limit_sources(&self) -> Vec<Box<dyn LimitSource>>;

    /// Counts the tokens spent from local files. `None`: not supported.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;
}
