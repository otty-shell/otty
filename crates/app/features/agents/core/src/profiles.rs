//! What Agents mode read of the profiles and the accounts: each shown
//! profile with what was read of it, the projects made from their
//! sessions, the accounts and what was read of them, the list prices of
//! the money at API prices, what `agents.json` held, the user's time zone,
//! and the reads of the profiles themselves (spec Decisions 5, 9, 10 and
//! 16).
//!
//! A profile or an account the user hid is not read (spec 2026-11-19,
//! Decision 7): a hidden profile is kept apart from the shown ones with
//! nothing read of it, and no numbers or bill are asked for a hidden
//! account.
//!
//! The accounts come from the provider parts, each found by one part in
//! the shown profiles of its own plugin and kept by its plugin and id
//! (spec 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8). Two
//! plugins that read one login give two accounts, which OTTY never joins.
//! A profile and an account know nothing of each other (Decision 1): the
//! numbers of each account, and the bill of an organization, are read by
//! the provider part that found it, in the folders it was found in, once
//! per account.

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    Account, AccountKind, AgentId, ApiPrices, FoundAccounts, GivenKey,
    PartNeed, PartNeedAction, Profile, ProfileStatus, Project, Registry,
    SavedProfiles,
};

use super::account_reads::AccountReads;
use super::freshness::{ReadState, Which};
use super::limits::AccountLimits;
use super::prices::ListPrices;
use super::profile_state::ProfileState;
use super::types::{
    LoadedAccountNumbers, LoadedBill, LoadedPrices, LoadedProfile,
    LoadedProfiles, LoadedSessions, LoadedSpend, LoadedVersion,
};

/// The profiles and what was read of them.
pub(crate) struct Profiles {
    shown: Vec<ProfileState>,
    hidden: Vec<Profile>,
    found: FoundAccounts,
    projects: Vec<Project>,
    prices: ListPrices,
    api_prices: Arc<ApiPrices>,
    saved: Option<SavedProfiles>,
    time_zone: Option<TimeZone>,
    reads: ReadState,
    is_reload_wanted: bool,
    sessions_wanted: Vec<Profile>,
    registry: Arc<Registry>,
    versions_asked: Vec<AgentId>,
    program_versions: Vec<(AgentId, String)>,
    account_reads: AccountReads,
}

impl Profiles {
    /// The profiles, in their order.
    pub(crate) fn shown(&self) -> &[ProfileState] {
        &self.shown
    }

    /// The profiles the user hid, in their order. Nothing of them is read.
    pub(crate) fn hidden_profiles(&self) -> &[Profile] {
        &self.hidden
    }

    /// The projects of every profile's sessions, the latest first.
    pub(crate) fn projects(&self) -> &[Project] {
        &self.projects
    }

    /// The list prices the money of every profile is counted at, as their
    /// latest read left them.
    pub(crate) fn prices(&self) -> &ListPrices {
        &self.prices
    }

    /// The list prices the agent parts price the tokens at, with the
    /// registry that names a provider to them (spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 3). The same
    /// value until the prices or the registry change.
    pub(crate) fn api_prices(&self) -> &Arc<ApiPrices> {
        &self.api_prices
    }

    /// What `agents.json` held at the last read; `None` when it could not
    /// be read.
    pub(crate) fn saved(&self) -> Option<&SavedProfiles> {
        self.saved.as_ref()
    }

    /// The user's time zone, as the latest read of the profiles that
    /// could read it told; `None` before that, and the times are then
    /// written in UTC.
    pub(crate) fn time_zone(&self) -> Option<&TimeZone> {
        self.time_zone.as_ref()
    }

    /// Whether the profiles were read at least once.
    pub(crate) fn is_loaded(&self) -> bool {
        self.reads.read_at().is_some()
    }

    /// Whether the profiles are being read.
    pub(crate) fn is_loading(&self) -> bool {
        self.reads.is_reading()
    }

    /// Whether the latest read of the profiles ended less than the period
    /// of the timer before `now`.
    pub(crate) fn are_fresh(&self, now: SystemTime) -> bool {
        !self.reads.is_wanted(Which::Stale(now))
    }

    /// The shown profile `profile`, with what was read of it. A profile
    /// is found by its agent and config folder, so a value from before an
    /// edit of its name finds it too.
    pub(crate) fn get(&self, profile: &Profile) -> Option<&ProfileState> {
        self.shown
            .iter()
            .find(|shown| is_same_profile(shown.profile(), profile))
    }

