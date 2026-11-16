use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use otty_agents::{
    Agent, AgentSession, Answer, Limits, Profile, ProfileStatus, SavedProfiles,
    SourceError, SourceFailure, Spend,
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
    /// The names of every source of the agent, best first, those after
    /// the one that answered too: the dashboard lists them with their
    /// states.
    pub sources: Vec<&'static str>,
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
