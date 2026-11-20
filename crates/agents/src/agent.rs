//! A coding agent OTTY knows, and how it is told apart from the others.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use otty_plugin_host::agent::Login;

use crate::account::Account;
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::hidden_terminal::StartTerminal;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::profile_list::found_in_home;
use crate::program::StartProgram;
use crate::registry::PluginPart;
use crate::session::{SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The id of an agent, as the agent part of its plugin names it.
///
/// It is the name the agent is saved under, so it never changes once an
/// agent is released; a new id is a new agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(String);

impl AgentId {
    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// An agent id from its text.
    ///
    /// ```
    /// use otty_agents::AgentId;
    ///
    /// assert_eq!(AgentId::new("coder").as_str(), "coder");
    /// ```
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// What the app shows and needs to know of an agent without asking it
/// anything (spec "Links in the interface"): its id, name and icon, its
/// program, the provider its models come from, and the plugin part that
/// reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEntry {
    id: AgentId,
    name: String,
    icon: Option<Cow<'static, [u8]>>,
    program: String,
    provider_name: String,
    part: PluginPart,
    keyring_services: Vec<String>,
}

impl AgentEntry {
    /// The id of the agent.
    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// The name shown to the user.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The SVG of its icon, in one color the app tints; `None` when it has
    /// none, and the app draws the first letter of its name.
    pub fn icon(&self) -> Option<&Cow<'static, [u8]>> {
        self.icon.as_ref()
    }

    /// The name of the agent's program, looked up on `PATH`.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The company whose models the agent uses and whose API an API key
    /// pays: the name of the plugin that reads it. The profile dialog
    /// shows it next to the agent's name.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// The plugin part that reads the agent's profiles.
    pub fn part(&self) -> &PluginPart {
        &self.part
    }

    /// The services of the system keyring the agent's part may read (spec
    /// 2026-11-19, Decision 8); empty when it reads none.
    pub fn keyring_services(&self) -> &[String] {
        &self.keyring_services
    }

    /// An entry with no icon, read by `part`.
    ///
    /// ```
    /// use otty_agents::{AgentEntry, AgentId, PluginPart};
    ///
    /// let part = PluginPart::new("example", "Example", "0.1.0", "", "coder");
    /// let coder = AgentEntry::new(
    ///     AgentId::new("coder"),
    ///     "Coder",
    ///     "coder",
    ///     "Example",
    ///     part,
    /// );
    ///
    /// assert_eq!(coder.name(), "Coder");
    /// assert!(coder.icon().is_none());
    /// ```
    pub fn new(
        id: AgentId,
        name: impl Into<String>,
        program: impl Into<String>,
        provider_name: impl Into<String>,
        part: PluginPart,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            icon: None,
            program: program.into(),
            provider_name: provider_name.into(),
            part,
            keyring_services: Vec::new(),
        }
    }

    /// The same entry with the SVG `icon`.
    pub fn with_icon(mut self, icon: impl Into<Cow<'static, [u8]>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// The same entry whose part may read the services `services` of the
    /// system keyring.
    pub fn with_keyring_services(mut self, services: Vec<String>) -> Self {
        self.keyring_services = services;
        self
    }
}

/// Where an agent keeps its config folder: the folder's name in the home
/// folder and the environment variable that points the agent's program at
/// another folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFolder {
    name: Cow<'static, str>,
    variable: Cow<'static, str>,
}

impl ConfigFolder {
    /// The name of the agent's default config folder in the home folder,
    /// such as `.coder`. The folder proposed for a new profile starts with
    /// it, such as `.coder-work`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The environment variable that points the agent's program at a
    /// config folder, such as `CODER_HOME`. When the app found it
    /// set, its folder is the one of the default profile, or holds it.
    pub fn variable(&self) -> &str {
        &self.variable
    }

    /// The config folder `name` in the home folder, which `variable`
    /// moves.
    ///
    /// ```
    /// use otty_agents::ConfigFolder;
    ///
    /// const CODER: ConfigFolder = ConfigFolder::new(".coder", "CODER_HOME");
    ///
    /// assert_eq!(CODER.variable(), "CODER_HOME");
    /// ```
    pub const fn new(name: &'static str, variable: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            variable: Cow::Borrowed(variable),
        }
    }

    /// The config folder `name` an agent part of a plugin names, which
    /// the variable `variable` moves; empty when the part lists none.
    pub fn of_plugin(name: String, variable: String) -> Self {
        Self {
            name: Cow::Owned(name),
            variable: Cow::Owned(variable),
        }
    }
}