    /// The shown profile of `agent` named `name`.
    pub(crate) fn named(
        &self,
        agent: &AgentId,
        name: &str,
    ) -> Option<&ProfileState> {
        self.shown.iter().find(|shown| {
            shown.profile().agent() == agent && shown.profile().name() == name
        })
    }

    /// The accounts the provider parts found that the user did not hide,
    /// each once however many profiles it was found in, in the order the
    /// parts gave them.
    pub(crate) fn accounts(&self) -> Vec<Account> {
        let mut accounts = self.all_accounts();

        accounts.retain(|account| !self.is_account_hidden(account));
        accounts
    }

    /// The accounts the provider parts found, hidden ones too, each once,
    /// in the order the parts gave them.
    pub(crate) fn all_accounts(&self) -> Vec<Account> {
        self.found.accounts().to_vec()
    }

    /// What the provider parts asked the user that belongs to no account.
    pub(crate) fn found_needs(&self) -> &[PartNeed] {
        self.found.needs()
    }

    /// What was read of the accounts.
    pub(crate) fn account_reads(&self) -> &AccountReads {
        &self.account_reads
    }

    /// The limits of the subscription `account` as its provider part read
    /// them; `None` before a read, or when no part reads it.
    pub(crate) fn account_limits(
        &self,
        account: &Account,
    ) -> Option<AccountLimits> {
        self.registry.reader_of(account)?;
        self.account_reads.of(account)?.limits()
    }

    /// Whether the user hid `account`, as `agents.json` holds it.
    pub(crate) fn is_account_hidden(&self, account: &Account) -> bool {
        self.saved
            .as_ref()
            .is_some_and(|saved| saved.is_account_hidden(account))
    }

    /// Whether the user hid `profile`, as `agents.json` holds it.
    pub(crate) fn is_profile_hidden(&self, profile: &Profile) -> bool {
        self.saved
            .as_ref()
            .is_some_and(|saved| saved.is_profile_hidden(profile))
    }

    /// The name the user gave `account`, as `agents.json` holds it.
    pub(crate) fn account_name(&self, account: &Account) -> Option<&str> {
        self.saved.as_ref()?.account_name(account)
    }

    /// The hidden profile of `agent` named `name`.
    pub(crate) fn hidden_named(
        &self,
        agent: &AgentId,
        name: &str,
    ) -> Option<&Profile> {
        self.hidden
            .iter()
            .find(|profile| profile.agent() == agent && profile.name() == name)
    }

    /// Every profile, hidden ones too: the names and folders a new or
    /// edited profile is checked against.
    pub(crate) fn all(&self) -> Vec<&Profile> {
        self.shown
            .iter()
            .map(ProfileState::profile)
            .chain(&self.hidden)
            .collect()
    }

    /// The project of `folder`, while it is in the list.
    pub(crate) fn project(&self, folder: &Path) -> Option<&Project> {
        self.projects
            .iter()
            .find(|project| project.folder() == folder)
    }

    /// Whether the folder of a project no longer exists, as the last read
    /// of any profile found.
    pub(crate) fn is_missing(&self, folder: &Path) -> bool {
        self.shown.iter().any(|shown| {
            shown
                .missing_folders()
                .iter()
                .any(|missing| missing == folder)
        })
    }

    /// Whether a session of `profile` can be started: the profile is shown
    /// and its program was found.
    pub(crate) fn can_start(&self, profile: &Profile) -> bool {
        self.get(profile).is_some_and(ProfileState::can_start)
    }

    /// Whether `profile` can log in: nobody is logged in with it and the
    /// need its agent part gives runs a command, its login.
    pub(crate) fn can_log_in(&self, profile: &Profile) -> bool {
        self.get(profile).is_some_and(|shown| {
            matches!(
                shown.status(),
                ProfileStatus::NotLoggedIn(Some(need))
                    if matches!(need.action(), PartNeedAction::Run(_))
            )
        })
    }

    /// The profiles a new session can start with, in their order.
    pub(crate) fn startable(&self) -> Vec<&Profile> {
        self.shown
            .iter()
            .filter(|shown| shown.can_start())
            .map(ProfileState::profile)
            .collect()
    }

