//! One shown profile with what was read of it: its sessions and the tokens
//! it spent. Nothing of it depends on an account: no account is found in
//! a profile as far as the profile knows, and the numbers and bills of the
//! accounts are read by themselves (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 1).

use std::path::PathBuf;
use std::time::SystemTime;

use otty_agents::{AgentSession, Profile, ProfileStatus, SourceFailure};

use super::freshness::ReadState;
use super::spend::ProfileSpend;
use super::types::{SessionsRead, SpendCount};

/// One shown profile with what was read of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileState {
    profile: Profile,
    status: ProfileStatus,
    version: Option<String>,
    sessions: Option<Vec<AgentSession>>,
    failures: Option<Vec<SourceFailure>>,
    missing_folders: Vec<PathBuf>,
    session_reads: ReadState,
    spend: ProfileSpend,
}

impl ProfileState {
    /// The profile.
    pub(crate) fn profile(&self) -> &Profile {
        &self.profile
    }

    /// Whether the profile is installed and logged in.
    pub(crate) fn status(&self) -> &ProfileStatus {
        &self.status
    }

    /// The version of the agent's program the profile last ran, as its
    /// files or the program tell; `None` when neither does.
    pub(crate) fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// The sessions of the profile, once they were read.
    pub(crate) fn sessions(&self) -> Option<&[AgentSession]> {
        self.sessions.as_deref()
    }

    /// Why the last read of the sessions failed, when it did.
    pub(crate) fn failures(&self) -> Option<&[SourceFailure]> {
        self.failures.as_deref()
    }

    /// The project folders of its sessions that no longer exist, as the
    /// last read found.
    pub(crate) fn missing_folders(&self) -> &[PathBuf] {
        &self.missing_folders
    }

    /// Whether its sessions are being read.
    pub(crate) fn is_reading_sessions(&self) -> bool {
        self.session_reads.is_reading()
    }

    /// What was counted of the tokens spent.
    pub(crate) fn spend(&self) -> &ProfileSpend {
        &self.spend
    }

    /// The profile with its `status`, before anything was read of it.
    pub(crate) fn new(profile: Profile, status: ProfileStatus) -> Self {
        Self {
            profile,
            status,
            version: None,
            sessions: None,
            failures: None,
            missing_folders: Vec::new(),
            session_reads: ReadState::default(),
            spend: ProfileSpend::default(),
        }
    }

    /// The same profile with the version of its agent's program.
    pub(crate) fn with_version(mut self, version: Option<String>) -> Self {
        self.version = version;
        self
    }

    /// Whether a session of the profile can be started: its program was
    /// found.
    pub(crate) fn can_start(&self) -> bool {
        self.status != ProfileStatus::ProgramNotFound
    }

    /// Whether its config folder is there, so it has something to read.
    pub(crate) fn has_config_dir(&self) -> bool {
        self.status != ProfileStatus::ConfigDirNotFound
    }

    /// Start a read of the sessions; `false` when one is on its way.
    pub(crate) fn start_session_read(&mut self) -> bool {
        self.session_reads.start()
    }

    /// Take the sessions that were read, with the folders of them that no
    /// longer exist. A failure keeps the sessions read before: the last
    /// good answer stays on screen. An agent that can not list sessions
    /// has none.
    pub(crate) fn apply_sessions(
        &mut self,
        sessions: SessionsRead,
        missing_folders: Vec<PathBuf>,
    ) {
        self.session_reads.end();

        match sessions {
            SessionsRead::Read(sessions) => {
                self.sessions = Some(sessions);
                self.failures = None;
                self.missing_folders = missing_folders;
            },
            SessionsRead::NoWay => {
                self.sessions = Some(Vec::new());
                self.failures = None;
                self.missing_folders = Vec::new();
            },
            SessionsRead::Failed(failures) => self.failures = Some(failures),
        }
    }

    /// Start a count of the tokens; `false` when one is on its way.
    pub(crate) fn start_spend_count(&mut self) -> bool {
        self.spend.start_reading()
    }

    /// A count of the tokens is wanted once the list prices are read: they
    /// changed, or they are being read for the first time.
    pub(crate) fn spend_waits_for_prices(&mut self) {
        self.spend.wait_for_prices();
    }

    /// Take the tokens a count gave at `counted_at`.
    pub(crate) fn apply_spend(
        &mut self,
        spend: SpendCount,
        counted_at: SystemTime,
    ) {
        self.spend.apply(spend, counted_at);
    }

    /// Keep what was read of the same profile before: its sessions and its
    /// tokens, and whether its sessions are being read, so a read on its
    /// way is not asked for twice.
    pub(crate) fn take_reads_of(&mut self, earlier: ProfileState) {
        self.sessions = earlier.sessions;
        self.failures = earlier.failures;
        self.missing_folders = earlier.missing_folders;
        self.session_reads = earlier.session_reads;
        self.spend = earlier.spend;
    }

    /// Forget what was read of a profile whose config folder is gone.
    pub(crate) fn forget_reads(&mut self) {
        self.sessions = None;
        self.failures = None;
        self.missing_folders = Vec::new();
        self.spend = ProfileSpend::default();
    }
}
