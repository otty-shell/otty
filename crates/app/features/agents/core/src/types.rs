use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    Account, AccountNumbers, Agent, AgentEnv, AgentId, AgentSession, Answer,
    Bill, FetchedPrices, GivenKey, Limits, Profile, ProfileStatus,
    ProjectStats, Registry, SavedProfiles, SessionId, SessionStats,
    SourceError, SourceFailure, Spend,
};

/// The agents OTTY knows, shared by the feature and the reads that run on
/// a background thread.
///
/// The app makes it from the agent parts of the plugins that are on and
/// keeps it until they change: an agent may remember what it read between
/// two refreshes, such as the session files it counted.
pub type AgentList = Arc<[Box<dyn Agent>]>;

/// What Agents mode reads with, swapped in when the installed plugins
/// change: the agents, the registry that names them and their providers,
/// and what the agents look at in the environment. A plain bag of shared
/// values; two bags are the same when they share their agents.
#[derive(Clone)]
pub struct AgentReaders {
    /// The agents, plugin parts first.
    pub agents: AgentList,
    /// The agents and providers OTTY knows now.
    pub registry: Arc<Registry>,
    /// What the agents look at in the environment.
    pub env: AgentEnv,
}

impl PartialEq for AgentReaders {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.agents, &other.agents)
            && self.registry == other.registry
            && self.env == other.env
    }
}

impl std::fmt::Debug for AgentReaders {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentReaders")
            .field("agents", &self.agents.len())
            .field("registry", &self.registry)
            .field("env", &self.env)
            .finish()
    }
}

/// The answer of the profiles read: what `otty-app-agents-io` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProfiles {
    /// Every profile to show and read, in the order to show them.
    pub profiles: Vec<LoadedProfile>,
    /// The profiles the user hid, in their order: they were not checked
    /// and nothing of them is read (spec 2026-11-19, Decision 7).
    pub hidden_profiles: Vec<Profile>,
    /// What `agents.json` holds; `None` when it could not be read, so the
    /// feature does not write over it.
    pub saved: Option<SavedProfiles>,
    /// Why `agents.json` could not be read, ready to show; the profiles
    /// are then the defaults.
    pub problem: Option<String>,
    /// The user's time zone, read with the profiles, before any limits;
    /// `None` when it could not be read. The dashboard writes its times in
    /// it.
    pub time_zone: Option<TimeZone>,
    /// When the read ended. Opening Agents mode reads the profiles again
    /// only when they are older than the period of its timer.
    pub read_at: SystemTime,
}

/// One profile, whether it can be used, and the accounts found in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProfile {
    /// The profile.
    pub profile: Profile,
    /// Whether it is installed and logged in.
    pub status: ProfileStatus,
    /// The logins and keys found in its files (spec Decision 2): the
    /// limits and the bill of an account are read once for all the
    /// profiles it was found in.
    pub accounts: Vec<Account>,
    /// The version of the agent's program, from the profile's session
    /// files or else from the program itself; `None` when neither tells.
    pub version: Option<String>,
}

/// The answer of a question to an agent's program for its version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedVersion {
    /// The agent whose program was asked.
    pub agent: AgentId,
    /// The version it printed; `None` when it printed none or failed.
    pub version: Option<String>,
}

/// The answer of the sessions read of one profile.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedSessions {
    /// The profile the sessions belong to.
    pub profile: Profile,
    /// The sessions, why each way to read them failed, or that the agent
    /// can not list sessions.
    pub sessions: SessionsRead,
    /// The project folders of these sessions that no longer exist.
    pub missing_folders: Vec<PathBuf>,
}

/// What a read of the sessions of one profile gave.
///
/// # Example
///
/// A walk over no sources fails with no failure at all: the agent can not
/// list sessions.
///
/// ```
/// use otty_app_agents::SessionsRead;
///
/// assert_eq!(
///     SessionsRead::from_walk(Err(Vec::new())),
///     SessionsRead::NoWay
/// );
/// assert_eq!(
///     SessionsRead::from_walk(Ok(Vec::new())),
///     SessionsRead::Read(Vec::new())
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum SessionsRead {
    /// The sessions the first way that answered gave.
    Read(Vec<AgentSession>),
    /// Every way to read them failed, each with why.
    Failed(Vec<SourceFailure>),
    /// The agent has no way to list sessions: the profile has none.
    NoWay,
}

