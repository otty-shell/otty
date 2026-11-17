use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use otty_agents::{
    Agent, AgentSession, Answer, Bill, Limits, Profile, ProfileStatus,
    ProjectStats, SavedProfiles, SessionId, SessionStats, SourceError,
    SourceFailure, SourceId, Spend,
};

/// The agents OTTY knows, shared by the feature and the reads that run on
/// a background thread.
///
/// The app makes it once from [`otty_agents::known_agents`] and keeps it
/// for the whole run: an agent may remember what it read between two
/// refreshes, such as the Codex session files.
pub type AgentList = Arc<[Box<dyn Agent>]>;

/// The answer of the profiles read: what `otty-app-agents-io` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProfiles {
    /// Every profile to show, hidden ones too, in the order to show them.
    pub profiles: Vec<LoadedProfile>,
    /// What `agents.json` holds; `None` when it could not be read, so the
    /// feature does not write over it.
    pub saved: Option<SavedProfiles>,
    /// Why `agents.json` could not be read, ready to show; the profiles
    /// are then the defaults.
    pub problem: Option<String>,
    /// The user's offset from UTC in seconds, east of UTC positive, read
    /// with the profiles, before any limits; `None` when it could not be
    /// read. The dashboard writes its times with it.
    pub utc_offset_seconds: Option<i32>,
    /// When the read ended. Opening Agents mode reads the profiles again
    /// only when they are older than the period of its timer.
    pub read_at: SystemTime,
}

/// One profile and whether it can be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProfile {
    /// The profile.
    pub profile: Profile,
    /// Whether it is installed and logged in.
    pub status: ProfileStatus,
}

/// The answer of the sessions read of one profile.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedSessions {
    /// The profile the sessions belong to.
    pub profile: Profile,
    /// The sessions, or the failure of each way to read them. No failures
    /// at all means the agent can not list sessions.
    pub sessions: Result<Vec<AgentSession>, Vec<SourceFailure>>,
    /// The project folders of these sessions that no longer exist.
    pub missing_folders: Vec<PathBuf>,
}

/// The answer of the limits read of one profile.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedLimits {
    /// The profile the limits belong to.
    pub profile: Profile,
    /// The limits with the source that answered, or the failure of each
    /// source. No failures at all means the agent can not read limits.
    pub limits: Result<Answer<Limits>, Vec<SourceFailure>>,
    /// The ids of every source of the agent, best first, those after the
    /// one that answered too: the dashboard lists them with their states.
    pub sources: Vec<SourceId>,
    /// When the read ended.
    pub read_at: SystemTime,
}

/// The answer of the count of the tokens one profile spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSpend {
    /// The profile the tokens belong to.
    pub profile: Profile,
    /// The tokens, or why they could not be counted. `None` means the
    /// agent can not count tokens.
    pub spend: Option<Result<Spend, SourceError>>,
    /// When the count ended.
    pub counted_at: SystemTime,
}

/// The answer of the read of the bill of one API key profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedBill {
    /// The profile the bill belongs to.
    pub profile: Profile,
    /// The bill of its organization, or why it could not be read.
    pub bill: Result<Bill, SourceError>,
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
    /// The statistics, or why they could not be read, ready to show.
    pub stats: Result<SessionStats, String>,
}

/// The answer of the read of the statistics of one project folder.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedProjectStats {
    /// The project folder.
    pub folder: PathBuf,
    /// What each profile's files tell about the folder, in the order of
    /// the profiles asked, or why a profile could not be read, ready to
    /// show.
    pub parts: Result<Vec<(Profile, ProjectStats)>, String>,
}

/// What the folder field of the New session dialog asks about the typed
/// path: whether `folder` is a folder on this computer, and which folders
/// in `list_in` have a name that starts with `prefix`, case aside.
///
/// # Example
///
/// `~/pro` asks whether `/home/user/pro` is a folder and for the folders
/// of `/home/user` whose name starts with `pro`.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_app_agents::FolderQuery;
///
/// let query = FolderQuery {
///     folder: PathBuf::from("/home/user/pro"),
///     list_in: PathBuf::from("/home/user"),
///     prefix: String::from("pro"),
/// };
///
/// assert_eq!(query.prefix, "pro");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderQuery {
    /// The full path typed.
    pub folder: PathBuf,
    /// The folder whose folders may be suggested.
    pub list_in: PathBuf,
    /// The start of the name of a suggested folder; empty for every folder
    /// of `list_in`.
    pub prefix: String,
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
