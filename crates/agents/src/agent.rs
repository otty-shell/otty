//! A coding agent OTTY knows, and how it is told apart from the others.

use std::borrow::Cow;
use std::path::PathBuf;

use otty_plugin_host::PluginPart;

use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::profile::{Profile, ProfileStatus};
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
    program_places: Vec<PathBuf>,
    install: Option<AgentInstall>,
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

    /// The places on this computer the agent's official installer puts its
    /// program, which OTTY checks when the login shell does not find it by
    /// its name (issue 32).
    pub fn program_places(&self) -> &[PathBuf] {
        &self.program_places
    }

    /// How to install the agent's program, if its part says.
    pub fn install(&self) -> Option<&AgentInstall> {
        self.install.as_ref()
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
    /// use otty_agents::{AgentEntry, AgentId};
    /// use otty_plugin_host::PluginPart;
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
            program_places: Vec::new(),
            install: None,
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

    /// The same entry whose program its installer puts at one of `places`.
    pub fn with_program_places(mut self, places: Vec<PathBuf>) -> Self {
        self.program_places = places;
        self
    }

    /// The same entry whose program `install` installs.
    pub fn with_install(mut self, install: AgentInstall) -> Self {
        self.install = Some(install);
        self
    }
}

/// How to install an agent's program (issue 32): a command that runs in a
/// terminal tab the user sees, a page about it, or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInstall {
    command: Option<AgentCommand>,
    page: Option<String>,
}

impl AgentInstall {
    /// The command that installs the program; `None` when only a page is
    /// given.
    pub fn command(&self) -> Option<&AgentCommand> {
        self.command.as_ref()
    }

    /// The `https` page about installing the program.
    pub fn page(&self) -> Option<&str> {
        self.page.as_deref()
    }

    /// The install by `command`, with the page `page`.
    ///
    /// ```
    /// use otty_agents::{AgentCommand, AgentInstall};
    ///
    /// let install = AgentInstall::new(
    ///     Some(
    ///         AgentCommand::new("npm")
    ///             .with_arg("i")
    ///             .with_arg("-g")
    ///             .with_arg("coder"),
    ///     ),
    ///     Some(String::from("https://example.com/docs")),
    /// );
    ///
    /// assert_eq!(install.page(), Some("https://example.com/docs"));
    /// ```
    pub fn new(command: Option<AgentCommand>, page: Option<String>) -> Self {
        Self { command, page }
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

    /// Config folder of the default profile: the only folder of the agent
    /// OTTY lists by itself (spec 2026-11-23, Decision 4).
    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf;

    /// Whether the profile is installed and logged in.
    fn detect(&self, profile: &Profile) -> ProfileStatus;

    /// The version of the agent's program as the agent wrote it in the
    /// newest session of `profile`, such as "2.1.328". `None` when no
    /// session names one; the app then asks the program itself.
    fn version(&self, profile: &Profile) -> Option<String>;

    /// OTTY found the agent's program at `place`, one of its
    /// [`AgentEntry::program_places`], since the login shell does not find
    /// it by its name; `None`: the shell finds it, or it is nowhere. An
    /// agent that starts programs starts its own by that path from now on;
    /// by default it is not told.
    fn start_program_from(&self, _place: Option<PathBuf>) {}

    /// The agent's program as OTTY starts it: its name, or the full path it
    /// was found at (see [`Agent::start_program_from`]).
    fn program_to_start(&self) -> String {
        self.entry().program().to_string()
    }

    /// Program, arguments and variables that start a new session, or
    /// continue `session` when it is given.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand;

    /// Ways to list sessions, best first. Empty: not supported.
    fn session_sources(&self) -> Vec<Box<dyn SessionSource>>;

    /// The ids of the sources [`Agent::session_sources`] gives, in the
    /// same order, known without making the sources.
    fn session_source_ids(&self) -> &'static [SourceId];

    /// Counts the tokens spent from local files. `None`: not supported.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;

    /// Reads the statistics of a session and of a project folder from
    /// local files (spec Decision 14). `None`: not supported, and Show
    /// statistics is off for the agent's sessions.
    fn stats_source(&self) -> Option<Box<dyn StatsSource>>;

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