impl SessionsRead {
    /// What the walk over the session sources of an agent gave, `walk`:
    /// no failures at all means the agent has no source.
    pub fn from_walk(
        walk: Result<Vec<AgentSession>, Vec<SourceFailure>>,
    ) -> Self {
        match walk {
            Ok(sessions) => Self::Read(sessions),
            Err(failures) if failures.is_empty() => Self::NoWay,
            Err(failures) => Self::Failed(failures),
        }
    }
}

/// The answer of the limits read of one profile.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedLimits {
    /// The profile the limits belong to.
    pub profile: Profile,
    /// The limits with the part that answered, why each part failed, or
    /// that nothing reads the limits of the profile.
    pub limits: LimitsRead,
    /// When the read ended.
    pub read_at: SystemTime,
}

/// What a read of the limits of one profile gave.
///
/// # Example
///
/// A read that no part answered and no part failed: nothing reads the
/// limits of the profile, or the provider part that reads its account had
/// no way to read them (spec 2026-11-19-agent-plugins, issue 29).
///
/// ```
/// use otty_app_agents::LimitsRead;
///
/// assert_eq!(
///     LimitsRead::from_walk(Err(Vec::new())),
///     LimitsRead::NoSources
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum LimitsRead {
    /// The limits with the part that answered: the provider part of the
    /// account whose limits the profile shows.
    Answered(Answer<Limits>),
    /// Every part failed, each with why.
    Failed(Vec<SourceFailure>),
    /// Nothing reads the limits: the profile has no subscription login,
    /// no plugin reads it, or its part had no way to read them.
    NoSources,
}

impl LimitsRead {
    /// What a walk over the parts that read the limits gave, `walk`: no
    /// failures at all means nothing reads the limits.
    pub fn from_walk(walk: Result<Answer<Limits>, Vec<SourceFailure>>) -> Self {
        match walk {
            Ok(answer) => Self::Answered(answer),
            Err(failures) if failures.is_empty() => Self::NoSources,
            Err(failures) => Self::Failed(failures),
        }
    }

    /// The failures of the parts the read asked before the one that
    /// answered, or of every part when none did; empty when nothing reads
    /// the limits.
    pub fn failures(&self) -> &[SourceFailure] {
        match self {
            Self::Answered(answer) => answer.earlier_failures(),
            Self::Failed(failures) => failures,
            Self::NoSources => &[],
        }
    }
}

/// What a count of the tokens one profile spent gave.
///
/// # Example
///
/// ```
/// use otty_agents::SourceError;
/// use otty_app_agents::SpendCount;
///
/// let failed = SpendCount::Failed(SourceError::Network("down".into()));
///
/// assert_ne!(failed, SpendCount::NoCounter);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpendCount {
    /// The tokens counted.
    Counted(Spend),
    /// Why the tokens could not be counted.
    Failed(SourceError),
    /// The agent can not count tokens.
    NoCounter,
}

/// The answer of the count of the tokens one profile spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSpend {
    /// The profile the tokens belong to.
    pub profile: Profile,
    /// The tokens, why they could not be counted, or that the agent can
    /// not count tokens.
    pub spend: SpendCount,
    /// When the count ended.
    pub counted_at: SystemTime,
}

/// The answer of the read of the bill of one organization, read through
/// one profile that pays through it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedBill {
    /// The profile the bill was read through.
    pub profile: Profile,
    /// The keys given for the organization the bill was read with: an
    /// answer read with keys since changed is dropped.
    pub given: Vec<GivenKey>,
    /// The bill of the organization, or why it could not be read.
    pub bill: Result<Bill, SourceError>,
    /// When the read ended.
    pub read_at: SystemTime,
}

/// The answer of the read of the numbers of one account with its own key
/// (spec 2026-11-19, Decision 5): its money left, the limit of its key, the
/// bill of its organization or the limits of its login.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedAccountNumbers {
    /// The account the numbers belong to.
    pub account: Account,
    /// The numbers, or why they could not be read.
    pub numbers: Result<AccountNumbers, SourceError>,
    /// When the read ended.
    pub read_at: SystemTime,
}

/// The answer of the read of the list prices of the money estimate (spec
/// Decision 16).
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPrices {
    /// The prices with the source that answered, the LiteLLM price table
    /// or its kept copy, or the failure of each source.
    pub prices: Result<Answer<FetchedPrices>, Vec<SourceFailure>>,
    /// When the read ended.
    pub read_at: SystemTime,
}

/// The answer of the read of the statistics of one session.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedSessionStats {
    /// The profile the session belongs to.
    pub profile: Profile,
    /// The session.
    pub session: SessionId,
    /// The statistics, or why they could not be read.
    pub stats: Result<SessionStats, SourceError>,
}

