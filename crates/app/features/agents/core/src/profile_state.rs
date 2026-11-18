//! One shown profile with what was read of it: its sessions, its limits,
//! the tokens it spent, the money estimated from them and its bill, and
//! how it pays (spec Decisions 5 and 9).

use std::path::PathBuf;
use std::time::SystemTime;

use otty_agents::{
    AgentSession, Bill, BillingKind, MoneyEstimate, PriceTable, Profile,
    ProfileStatus, SourceError, SourceFailure, SourceId,
};

use super::bill::ProfileBill;
use super::freshness::ReadState;
use super::limits::ProfileLimits;
use super::spend::ProfileSpend;
use super::types::{LimitsRead, SessionsRead, SpendCount};

/// One shown profile with what was read of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileState {
    profile: Profile,
    status: ProfileStatus,
    sessions: Option<Vec<AgentSession>>,
    failures: Option<Vec<SourceFailure>>,
    missing_folders: Vec<PathBuf>,
    session_reads: ReadState,
    limits: ProfileLimits,
    spend: ProfileSpend,
    money: Option<MoneyEstimate>,
    bill: ProfileBill,
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

    /// What was read of the limits.
    pub(crate) fn limits(&self) -> &ProfileLimits {
        &self.limits
    }

    /// What was counted of the tokens spent.
    pub(crate) fn spend(&self) -> &ProfileSpend {
        &self.spend
    }

    /// The money an API key profile spent, estimated from the tokens
    /// counted; `None` for another billing or before a count.
    pub(crate) fn money(&self) -> Option<&MoneyEstimate> {
        self.money.as_ref()
    }

    /// What was read of the real bill of an API key profile.
    pub(crate) fn bill(&self) -> &ProfileBill {
        &self.bill
    }

    /// The profile with its `status`, before anything was read of it.
    pub(crate) fn new(profile: Profile, status: ProfileStatus) -> Self {
        Self {
            profile,
            status,
            sessions: None,
            failures: None,
            missing_folders: Vec::new(),
            session_reads: ReadState::default(),
            limits: ProfileLimits::default(),
            spend: ProfileSpend::default(),
            money: None,
            bill: ProfileBill::default(),
        }
    }

    /// How the account of the profile is paid for: an API key when its
    /// files tell so, else what its last good limits answer says, else
    /// what its files tell; `None` while nothing tells.
    fn billing_kind(&self) -> Option<BillingKind> {
        if self.files_say_api_key() {
            return Some(BillingKind::ApiKey);
        }

        let from_limits = self
            .limits
            .last_good()
            .map(|(answer, _)| answer.value().billing().kind());
        let from_files = match &self.status {
            ProfileStatus::Connected { billing, .. } => *billing,
            _ => None,
        };

        from_limits.or(from_files)
    }

    /// Whether the profile pays per token with an API key (spec Decision
    /// 9), as [`Self::billing_kind`] tells.
    pub(crate) fn is_api_key(&self) -> bool {
        self.billing_kind() == Some(BillingKind::ApiKey)
    }

    /// Whether the files of the profile tell it pays with an API key: it
    /// then has no limits to read.
    pub(crate) fn files_say_api_key(&self) -> bool {
        matches!(
            self.status,
            ProfileStatus::Connected {
                billing: Some(BillingKind::ApiKey),
                ..
            }
        )
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

    /// Start a read of the limits, which may start the agent's program
    /// when `may_start_programs`; `false` when one is on its way.
    pub(crate) fn start_limit_read(
        &mut self,
        may_start_programs: bool,
    ) -> bool {
        self.limits.start_reading(may_start_programs)
    }

    /// Take the limits a read over `sources` gave at `read_at`, and
    /// estimate the money at `prices` again: the answer may tell the
    /// profile pays with an API key.
    pub(crate) fn apply_limits(
        &mut self,
        limits: LimitsRead,
        sources: Vec<SourceId>,
        read_at: SystemTime,
        prices: Option<&PriceTable>,
    ) {
        self.limits.apply(limits, sources, read_at);
        self.estimate_money(prices);
    }

    /// Start a count of the tokens; `false` when one is on its way.
    pub(crate) fn start_spend_count(&mut self) -> bool {
        self.spend.start_reading()
    }

    /// Take the tokens a count gave at `counted_at`, and estimate the
    /// money at `prices` again.
    pub(crate) fn apply_spend(
        &mut self,
        spend: SpendCount,
        counted_at: SystemTime,
        prices: Option<&PriceTable>,
    ) {
        self.spend.apply(spend, counted_at);
        self.estimate_money(prices);
    }

    /// Start a read of the bill; `false` when one is on its way.
    pub(crate) fn start_bill_read(&mut self) -> bool {
        self.bill.start_reading()
    }

    /// Take the bill a read gave at `read_at`.
    pub(crate) fn apply_bill(
        &mut self,
        bill: Result<Bill, SourceError>,
        read_at: SystemTime,
    ) {
        self.bill.apply(bill, read_at);
    }

    /// Estimate the money of an API key profile from its last good count
    /// of tokens at `prices` (spec Decision 9); another profile, and any
    /// profile while there are no prices, has none.
    pub(crate) fn estimate_money(&mut self, prices: Option<&PriceTable>) {
        let is_api_key = self.is_api_key();

        self.money = self
            .spend
            .last_good()
            .filter(|_| is_api_key)
            .zip(prices)
            .map(|((spend, _), prices)| MoneyEstimate::new(spend, prices));
    }

    /// Keep what was read of the same profile before: its sessions, its
    /// limits, its tokens and its bill, and whether its sessions are being
    /// read, so a read on its way is not asked for twice.
    pub(crate) fn take_reads_of(&mut self, earlier: ProfileState) {
        self.sessions = earlier.sessions;
        self.failures = earlier.failures;
        self.missing_folders = earlier.missing_folders;
        self.session_reads = earlier.session_reads;
        self.limits = earlier.limits;
        self.spend = earlier.spend;
        self.bill = earlier.bill;
    }

    /// Forget what was read of a profile whose config folder is gone.
    pub(crate) fn forget_reads(&mut self) {
        self.sessions = None;
        self.failures = None;
        self.missing_folders = Vec::new();
        self.limits = ProfileLimits::default();
        self.spend = ProfileSpend::default();
        self.money = None;
        self.bill = ProfileBill::default();
    }
}
