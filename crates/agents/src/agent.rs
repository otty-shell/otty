//! A coding agent OTTY knows, and how it is told apart from the others.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use crate::account::Account;
use crate::account_key::AccountKey;
use crate::bill::BillSource;
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::hidden_terminal::StartTerminal;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::registry::Reader;
use crate::session::{SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The id of an agent, such as `claude-code` or `codex`.
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
    /// assert_eq!(AgentId::new("codex").as_str(), "codex");
    /// ```
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// What the app shows and needs to know of an agent without asking it
/// anything (spec "Links in the interface"): its id, name and icon, its
/// program, the provider its models come from, and what reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEntry {
    id: AgentId,
    name: String,
    icon: Option<Cow<'static, [u8]>>,
    program: String,
    provider_name: String,
    reader: Reader,
}

impl AgentEntry {
    /// The id of the agent.
    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// The name shown to the user, such as "Claude Code".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The SVG of its icon, in one color the app tints; `None` when it has
    /// none, and the app draws the first letter of its name.
    pub fn icon(&self) -> Option<&Cow<'static, [u8]>> {
        self.icon.as_ref()
    }

    /// The name of the agent's program, looked up on `PATH`, such as
    /// `claude`.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The company whose models the agent uses and whose API an API key
    /// pays, such as "Anthropic". The profile dialog shows it next to the
    /// agent's name.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// What reads the agent's profiles.
    pub fn reader(&self) -> &Reader {
        &self.reader
    }

    /// An entry with no icon.
    ///
    /// ```
    /// use otty_agents::{AgentEntry, AgentId, Reader};
    ///
    /// let codex = AgentEntry::new(
    ///     AgentId::new("codex"),
    ///     "Codex",
    ///     "codex",
    ///     "OpenAI",
    ///     Reader::BuiltIn,
    /// );
    ///
    /// assert_eq!(codex.name(), "Codex");
    /// assert!(codex.icon().is_none());
    /// ```
    pub fn new(
        id: AgentId,
        name: impl Into<String>,
        program: impl Into<String>,
        provider_name: impl Into<String>,
        reader: Reader,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            icon: None,
            program: program.into(),
            provider_name: provider_name.into(),
            reader,
        }
    }

    /// The same entry with the SVG `icon`.
    pub fn with_icon(mut self, icon: impl Into<Cow<'static, [u8]>>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Where an agent keeps its config folder: the folder's name in the home
/// folder and the environment variables that point the agent's program at
/// another folder or another database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigFolder {
    name: &'static str,
    variable: &'static str,
    database_variable: Option<&'static str>,
}

impl ConfigFolder {
    /// The name of the agent's default config folder in the home folder,
    /// such as `.claude`. The folder proposed for a new profile starts with
    /// it, such as `.claude-work`.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The environment variable that points the agent's program at a
    /// config folder, such as `CLAUDE_CONFIG_DIR`. When the app found it
    /// set, its folder is the one of the default profile, or holds it, as
    /// `XDG_DATA_HOME` holds the `kilo` folder of Kilo.
    pub fn variable(&self) -> &'static str {
        self.variable
    }

    /// The environment variable that points the agent's program at
    /// another database file, such as `KILO_DB`; `None` for an agent with
    /// no database. When the app found it set, the default profile reads
    /// that database.
    pub fn database_variable(&self) -> Option<&'static str> {
        self.database_variable
    }

    /// The config folder `name` in the home folder, which `variable`
    /// moves.
    ///
    /// ```
    /// use otty_agents::ConfigFolder;
    ///
    /// const CODEX: ConfigFolder = ConfigFolder::new(".codex", "CODEX_HOME");
    ///
    /// assert_eq!(CODEX.variable(), "CODEX_HOME");
    /// assert_eq!(CODEX.database_variable(), None);
    /// ```
    pub const fn new(name: &'static str, variable: &'static str) -> Self {
        Self {
            name,
            variable,
            database_variable: None,
        }
    }

    /// The same folder of an agent that keeps its sessions in a database
    /// file, which the environment variable `variable` moves.
    pub const fn with_database_variable(self, variable: &'static str) -> Self {
        Self {
            database_variable: Some(variable),
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
    /// The entry the agent puts in the registry: its id, name, icon,
    /// program and provider, read by this built-in reader.
    fn entry(&self) -> &AgentEntry;

    /// Where the agent keeps its config folder.
    fn config_folder(&self) -> ConfigFolder;

    /// Whether the agent writes what each answer cost, as Kilo and
    /// OpenCode do: its money is that cost, exact, not an estimate at the
    /// list prices.
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

    /// The key or the login token of `account`, read again from the files
    /// of `profile` it was found in, for one request that reads the
    /// account's own numbers, such as the money left on a DeepSeek key or
    /// the limits of a ChatGPT login Kilo keeps (spec 2026-11-19, Decision
    /// 5). `None` when the files hold it no longer, or the agent's
    /// accounts have no numbers of their own to read.
    ///
    /// It only reads: the key is never kept or written, and a login is
    /// never renewed.
    fn account_key(
        &self,
        profile: &Profile,
        account: &Account,
    ) -> Option<AccountKey>;

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

    /// The ids of the sources [`Agent::limit_sources`] gives, in the same
    /// order, known without making the sources, so nothing that could
    /// start a program is needed.
    fn limit_source_ids(&self) -> &'static [SourceId];

    /// Counts the tokens spent from local files. `None`: not supported.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>>;

    /// Reads the real bill of an API key account with an admin key (spec
    /// Decision 9). `None`: the agent's provider has no bill OTTY can read,
    /// and its API key profiles show only the estimate.
    fn bill_source(&self) -> Option<Box<dyn BillSource>>;

    /// Reads the statistics of a session and of a project folder from
    /// local files (spec Decision 14). `None`: not supported, and Show
    /// statistics is off for the agent's sessions.
    fn stats_source(&self) -> Option<Box<dyn StatsSource>>;

    /// Whether the agent reads the real bill of an API key account with an
    /// admin key: its [`Agent::bill_source`] gives one (spec Decision 9).
    /// The app asks this, so it never makes a source it does not use.
    fn has_bill(&self) -> bool {
        self.bill_source().is_some()
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
/// use otty_agents::{AgentId, agent_of, known_agents};
///
/// let agents = known_agents();
///
/// let codex = agent_of(&agents, &AgentId::new("codex"));
///
/// assert_eq!(codex.map(|agent| agent.entry().name()), Some("Codex"));
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
