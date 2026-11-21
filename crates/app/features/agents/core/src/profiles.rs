//! What Agents mode read of the profiles: each shown profile with what was
//! read of it, the projects made from their sessions, the list prices of
//! the money estimate, what `agents.json` held, the user's time zone, and
//! the reads of the profiles themselves (spec Decisions 5, 9, 10 and 16).
//!
//! The limits and the bill belong to an account, so they are read once per
//! account (spec 2026-11-19, Decision 5): through the first profile of the
//! account that wants a read, and the answer is copied to every other
//! shown profile of the account.
//!
//! A profile or an account the user hid is not read (spec 2026-11-19,
//! Decision 7): a hidden profile is kept apart from the shown ones with
//! nothing read of it, and no limits or bill are asked for a hidden
//! account.

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    Account, AccountKind, AgentId, GivenKey, KeyPlace, PartNeedAction, Profile,
    ProfileStatus, Project, Registry, SavedProfiles, merge_accounts,
    own_reader_of,
};

use super::freshness::{ReadState, Which};
use super::limits::ProfileLimits;
use super::own_reads::OwnReads;
use super::prices::ListPrices;
use super::profile_state::ProfileState;
use super::types::{
    LoadedAccountNumbers, LoadedBill, LoadedLimits, LoadedPrices,
    LoadedProfile, LoadedProfiles, LoadedSessions, LoadedSpend, LoadedVersion,
};

/// The profiles and what was read of them.
pub(crate) struct Profiles {
    shown: Vec<ProfileState>,
    hidden: Vec<Profile>,
    projects: Vec<Project>,
    prices: ListPrices,
    saved: Option<SavedProfiles>,
    time_zone: Option<TimeZone>,
    reads: ReadState,
    is_reload_wanted: bool,
    sessions_wanted: Vec<Profile>,
    registry: Arc<Registry>,
    versions_asked: Vec<AgentId>,
    program_versions: Vec<(AgentId, String)>,
    own_reads: OwnReads,
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