/// A coding agent OTTY knows (spec Decision 4).
///
/// It is a trait because each agent part of a plugin is one
/// ([`PluginAgent`](crate::PluginAgent)), and the tests of this crate and
/// of the app give made-up agents in their place. What an agent can not do, it does not
/// return: no source or no spend counter means the mode does not show that
/// part for its profiles.
///
/// All methods are plain blocking calls; the app runs them on a background
/// thread, so an agent can be shared between threads.
pub trait Agent: Send + Sync {
    /// The entry the agent puts in the registry: its id, name, icon,
    /// program and provider, and the plugin part that reads it.
    fn entry(&self) -> &AgentEntry;

    /// Where the agent keeps its config folder.
    fn config_folder(&self) -> ConfigFolder;

    /// Whether the agent writes what each answer cost: its money is that
    /// cost, exact, not an estimate at the list prices.
    fn writes_cost(&self) -> bool;

    /// Config folder of the default profile.
    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf;

    /// Whether the profile is installed and logged in.
    fn detect(&self, profile: &Profile) -> ProfileStatus;

    /// The accounts found in the files of the profile: its logins and the
    /// keys it uses, each found in the profile's folder (spec Decision 2).
    /// Empty: none found. The same account found in several profiles is
    /// listed by each; [`merge_accounts`] makes it one.
    ///
    /// It is a method of the agent because each agent keeps its logins and
    /// keys in its own files. Only reads them: a key is kept only as its
    /// fingerprint, and a token never.
    ///
    /// [`merge_accounts`]: crate::merge_accounts
    fn accounts(&self, profile: &Profile) -> Vec<Account>;

    /// Whether the folder of `profile` holds a session of the agent,
    /// without reading one. An extra folder of the agent in the home folder
    /// is a profile when it holds a login or a session (spec Decision 2).
    fn has_sessions(&self, profile: &Profile) -> bool;

    /// The version of the agent's program as the agent wrote it in the
    /// newest session of `profile`, such as "2.1.328". `None` when no
    /// session names one; the app then asks the program itself.
    fn version(&self, profile: &Profile) -> Option<String>;

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

    /// The ids of the sources [`Agent::session_sources`] gives, in the
    /// same order, known without making the sources.
    fn session_source_ids(&self) -> &'static [SourceId];

    /// The read of the limits the agent itself shows. `None`: the agent
    /// shows none.
    ///
    /// A read that asks the agent's own program starts it with
    /// `programs`, and one that reads the program's screen starts it in a
    /// hidden terminal with `terminals`.
    fn limit_source(
        &self,
        programs: Arc<dyn StartProgram>,
        terminals: Arc<dyn StartTerminal>,
    ) -> Option<Box<dyn LimitSource>>;

    /// Counts the tokens spent from local files. `None`: not supported.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;

    /// Reads the statistics of a session and of a project folder from
    /// local files (spec Decision 14). `None`: not supported, and Show
    /// statistics is off for the agent's sessions.
    fn stats_source(&self) -> Option<Box<dyn StatsSource>>;

    /// The config folders of the agent besides its default one found on
    /// this computer: by default the folders in the home folder whose
    /// names start with the name of its config folder and a `-` or `_`,
    /// such as `~/.coder-work`, that hold a login or a session (spec
    /// 2026-11-19, Decision 2). A plugin part finds them itself.
    fn found_config_dirs(&self, env: &AgentEnv) -> Vec<PathBuf> {
        found_in_home(self, env)
    }

    /// The login of `account` as the plugin part that found it in
    /// `profile` names it, with the place its key sits, so that a
    /// provider part can be given a handle to it. `None` when the part no
    /// longer finds it there.
    fn found_login(
        &self,
        _profile: &Profile,
        _account: &Account,
    ) -> Option<Login> {
        None
    }

    /// Whether the agent reads the statistics of a session and of a
    /// project: its [`Agent::stats_source`] gives one (spec Decision 14).
    fn has_stats(&self) -> bool {
        self.stats_source().is_some()
    }
}

/// The agent of `agents` with the id `id`; `None` when it is not one of
/// them, such as the agent of a profile saved by a newer OTTY.
///
/// ```
/// use otty_agents::fixtures::codex_stand_in;
/// use otty_agents::{AgentId, agent_of};
///
/// let agents = vec![codex_stand_in()];
///
/// let found = agent_of(&agents, &AgentId::new("codex"));
///
/// assert_eq!(found.map(|agent| agent.entry().name()), Some("Codex"));
/// assert!(agent_of(&agents, &AgentId::new("cursor")).is_none());
/// ```
pub fn agent_of<'a>(
    agents: &'a [Box<dyn Agent>],
    id: &AgentId,
) -> Option<&'a dyn Agent> {
    agents
        .iter()
        .find(|agent| agent.entry().id() == id)
        .map(|agent| agent.as_ref())
}