    /// The keys the user gave for `account` after needs of plugin parts
    /// asked for them; none while `agents.json` is not read.
    pub(crate) fn given_keys_of(&self, account: &Account) -> Vec<GivenKey> {
        self.saved
            .as_ref()
            .map(|saved| saved.given_keys_of(account).to_vec())
            .unwrap_or_default()
    }

    /// Whether `profile` is still shown: an answer of a read of a profile
    /// no longer shown is dropped.
    pub(crate) fn is_shown(&self, profile: &Profile) -> bool {
        self.get(profile).is_some()
    }

    /// Whether the keys given for `organization` are still `given`: a
    /// bill read with keys since changed is dropped.
    pub(crate) fn has_given_keys(
        &self,
        organization: &Account,
        given: &[GivenKey],
    ) -> bool {
        self.given_keys_of(organization) == given
    }

    /// Nothing read yet; `registry` names the agents and the providers of
    /// the accounts.
    pub(crate) fn new(registry: Arc<Registry>) -> Self {
        Self {
            shown: Vec::new(),
            hidden: Vec::new(),
            found: FoundAccounts::default(),
            projects: Vec::new(),
            prices: ListPrices::default(),
            api_prices: Arc::new(ApiPrices::new(
                Default::default(),
                Arc::clone(&registry),
            )),
            saved: None,
            time_zone: None,
            reads: ReadState::default(),
            is_reload_wanted: false,
            sessions_wanted: Vec::new(),
            registry,
            versions_asked: Vec::new(),
            program_versions: Vec::new(),
            account_reads: AccountReads::default(),
        }
    }

    /// Read with `registry` from now on, as when the installed plugins
    /// change. What was read stays until it is read again.
    pub(crate) fn set_readers(&mut self, registry: Arc<Registry>) {
        self.registry = registry;
        self.renew_api_prices();
    }

    /// Start a read of the profiles; `false` when one is on its way.
    pub(crate) fn start_loading(&mut self) -> bool {
        self.reads.start()
    }

    /// Start a read of the profiles after `agents.json` changed; `false`
    /// when one is on its way: it may have read the file before the change,
    /// so another read follows it.
    pub(crate) fn start_reload(&mut self) -> bool {
        if self.start_loading() {
            return true;
        }

        self.is_reload_wanted = true;
        false
    }

    /// End the read of the profiles that stopped before it answered. The
    /// profiles shown stay as they are, and so does the time of the last
    /// read that answered: profiles never read stay unread and are read
    /// again when the mode opens.
    pub(crate) fn end_stopped_read(&mut self) {
        self.reads.end();
    }

    /// Whether a read must follow the one that just ended, and forget it.
    pub(crate) fn take_reload_wanted(&mut self) -> bool {
        std::mem::take(&mut self.is_reload_wanted)
    }

    /// Take the profiles and the accounts that were read and return the
    /// profiles whose sessions to read. A profile that was shown before,
    /// the same agent with the same config folder, keeps its sessions and
    /// its tokens until they are read again, so the lists do not go empty
    /// during a refresh or after an edit of its name. A profile whose
    /// config folder is gone has nothing to read. An account keeps what was
    /// read of it while the keys given for it are the same. A time zone
    /// that could not be read keeps the one known.
    pub(crate) fn apply_profiles(
        &mut self,
        loaded: LoadedProfiles,
    ) -> Vec<Profile> {
        let mut earlier = std::mem::take(&mut self.shown);
        let earlier_saved = std::mem::replace(&mut self.saved, loaded.saved);
        let mut to_read = Vec::new();

        for LoadedProfile {
            profile,
            status,
            version,
        } in loaded.profiles
        {
            let mut shown =
                ProfileState::new(profile, status).with_version(version);
            let position = earlier.iter().position(|old| {
                is_same_profile(old.profile(), shown.profile())
            });
            if let Some(position) = position {
                shown.take_reads_of(earlier.swap_remove(position));
            }
            if !shown.has_config_dir() {
                shown.forget_reads();
            } else if shown.start_session_read() {
                to_read.push(shown.profile().clone());
            }

            self.shown.push(shown);
        }

        self.hidden = loaded.hidden_profiles;
        self.found = loaded.accounts;
        self.forget_reads_with_other_keys(earlier_saved.as_ref());
        if loaded.time_zone.is_some() {
            self.time_zone = loaded.time_zone;
        }
        self.reads.end_at(loaded.read_at);
        self.sessions_wanted.clear();
        self.rebuild_projects();

        to_read
    }