    /// The list prices the money of an API key profile is estimated at,
    /// as their latest read left them.
    pub(crate) fn prices(&self) -> &ListPrices {
        &self.prices
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

    /// Whether a read of the limits is on its way for any profile.
    pub(crate) fn is_reading_limits(&self) -> bool {
        self.shown
            .iter()
            .any(|state| state.limits().reads().is_reading())
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

    /// The accounts found in the shown profiles that the user did not
    /// hide, each once however many profiles it was found in (spec
    /// Decision 2), in the order of the profiles.
    pub(crate) fn accounts(&self) -> Vec<Account> {
        let mut accounts = self.all_accounts();

        accounts.retain(|account| !self.is_account_hidden(account));
        accounts
    }

    /// The accounts found in the shown profiles, hidden ones too, each
    /// once, in the order of the profiles, then the accounts the user
    /// added (spec 2026-11-19, Decision 7).
    pub(crate) fn all_accounts(&self) -> Vec<Account> {
        let added = self
            .saved
            .as_ref()
            .map(SavedProfiles::added_accounts)
            .unwrap_or_default();
        let found = self
            .shown
            .iter()
            .flat_map(|shown| shown.accounts().iter().cloned())
            .chain(added)
            .collect();

        merge_accounts(found, &self.registry)
    }

    /// What was read of the accounts that tell their own numbers.
    pub(crate) fn own_reads(&self) -> &OwnReads {
        &self.own_reads
    }

    /// The limits of `account` as they were read: through a shown profile
    /// of it, else with its own login, for a subscription login no profile
    /// reads the limits of (spec 2026-11-19, Decision 5).
    pub(crate) fn account_limits(
        &self,
        account: &Account,
    ) -> Option<ProfileLimits> {
        if let Some(shown) = self.with_limits_of(account) {
            return Some(shown.limits().clone());
        }

        own_reader_of(account, &self.registry)?;
        self.own_reads.of(account)?.limits()
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

    /// The first shown profile `account` was found in: the profile its
    /// limits or its bill are read through.
    pub(crate) fn first_with(
        &self,
        account: &Account,
    ) -> Option<&ProfileState> {
        self.found_with(account).next()
    }

    /// The shown profiles `account` was found in, in their order: the
    /// profiles that use it.
    pub(crate) fn found_with<'a>(
        &'a self,
        account: &Account,
    ) -> impl Iterator<Item = &'a ProfileState> {
        self.shown.iter().filter(move |shown| {
            shown
                .accounts()
                .iter()
                .any(|found| found.is_same_account(account))
        })
    }

    /// The first shown profile whose limits are those of `account`: the
    /// limits of the account as they were read, through it or copied to
    /// it.
    pub(crate) fn with_limits_of(
        &self,
        account: &Account,
    ) -> Option<&ProfileState> {
        self.shown.iter().find(|shown| shown.has_limits_of(account))
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

    /// Whether the shown profile `profile` pays per token with an API key.
    pub(crate) fn is_api_key(&self, profile: &Profile) -> bool {
        self.get(profile).is_some_and(ProfileState::is_api_key)
    }

    /// The organization whose bill is read through `shown`, an API key
    /// profile: the one it pays through, when a provider part serves it
    /// (spec Decision 9; issue 20 of spec 2026-11-19-agent-plugins).
    /// Which key reads the bill is the part's business.
    pub(crate) fn bill_organization<'a>(
        &self,
        shown: &'a ProfileState,
    ) -> Option<&'a Account> {
        let organization = shown.organization()?;

        self.registry
            .provider_of(organization.provider())
            .map(|_| organization)
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

    /// Whether `profile` is still shown and the keys given for its
    /// organization are still `given`: a bill read with keys since changed
    /// is dropped.
    pub(crate) fn is_shown_with_given_keys(
        &self,
        profile: &Profile,
        given: &[GivenKey],
    ) -> bool {
        self.get(profile).is_some_and(|shown| {
            given_keys_in(self.saved.as_ref(), shown) == given
        })
    }

    /// Nothing read yet; `registry` names the agents and the providers of
    /// the accounts.
    pub(crate) fn new(registry: Arc<Registry>) -> Self {
        Self {
            shown: Vec::new(),
            hidden: Vec::new(),
            projects: Vec::new(),
            prices: ListPrices::default(),
            saved: None,
            time_zone: None,
            reads: ReadState::default(),
            is_reload_wanted: false,
            sessions_wanted: Vec::new(),
            registry,
            versions_asked: Vec::new(),
            program_versions: Vec::new(),
            own_reads: OwnReads::default(),
        }
    }

    /// Read with `registry` from now on, as when the installed plugins
    /// change. What was read stays until it is read again.
    pub(crate) fn set_readers(&mut self, registry: Arc<Registry>) {
        self.registry = registry;
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

    /// Take the profiles that were read and return those whose sessions
    /// to read. A profile that was shown before, the same agent with the
    /// same config folder, keeps its sessions and its limits until they
    /// are read again, so the lists do not go empty during a refresh or
    /// after an edit of its name; it keeps its bill while the keys given
    /// for its organization are the same. A profile whose config folder is gone
    /// has nothing to read. A time zone that could not be read keeps the
    /// one known. The provider of each account found by the host of its
    /// key is named as the registry names it (spec
    /// 2026-11-19-agent-plugins, "Identity and links do not depend on a
    /// provider plugin").
    pub(crate) fn apply_profiles(
        &mut self,
        loaded: LoadedProfiles,
    ) -> Vec<Profile> {
        let saved = loaded
            .saved
            .map(|saved| saved.with_providers_resolved(&self.registry));
        let mut earlier = std::mem::take(&mut self.shown);
        let earlier_saved = std::mem::replace(&mut self.saved, saved);
        let mut to_read = Vec::new();

        for LoadedProfile {
            profile,
            status,
            accounts,
            version,
        } in loaded.profiles
        {
            let accounts = merge_accounts(accounts, &self.registry);
            let mut shown = ProfileState::new(profile, status, accounts)
                .with_version(version);
            let position = earlier.iter().position(|old| {
                is_same_profile(old.profile(), shown.profile())
            });
            if let Some(position) = position {
                let old = earlier.swap_remove(position);
                let keeps_bill = given_keys_in(earlier_saved.as_ref(), &old)
                    == given_keys_in(self.saved.as_ref(), &shown);
                shown.take_reads_of(old, keeps_bill);
            }
            if !shown.has_config_dir() {
                shown.forget_reads();
            } else if shown.start_session_read() {
                to_read.push(shown.profile().clone());
            }

            self.shown.push(shown);
        }

        self.hidden = loaded.hidden_profiles;
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
    /// with the next read of the profiles.
    pub(crate) fn apply_saved(&mut self, saved: SavedProfiles) {
        let (hidden, shown): (Vec<_>, Vec<_>) = std::mem::take(&mut self.shown)
            .into_iter()
            .partition(|shown| saved.is_profile_hidden(shown.profile()));

        self.shown = shown;
        self.hidden
            .extend(hidden.into_iter().map(|state| state.profile().clone()));
        self.saved = Some(saved);
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

    /// Start a read of the limits of each shown profile that is
    /// connected, that `which` picks and that has none on its way, and
    /// return those profiles. The read may start the agent's program only
    /// when `may_start_programs`, while the mode is on screen. A profile
    /// whose files say it pays with an API key has no limits to read: its
    /// money comes from its tokens (spec Decision 9). A profile whose
    /// account has a read on its way through another profile waits for
    /// that read: an account is read once. The limits of a hidden account
    /// are not read.
    pub(crate) fn start_limit_reads(
        &mut self,
        which: Which,
        may_start_programs: bool,
    ) -> Vec<Profile> {
        let mut started = Vec::new();

        for index in 0..self.shown.len() {
            let shown = &self.shown[index];
            let is_hidden = shown
                .limits_account()
                .is_some_and(|account| self.is_account_hidden(account));
            let is_wanted = !is_hidden
                && shown.reads_limits()
                && shown.limits().is_wanted(which);
            let is_read_by_another =
                shown.limits_account().is_some_and(|account| {
                    self.shown.iter().any(|other| {
                        other.limits().reads().is_reading()
                            && other.has_limits_of(account)
                    })
                });
            if !is_wanted || is_read_by_another {
                continue;
            }

            let shown = &mut self.shown[index];
            if shown.start_limit_read(may_start_programs) {
                started.push(shown.profile().clone());
            }
        }

        started
    }

    /// Take the limits read through one shown profile, and copy them to
    /// every other shown profile of its account that has no read of its
    /// own on its way.
    pub(crate) fn apply_limits(&mut self, loaded: LoadedLimits) {
        let Some(index) = position_of(&self.shown, &loaded.profile) else {
            return;
        };

        self.shown[index].apply_limits(loaded.limits, loaded.read_at);

        let Some(account) = self.shown[index].limits_account().cloned() else {
            return;
        };
        let limits = self.shown[index].limits().clone();
        for (other, shown) in self.shown.iter_mut().enumerate() {
            let shares = other != index
                && shown.reads_limits()
                && !shown.limits().reads().is_reading()
                && shown.has_limits_of(&account);
            if shares {
                shown.share_limits(&limits);
            }
        }
    }

    /// Start a count of the tokens of each shown profile whose config
    /// folder exists, that `which` picks and that has none on its way, and
    /// return those profiles.
    pub(crate) fn start_spend_reads(&mut self, which: Which) -> Vec<Profile> {
        self.shown
            .iter_mut()
            .filter(|shown| {
                shown.has_config_dir() && shown.spend().reads().is_wanted(which)
            })
            .filter_map(|shown| {
                shown.start_spend_count().then(|| shown.profile().clone())
            })
            .collect()
    }

    /// Take the tokens of one shown profile.
    pub(crate) fn apply_spend(&mut self, loaded: LoadedSpend) {
        let Some(shown) = shown_mut(&mut self.shown, &loaded.profile) else {
            return;
        };

        shown.apply_spend(loaded.spend, loaded.counted_at);
    }

    /// Start a read of the list prices when a shown profile pays with an
    /// API key, so its money is estimated with them, the prices are wanted
    /// for `which` at `now` and no read is on its way (spec Decision 16).
    /// Returns whether the read must fetch the price table again even
    /// within the day; `None` when no read starts.
    pub(crate) fn start_price_read(
        &mut self,
        which: Which,
        now: SystemTime,
    ) -> Option<bool> {
        let uses_estimate = self.shown.iter().any(ProfileState::is_api_key);
        if !uses_estimate || !self.prices.is_wanted(which, now) {
            return None;
        }

        let fetch_again = self.prices.fetch_again();
        self.prices.start_reading().then_some(fetch_again)
    }

    /// Take the list prices that were read: the money of the accounts is
    /// estimated at them when it is shown.
    pub(crate) fn apply_prices(&mut self, loaded: LoadedPrices) {
        self.prices.apply(loaded.prices, loaded.read_at);
    }

    /// Start a read of the bill of each shown API key profile that pays
    /// through an organization a provider part serves, that `which` picks
    /// and that has no read on its way, and return those profiles with the
    /// organization and the keys the user gave for it (spec Decision 9). A
    /// profile whose organization has a read on its way through another
    /// profile waits for that read: an organization is read once. The bill
    /// of a hidden organization is not read.
    pub(crate) fn start_bill_reads(
        &mut self,
        which: Which,
    ) -> Vec<(Profile, Account, Vec<GivenKey>)> {
        let mut started = Vec::new();

        for index in 0..self.shown.len() {
            let shown = &self.shown[index];
            let Some(organization) = self.bill_organization(shown).cloned()
            else {
                continue;
            };
            let is_wanted = !self.is_account_hidden(&organization)
                && shown.is_api_key()
                && shown.bill().reads().is_wanted(which);
            let is_read_by_another = self.shown.iter().any(|other| {
                other.bill().reads().is_reading()
                    && other.pays_through(&organization)
            });
            if !is_wanted || is_read_by_another {
                continue;
            }

            let given = self.given_keys_of(&organization);
            let shown = &mut self.shown[index];
            if shown.start_bill_read() {
                started.push((shown.profile().clone(), organization, given));
            }
        }

        started
    }

    /// Take the bill read through one shown profile, and copy it to every
    /// other shown profile of its organization that has no read of its
    /// own on its way.
    pub(crate) fn apply_bill(&mut self, loaded: LoadedBill) {
        let Some(index) = position_of(&self.shown, &loaded.profile) else {
            return;
        };

        self.shown[index].apply_bill(loaded.bill, loaded.read_at);

        let Some(organization) = self.shown[index].organization().cloned()
        else {
            return;
        };
        let bill = self.shown[index].bill().clone();
        for (other, shown) in self.shown.iter_mut().enumerate() {
            let shares = other != index
                && !shown.bill().reads().is_reading()
                && shown.pays_through(&organization);
            if shares {
                shown.share_bill(&bill);
            }
        }
    }

    /// Start a read of the numbers of each listed account that tells its
    /// own (spec 2026-11-19, Decision 5) that `which` picks and that has
    /// none on its way, and return those accounts with the place their key
    /// is read again from.
    pub(crate) fn start_own_reads(
        &mut self,
        which: Which,
    ) -> Vec<(Account, KeyPlace, Vec<GivenKey>)> {
        let wanted = self.own_read_places();

        self.own_reads
            .start(wanted, which)
            .into_iter()
            .map(|(account, place)| {
                let given = self.given_keys_of(&account);
                (account, place, given)
            })
            .collect()
    }

    /// Forget what was read of `account` with its own key, so it is read
    /// again at once.
    pub(crate) fn forget_own_read(&mut self, account: &Account) {
        self.own_reads.forget(account);
    }

    /// Take the numbers of one account read with its own key.
    pub(crate) fn apply_own_numbers(&mut self, loaded: LoadedAccountNumbers) {
        self.own_reads.apply(loaded);
    }

    /// The accounts not hidden whose own numbers OTTY reads, each with the
    /// place its key is read again from: an account the user added, from
    /// the place they gave; a key or a login, from the files of the first
    /// profile it was found in. A subscription login whose limits a
    /// profile of it reads already is left out: that profile asks its
    /// provider part first.
    fn own_read_places(&self) -> Vec<(Account, KeyPlace)> {
        self.accounts()
            .into_iter()
            .filter(|account| own_reader_of(account, &self.registry).is_some())
            .filter_map(|account| {
                let place = self.own_read_place(&account)?;
                Some((account, place))
            })
            .collect()
    }

    /// The place the key of `account` is read again from; `None` when it
    /// is not read by itself.
    fn own_read_place(&self, account: &Account) -> Option<KeyPlace> {
        if account.added_key().is_some() {
            return Some(KeyPlace::Added);
        }

        let mut profiles = self.found_with(account);
        if account.kind() != AccountKind::Money {
            let profiles: Vec<_> = profiles.collect();
            let is_read_by_a_profile = profiles.iter().any(|shown| {
                let agent = shown.profile().agent();
                self.registry.agent(agent).is_some() && shown.reads_limits()
            });
            if is_read_by_a_profile {
                return None;
            }
            return profiles
                .first()
                .map(|shown| KeyPlace::ProfileFiles(shown.profile().clone()));
        }

        profiles
            .next()
            .map(|shown| KeyPlace::ProfileFiles(shown.profile().clone()))
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

/// Where the profile `profile` is among `shown`.
fn position_of(shown: &[ProfileState], profile: &Profile) -> Option<usize> {
    shown
        .iter()
        .position(|shown| is_same_profile(shown.profile(), profile))
}

/// Where the admin key of the organization `shown` pays through is kept in
/// the content `saved` of `agents.json`.
fn given_keys_in<'a>(
    saved: Option<&'a SavedProfiles>,
    shown: &ProfileState,
) -> &'a [GivenKey] {
    match (saved, shown.organization()) {
        (Some(saved), Some(organization)) => saved.given_keys_of(organization),
        _ => &[],
    }
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

    use super::*;
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

    #[test]
    fn the_first_profile_of_an_account_is_the_first_it_was_found_in() {
        let state = with_accounts(
            &[
                (claude(), connected(), Vec::new()),
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
        let profiles = state.profiles();

        let first = profiles.first_with(&chatgpt_login());
        let none = profiles.first_with(&organization());

        assert_eq!(first.map(ProfileState::profile), Some(&codex()));
        assert!(none.is_none());
    }
}
