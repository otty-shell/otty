//! One shown profile with what was read of it: the accounts a provider
//! part found in it, its sessions, its limits, the tokens it spent and its
//! bill, and how it pays (spec Decisions 2, 5 and 9). The limits and the
//! bill belong to an account: they are read through the first profile the
//! account was found in, and a profile of an account read through another
//! profile takes a copy of that read. The screen links no profile to an
//! account (spec 2026-11-23-accounts-apart-from-profiles, Decision 1):
//! this is only how the reads are shared.

use std::path::PathBuf;
use std::time::SystemTime;

use otty_agents::{
    Account, AccountKind, AgentSession, Bill, BillingKind, Profile,
    ProfileStatus, SourceError, SourceFailure,
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
    accounts: Vec<Account>,
    limits_account: Option<Account>,
    organization: Option<Account>,
    version: Option<String>,
    sessions: Option<Vec<AgentSession>>,
    failures: Option<Vec<SourceFailure>>,
    missing_folders: Vec<PathBuf>,
    session_reads: ReadState,
    limits: ProfileLimits,
    spend: ProfileSpend,
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

    /// The accounts the provider parts found in the profile's folder.
    pub(crate) fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    /// The version of the agent's program the profile last ran, as its
    /// files or the program tell; `None` when neither does.
    pub(crate) fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// The account whose limits or credits the profile shows: its first
    /// login with a subscription the user did not hide. `None` when none
    /// was found: the profile then has no limits to read.
    pub(crate) fn limits_account(&self) -> Option<&Account> {
        self.limits_account.as_ref()
    }

    /// The organization the profile pays through, whose admin key reads
    /// its bill: the first the user did not hide; `None` when none was
    /// found.
    pub(crate) fn organization(&self) -> Option<&Account> {
        self.organization.as_ref()
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

    /// What was read of the real bill of an API key profile.
    pub(crate) fn bill(&self) -> &ProfileBill {
        &self.bill
    }

    /// The profile with its `status` and the `accounts` a provider part
    /// found in it, before anything was read of it.
    pub(crate) fn new(
        profile: Profile,
        status: ProfileStatus,
        accounts: Vec<Account>,
    ) -> Self {
        let mut state = Self {
            profile,
            status,
            accounts,
            limits_account: None,
            organization: None,
            version: None,
            sessions: None,
            failures: None,
            missing_folders: Vec::new(),
            session_reads: ReadState::default(),
            limits: ProfileLimits::default(),
            spend: ProfileSpend::default(),
            bill: ProfileBill::default(),
        };
        state.choose_shown_accounts(|_| false);

        state
    }

    /// Picks the account whose limits the profile shows and the
    /// organization it pays through among the accounts `is_hidden` does
    /// not say the user hid: the first of each.
    pub(crate) fn choose_shown_accounts(
        &mut self,
        is_hidden: impl Fn(&Account) -> bool,
    ) {
        let mut shown =
            self.accounts.iter().filter(|account| !is_hidden(account));
        self.limits_account = shown
            .clone()
            .find(|account| account.kind() == AccountKind::Subscription)
            .cloned();
        self.organization = shown
            .find(|account| account.kind() == AccountKind::Organization)
            .cloned();
    }

    /// The same profile with the version of its agent's program.
    pub(crate) fn with_version(mut self, version: Option<String>) -> Self {
        self.version = version;
        self
    }

    /// How the account of the profile is paid for: an API key when the
    /// accounts found in it say so, else what its last good limits answer
    /// says; `None` while nothing tells. The agent part says nothing of
    /// it (spec 2026-11-23-accounts-apart-from-profiles, Decision 2).
    fn billing_kind(&self) -> Option<BillingKind> {
        if self.accounts_say_api_key() {
            return Some(BillingKind::ApiKey);
        }

        self.limits
            .last_good()
            .map(|(answer, _)| answer.value().billing().kind())
    }

    /// Whether the limit sources of the agent are asked for the profile:
    /// it is connected and the accounts found in it do not say it pays
    /// with an API key.
    pub(crate) fn reads_limits(&self) -> bool {
        self.status == ProfileStatus::Connected && !self.accounts_say_api_key()
    }

    /// Whether the limits of the profile are those of `account`.
    pub(crate) fn has_limits_of(&self, account: &Account) -> bool {
        self.limits_account()
            .is_some_and(|own| own.is_same_account(account))
    }

    /// Whether the bill of the profile is the one of the organization
    /// `organization`.
    pub(crate) fn pays_through(&self, organization: &Account) -> bool {
        self.organization()
            .is_some_and(|own| own.is_same_account(organization))
    }

    /// Whether the profile pays per token with an API key (spec Decision
    /// 9), as [`Self::billing_kind`] tells.
    pub(crate) fn is_api_key(&self) -> bool {
        self.billing_kind() == Some(BillingKind::ApiKey)
    }

    /// Whether the accounts the provider part found in the profile tell
    /// it pays with an API key: some were found, and none of them is a
    /// subscription. It then has no limits to read.
    pub(crate) fn accounts_say_api_key(&self) -> bool {
        !self.accounts.is_empty()
            && self
                .accounts
                .iter()
                .all(|account| account.kind() != AccountKind::Subscription)
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

    /// Take the limits a read gave at `read_at`.
    pub(crate) fn apply_limits(
        &mut self,
        limits: LimitsRead,
        read_at: SystemTime,
    ) {
        self.limits.apply(limits, read_at);
    }

    /// Take `limits`, read through another profile of the same account.
    pub(crate) fn share_limits(&mut self, limits: &ProfileLimits) {
        self.limits = limits.clone();
    }

    /// Start a count of the tokens; `false` when one is on its way.
    pub(crate) fn start_spend_count(&mut self) -> bool {
        self.spend.start_reading()
    }

    /// The list prices changed: the money of its tokens wants a new count.
    pub(crate) fn spend_prices_changed(&mut self) {
        self.spend.prices_changed();
    }

    /// Take the tokens a count gave at `counted_at`.
    pub(crate) fn apply_spend(
        &mut self,
        spend: SpendCount,
        counted_at: SystemTime,
    ) {
        self.spend.apply(spend, counted_at);
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

    /// Take `bill`, read through another profile of the same organization.
    pub(crate) fn share_bill(&mut self, bill: &ProfileBill) {
        self.bill = bill.clone();
    }

    /// Keep what was read of the same profile before: its sessions, its
    /// limits, its tokens and its bill, and whether its sessions are being
    /// read, so a read on its way is not asked for twice. The bill is kept
    /// only when `keeps_bill`, while the admin key is the same: another key
    /// reads the bill of another organization.
    pub(crate) fn take_reads_of(
        &mut self,
        earlier: ProfileState,
        keeps_bill: bool,
    ) {
        self.sessions = earlier.sessions;
        self.failures = earlier.failures;
        self.missing_folders = earlier.missing_folders;
        self.session_reads = earlier.session_reads;
        self.limits = earlier.limits;
        self.spend = earlier.spend;
        if keeps_bill {
            self.bill = earlier.bill;
        }
    }

    /// Forget what was read of a profile whose config folder is gone.
    pub(crate) fn forget_reads(&mut self) {
        self.sessions = None;
        self.failures = None;
        self.missing_folders = Vec::new();
        self.limits = ProfileLimits::default();
        self.spend = ProfileSpend::default();
        self.bill = ProfileBill::default();
    }
}