    /// The version of the agent's program for `shown`: from its session
    /// files, else as the program told it.
    pub(crate) fn version_of<'a>(
        &'a self,
        shown: &'a ProfileState,
    ) -> Option<&'a str> {
        let agent = shown.profile().agent();
        let from_program = self
            .program_versions
            .iter()
            .find(|(asked, _)| asked == agent)
            .map(|(_, version)| version.as_str());

        shown.version().or(from_program)
    }

    /// Start asking the program of `agent` for its version; `false` when
    /// it was asked before in this run.
    pub(crate) fn start_version_read(&mut self, agent: &AgentId) -> bool {
        if self.versions_asked.contains(agent) {
            return false;
        }

        self.versions_asked.push(agent.clone());
        true
    }

    /// Take the version the program of `agent` told, for its profiles with
    /// no version from their sessions, for the rest of the run.
    pub(crate) fn apply_program_version(&mut self, loaded: LoadedVersion) {
        let Some(version) = loaded.version else {
            return;
        };

        self.program_versions
            .retain(|(agent, _)| *agent != loaded.agent);
        self.program_versions.push((loaded.agent, version));
    }

    /// Take `saved`, the new content of `agents.json` the user's choice
    /// made, before it is written, so the choice shows at once. A shown
    /// profile it hides leaves the shown ones with what was read of it, and
    /// its sessions leave the projects; a profile it shows again is read
    /// with the next read of the profiles. An account whose given keys it
    /// changes is read again.
    pub(crate) fn apply_saved(&mut self, saved: SavedProfiles) {
        let (hidden, shown): (Vec<_>, Vec<_>) = std::mem::take(&mut self.shown)
            .into_iter()
            .partition(|shown| saved.is_profile_hidden(shown.profile()));

        self.shown = shown;
        self.hidden
            .extend(hidden.into_iter().map(|state| state.profile().clone()));
        let earlier_saved = self.saved.replace(saved);
        self.forget_reads_with_other_keys(earlier_saved.as_ref());
        self.rebuild_projects();
    }

    /// Remember in what `agents.json` holds that the Found by OTTY report
    /// was shown, and return the new content to save; `None` when the file
    /// could not be read, so it is not written over.
    pub(crate) fn mark_found_report_shown(&mut self) -> Option<SavedProfiles> {
        let saved = self.saved.take()?.with_found_report_shown();

        self.saved = Some(saved.clone());
        Some(saved)
    }

    /// Read the sessions of `profile` again the next time Agents mode
    /// opens: a session it started may have made a new project.
    pub(crate) fn read_sessions_later(&mut self, profile: Profile) {
        if !self.sessions_wanted.contains(&profile) {
            self.sessions_wanted.push(profile);
        }
    }

    /// Start a read of the sessions of each profile
    /// [`Self::read_sessions_later`] asked for that is shown, whose config
    /// folder is there and that has no read on its way, and return those
    /// profiles.
    pub(crate) fn start_wanted_session_reads(&mut self) -> Vec<Profile> {
        let wanted = std::mem::take(&mut self.sessions_wanted);

        self.shown
            .iter_mut()
            .filter(|shown| {
                let is_wanted = wanted
                    .iter()
                    .any(|profile| is_same_profile(profile, shown.profile()));
                is_wanted && shown.has_config_dir()
            })
            .filter_map(|shown| {
                shown.start_session_read().then(|| shown.profile().clone())
            })
            .collect()
    }

    /// Take the sessions of one shown profile.
    pub(crate) fn apply_sessions(&mut self, loaded: LoadedSessions) {
        let Some(shown) = shown_mut(&mut self.shown, &loaded.profile) else {
            return;
        };

        shown.apply_sessions(loaded.sessions, loaded.missing_folders);
        self.rebuild_projects();
    }

    /// Start a count of the tokens of each shown profile whose config
    /// folder exists, that `which` picks and that has none on its way, and
    /// return those profiles. While the first read of the list prices is
    /// on its way the counts wait for it and start once it answers, so a
    /// start reads the session files once (issue 11 of spec
    /// 2026-11-23-accounts-apart-from-profiles).
    pub(crate) fn start_spend_reads(&mut self, which: Which) -> Vec<Profile> {
        let waits_for_prices = self.prices.is_first_read_on_its_way();
        let mut started = Vec::new();

        for shown in &mut self.shown {
            let spend = shown.spend();
            let is_wanted = shown.has_config_dir()
                && !spend.reads().is_reading()
                && spend.reads().is_wanted(which);
            if !is_wanted {
                continue;
            }

            if waits_for_prices {
                shown.spend_waits_for_prices();
            } else if shown.start_spend_count() {
                started.push(shown.profile().clone());
            }
        }

        started
    }

    /// Take the tokens of one shown profile.
    pub(crate) fn apply_spend(&mut self, loaded: LoadedSpend) {
        let Some(shown) = shown_mut(&mut self.shown, &loaded.profile) else {
            return;
        };

        shown.apply_spend(loaded.spend, loaded.counted_at);
    }

    /// Start a read of the list prices when a profile is shown, since the
    /// money of every profile is its tokens at API prices, on a
    /// subscription too (spec 2026-11-23-accounts-apart-from-profiles,
    /// Decision 3), the prices are wanted for `which` at `now` and no read
    /// is on its way (spec Decision 16). Returns whether the read must
    /// fetch the price table again even within the day; `None` when no
    /// read starts.
    pub(crate) fn start_price_read(
        &mut self,
        which: Which,
        now: SystemTime,
    ) -> Option<bool> {
        if self.shown.is_empty() || !self.prices.is_wanted(which, now) {
            return None;
        }

        let fetch_again = self.prices.fetch_again();
        self.prices.start_reading().then_some(fetch_again)
    }

    /// Take the list prices that were read: the money of the profiles is
    /// counted at them when it is shown.
    pub(crate) fn apply_prices(&mut self, loaded: LoadedPrices) {
        self.prices.apply(loaded.prices, loaded.read_at);
        self.renew_api_prices();
    }

    /// Start a count of the tokens of each shown profile that waits for
    /// the list prices, unless a count is on its way: that one starts once
    /// it ends. Nothing starts while the first read of the prices is on
    /// its way. Returns those profiles.
    pub(crate) fn start_counts_at_new_prices(&mut self) -> Vec<Profile> {
        if self.prices.is_first_read_on_its_way() {
            return Vec::new();
        }

        self.shown
            .iter_mut()
            .filter(|shown| {
                shown.has_config_dir() && shown.spend().waits_for_prices()
            })
            .filter_map(|shown| {
                shown.start_spend_count().then(|| shown.profile().clone())
            })
            .collect()
    }

    /// The prices the agent parts price at, from the latest prices read
    /// and the registry. When they changed, every shown profile wants a
    /// new count of its tokens, whose money follows the new prices, and
    /// every account paid per token a new read of its numbers, whose part
    /// prices its sessions at them (issue 22).
    fn renew_api_prices(&mut self) {
        let table = self.prices.table().cloned().unwrap_or_default();
        let renewed = ApiPrices::new(table, Arc::clone(&self.registry));
        if *self.api_prices == renewed {
            return;
        }

        self.api_prices = Arc::new(renewed);
        for shown in &mut self.shown {
            shown.spend_waits_for_prices();
        }
        self.account_reads.wait_for_prices();
    }

    /// Start a read of the numbers of each account paid per token that
    /// waits for new list prices and has none on its way, and return
    /// those accounts with the keys the user gave for them. Nothing starts
    /// while the first read of the prices is on its way. The read may
    /// start the agent's program when `may_start_programs`.
    pub(crate) fn start_account_reads_at_new_prices(
        &mut self,
        may_start_programs: bool,
    ) -> Vec<(Account, Vec<GivenKey>)> {
        if self.prices.is_first_read_on_its_way() {
            return Vec::new();
        }

        self.account_reads
            .start_at_new_prices(may_start_programs)
            .into_iter()
            .map(|account| {
                let given = self.given_keys_of(&account);
                (account, given)
            })
            .collect()
    }

    /// Start a read of the numbers of each listed account a provider part
    /// reads that `which` picks and that has none on its way, and return
    /// those accounts with the keys the user gave for them. The read may
    /// start the agent's program when `may_start_programs`, while the mode
    /// is on screen. A hidden account is not read.
    pub(crate) fn start_account_reads(
        &mut self,
        which: Which,
        may_start_programs: bool,
    ) -> Vec<(Account, Vec<GivenKey>)> {
        let wanted = self.read_accounts();

        self.account_reads
            .start(wanted, which, may_start_programs)
            .into_iter()
            .map(|account| {
                let given = self.given_keys_of(&account);
                (account, given)
            })
            .collect()
    }

    /// Start a read of the bill of each listed organization a provider
    /// part reads that `which` picks and that has none on its way, and
    /// return those organizations with the keys the user gave for them
    /// (spec Decision 9). Whether an account is an organization with a
    /// bill is its provider part's call. A hidden organization is not
    /// read.
    pub(crate) fn start_bill_reads(
        &mut self,
        which: Which,
    ) -> Vec<(Account, Vec<GivenKey>)> {
        let organizations = self
            .read_accounts()
            .into_iter()
            .filter(|account| account.kind() == AccountKind::Organization)
            .collect();

        self.account_reads
            .start_bills(organizations, which)
            .into_iter()
            .map(|organization| {
                let given = self.given_keys_of(&organization);
                (organization, given)
            })
            .collect()
    }

    /// Forget what was read of `account`, so it is read again at once.
    pub(crate) fn forget_account_read(&mut self, account: &Account) {
        self.account_reads.forget(account);
    }

    /// Take the numbers of one account.
    pub(crate) fn apply_account_numbers(
        &mut self,
        loaded: LoadedAccountNumbers,
    ) {
        self.account_reads.apply(loaded);
    }

    /// Take the bill of one organization.
    pub(crate) fn apply_bill(&mut self, loaded: LoadedBill) {
        self.account_reads.apply_bill(loaded);
    }

    /// Forget what was read of each account whose given keys are not the
    /// ones `earlier`, the content of `agents.json` before, held: another
    /// key reads other numbers, or the bill of another organization.
    fn forget_reads_with_other_keys(
        &mut self,
        earlier: Option<&SavedProfiles>,
    ) {
        let changed: Vec<Account> = self
            .found
            .accounts()
            .iter()
            .filter(|account| {
                let before = earlier
                    .map(|saved| saved.given_keys_of(account))
                    .unwrap_or_default();
                before != self.given_keys_of(account).as_slice()
            })
            .cloned()
            .collect();

        for account in &changed {
            self.account_reads.forget(account);
        }
    }

    /// The accounts not hidden whose numbers OTTY reads: each one a part
    /// in the registry found.
    fn read_accounts(&self) -> Vec<Account> {
        self.accounts()
            .into_iter()
            .filter(|account| self.registry.reader_of(account).is_some())
            .collect()
    }

    /// Make the projects again from the sessions of every profile.
    fn rebuild_projects(&mut self) {
        let read = self.shown.iter().filter_map(|shown| {
            let sessions = shown.sessions()?;
            Some((shown.profile(), sessions))
        });

        self.projects = otty_agents::projects(read);
    }
}