/// The answer of the read of the statistics of one project folder.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedProjectStats {
    /// The project folder.
    pub folder: PathBuf,
    /// What each profile's files tell about the folder, in the order of
    /// the profiles asked, or why a profile could not be read.
    pub parts: Result<Vec<(Profile, ProjectStats)>, ProjectStatsFailure>,
}

/// Why the statistics of a project folder could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectStatsFailure {
    /// The name of the profile whose files could not be read; `None` when
    /// the read stopped before it answered.
    pub profile_name: Option<String>,
    /// Why.
    pub error: SourceError,
}

/// What a folder field with suggestions asks about the typed path: whether
/// `folder` is a folder on this computer, and which folders in `list_in`
/// have a name that starts with `prefix`, case aside. The answer goes back
/// to the dialog that asked, `asker`.
///
/// # Example
///
/// `~/pro` asks whether `/home/user/pro` is a folder and for the folders
/// of `/home/user` whose name starts with `pro`.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_app_agents::{FolderAsker, FolderQuery};
///
/// let query = FolderQuery {
///     asker: FolderAsker::NewSessionDialog,
///     folder: PathBuf::from("/home/user/pro"),
///     list_in: PathBuf::from("/home/user"),
///     prefix: String::from("pro"),
/// };
///
/// assert_eq!(query.prefix, "pro");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderQuery {
    /// The dialog whose folder field asked; the answer is for it only.
    pub asker: FolderAsker,
    /// The full path typed.
    pub folder: PathBuf,
    /// The folder whose folders may be suggested.
    pub list_in: PathBuf,
    /// The start of the name of a suggested folder; empty for every folder
    /// of `list_in`.
    pub prefix: String,
}

impl FolderQuery {
    /// Whether a folder of `list_in` named `name` is one this query
    /// suggests: its name starts with the prefix, case aside, and a hidden
    /// folder only when the prefix starts with a dot.
    ///
    /// # Example
    ///
    /// ```
    /// use std::path::PathBuf;
    ///
    /// use otty_app_agents::{FolderAsker, FolderQuery};
    ///
    /// let query = FolderQuery {
    ///     asker: FolderAsker::ProfileDialog,
    ///     folder: PathBuf::from("/home/user/.c"),
    ///     list_in: PathBuf::from("/home/user"),
    ///     prefix: String::from(".c"),
    /// };
    ///
    /// assert!(query.suggests(".coder-work"));
    /// assert!(!query.suggests("work"));
    /// ```
    pub fn suggests(&self, name: &str) -> bool {
        let prefix = self.prefix.to_lowercase();
        let name = name.to_lowercase();
        let shows_hidden = prefix.starts_with('.');

        name.starts_with(&prefix) && (shows_hidden || !name.starts_with('.'))
    }
}

/// The dialog whose folder field asked a [`FolderQuery`]. Only one dialog
/// is open at a time; an answer for the other one is dropped.
///
/// # Example
///
/// ```
/// use otty_app_agents::FolderAsker;
///
/// assert_ne!(FolderAsker::NewSessionDialog, FolderAsker::ProfileDialog);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderAsker {
    /// The folder field of the New session dialog.
    NewSessionDialog,
    /// The config folder field of the profile dialog.
    ProfileDialog,
}

/// The answer to a [`FolderQuery`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedFolder {
    /// What was asked.
    pub query: FolderQuery,
    /// Whether the typed path is a folder on this computer.
    pub is_folder: bool,
    /// The folders the typed path may stand for, sorted by name.
    pub suggestions: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn query(prefix: &str) -> FolderQuery {
        FolderQuery {
            asker: FolderAsker::NewSessionDialog,
            folder: PathBuf::from("/home/user").join(prefix),
            list_in: PathBuf::from("/home/user"),
            prefix: String::from(prefix),
        }
    }

    /// Issue 39: the rule of the look-up, also used to narrow the rows
    /// while an answer is on its way: the name starts with the prefix, case
    /// aside, and a hidden folder only when the prefix starts with a dot.
    #[test]
    fn a_query_suggests_the_names_that_start_with_its_prefix() {
        assert!(query("pro").suggests("Projects"));
        assert!(query("").suggests("work"));
        assert!(!query("pro").suggests("public"));
        assert!(!query("").suggests(".config"));
        assert!(query(".c").suggests(".Claude-work"));
    }
}
