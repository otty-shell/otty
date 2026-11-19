//! A profile: one agent with one config folder and a name (spec Decision 1).

use std::path::{Path, PathBuf};

use crate::agent::AgentId;
use crate::limits::BillingKind;

/// One agent together with one config folder and a name.
///
/// Two profiles of the same agent are two accounts, such as "Claude Code"
/// in `~/.claude` and "Claude Code · Work" in `~/.claude-work`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    agent: AgentId,
    name: String,
    config_dir: PathBuf,
    is_default: bool,
    is_in_default_config_dir: bool,
}

impl Profile {
    /// The agent of this profile.
    pub fn agent(&self) -> AgentId {
        self.agent
    }

    /// The name shown to the user.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the agent keeps its login, settings and sessions.
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Whether this is the agent's profile in its default config folder.
    pub fn is_default(&self) -> bool {
        self.is_default
    }

    /// Whether the config folder is the agent's own default folder, the
    /// one from [`Agent::default_config_dir`]. The agent then needs no
    /// variable to find it.
    ///
    /// [`Agent::default_config_dir`]: crate::Agent::default_config_dir
    pub(crate) fn is_in_default_config_dir(&self) -> bool {
        self.is_in_default_config_dir
    }

    /// The profile of an agent in its default config folder.
    ///
    /// The profile counts as being in the agent's default folder. The list
    /// of profiles ([`profiles`](crate::profiles)) checks this against the
    /// agent's real default folder.
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_agents::{AgentId, Profile};
    ///
    /// let profile = Profile::default_for(
    ///     AgentId::new("codex"),
    ///     "Codex",
    ///     PathBuf::from("/home/user/.codex"),
    /// );
    ///
    /// assert!(profile.is_default());
    /// ```
    pub fn default_for(
        agent: AgentId,
        name: impl Into<String>,
        config_dir: PathBuf,
    ) -> Self {
        Self::new(agent, name.into(), config_dir, true)
    }

    /// A profile the user added.
    ///
    /// The profile counts as being outside the agent's default folder. The
    /// list of profiles ([`profiles`](crate::profiles)) checks this against
    /// the agent's real default folder.
    pub fn added(
        agent: AgentId,
        name: impl Into<String>,
        config_dir: PathBuf,
    ) -> Self {
        Self::new(agent, name.into(), config_dir, false)
    }

    /// The same profile, knowing the agent's default config folder:
    /// whether its own folder is that one decides whether the commands set
    /// the agent's config folder variable.
    pub(crate) fn with_agent_default_dir(mut self, default_dir: &Path) -> Self {
        self.is_in_default_config_dir = self.config_dir == default_dir;
        self
    }

    fn new(
        agent: AgentId,
        name: String,
        config_dir: PathBuf,
        is_default: bool,
    ) -> Self {
        Self {
            agent,
            name,
            config_dir,
            is_default,
            is_in_default_config_dir: is_default,
        }
    }
}

/// Whether a profile can be used, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileStatus {
    /// Installed and logged in.
    Connected {
        /// The account the profile is logged in with, when known.
        account: Option<String>,
        /// The name of the account's plan, when known.
        plan: Option<String>,
        /// How the account is paid for, when the profile's files tell;
        /// `None` when only the limits answer tells, as for a ChatGPT login
        /// of Codex.
        billing: Option<BillingKind>,
    },
    /// Installed, but nobody is logged in with this config folder.
    NotLoggedIn,
    /// The agent's program was not found.
    ProgramNotFound,
    /// The config folder does not exist.
    ConfigDirNotFound,
}