/// The profile `profile` among `shown`, to take what was read of it.
fn shown_mut<'a>(
    shown: &'a mut [ProfileState],
    profile: &Profile,
) -> Option<&'a mut ProfileState> {
    shown
        .iter_mut()
        .find(|shown| is_same_profile(shown.profile(), profile))
}

/// Whether `a` and `b` are the same profile: the same agent with the same
/// config folder, which no two profiles of an agent share. Its name may
/// differ after an edit.
fn is_same_profile(a: &Profile, b: &Profile) -> bool {
    a.agent() == b.agent() && a.config_dir() == b.config_dir()
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;

    use crate::testing::{
        chatgpt_login, chatgpt_login_in, claude, codex, codex_work, connected,
        organization, with_accounts,
    };

    /// Spec Decision 2: one account, one row, however many profiles it
    /// was found in.
    #[test]
    fn an_account_of_two_profiles_is_listed_once_in_the_order_found() {
        let state = with_accounts(
            &[
                (claude(), connected(), vec![organization()]),
                (
                    codex(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex")],
                ),
                (
                    codex_work(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex-work")],
                ),
            ],
            SavedProfiles::default(),
        );

        let accounts = state.profiles().accounts();

        assert_eq!(accounts.len(), 2);
        assert!(accounts[0].is_same_account(&organization()));
        assert!(accounts[1].is_same_account(&chatgpt_login()));
        assert_eq!(accounts[1].found_in().len(), 2);
    }
}
