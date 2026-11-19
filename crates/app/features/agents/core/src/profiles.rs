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
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    Account, AdminKeyRef, AgentId, Profile, ProfileStatus, Project,
    SavedProfiles, merge_accounts,
};

use super::freshness::{ReadState, Which};
use super::keyring::KeyringState;
use super::prices::ListPrices;
use super::profile_state::ProfileState;
use super::reads::ReadKind;
use super::types::{
    LoadedBill, LoadedLimits, LoadedPrices, LoadedProfile, LoadedProfiles,
    LoadedSessions, LoadedSpend, LoadedVersion,
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
    bill_agents: Vec<AgentId>,
    versions_asked: Vec<AgentId>,
    program_versions: Vec<(AgentId, String)>,
    keyring: KeyringState,
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

    /// Whether a read of the `kind` is on its way for any profile.
    pub(crate) fn is_reading_any(&self, kind: ReadKind) -> bool {
        let mut shown = self.shown.iter();

        match kind {
            ReadKind::Profiles => self.reads.is_reading(),
            ReadKind::Sessions => shown.any(ProfileState::is_reading_sessions),
            ReadKind::Limits => {
                shown.any(|state| state.limits().reads().is_reading())
            },
            ReadKind::Spend => {
                shown.any(|state| state.spend().reads().is_reading())
            },
            ReadKind::Prices => self.prices.reads().is_reading(),
            ReadKind::Bills => {
                shown.any(|state| state.bill().reads().is_reading())
            },
        }
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
        agent: AgentId,
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

    /// The accounts found in the shown profiles, hidden ones too, then
    /// those found in the system keyring, each once, in the order of the
    /// profiles.
    pub(crate) fn all_accounts(&self) -> Vec<Account> {
        let found = self
            .shown
            .iter()
            .flat_map(|shown| shown.accounts().iter().cloned())
            .chain(self.keyring.accounts().iter().cloned())
            .collect();

        merge_accounts(found)
    }

    /// What was read of the system keyring.
    pub(crate) fn keyring(&self) -> &KeyringState {
        &self.keyring
    }

    /// What was read of the system keyring, to start a read or take one.
    pub(crate) fn keyring_mut(&mut self) -> &mut KeyringState {
        &mut self.keyring
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
        agent: AgentId,
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

    /// Whether `profile` can log in: it is not logged in and its program
    /// was found.
    pub(crate) fn can_log_in(&self, profile: &Profile) -> bool {
        self.get(profile)
            .is_some_and(|shown| *shown.status() == ProfileStatus::NotLoggedIn)
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

    /// Whether the agent with the id `id` can read the real bill of an API
    /// key profile with an admin key (spec Decision 9).
    pub(crate) fn has_bill(&self, id: AgentId) -> bool {
        self.bill_agents.contains(&id)
    }

    /// Whether `profile` is still shown: an answer of a read of a profile
    /// no longer shown is dropped.
    pub(crate) fn is_shown(&self, profile: &Profile) -> bool {
        self.get(profile).is_some()
    }

    /// Where the admin key of the organization `shown` pays through is
    /// kept, as `agents.json` holds it; `None` with no organization or no
    /// key.
    pub(crate) fn admin_key_of(
        &self,
        shown: &ProfileState,
    ) -> Option<&AdminKeyRef> {
        admin_key_in(self.saved.as_ref(), shown)
    }

    /// Whether `profile` is still shown and its organization still has
    /// the admin key `admin_key`: a bill read with a key since changed is
    /// of another organization and is dropped.
    pub(crate) fn is_shown_with_admin_key(
        &self,
        profile: &Profile,
        admin_key: &AdminKeyRef,
    ) -> bool {
        self.get(profile)
            .is_some_and(|shown| self.admin_key_of(shown) == Some(admin_key))
    }

    /// Nothing read yet; `bill_agents` are the agents whose bill can be
    /// read.
    pub(crate) fn new(bill_agents: Vec<AgentId>) -> Self {
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
            bill_agents,
            versions_asked: Vec::new(),
            program_versions: Vec::new(),
            keyring: KeyringState::default(),
        }
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
    /// after an edit of its name; it keeps its bill while the admin key of
    /// its organization is the same. A profile whose config folder is gone
    /// has nothing to read. A time zone that could not be read keeps the
    /// one known.
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
            accounts,
            version,
        } in loaded.profiles
        {
            let mut shown = ProfileState::new(profile, status, accounts)
                .with_version(version);
            let position = earlier.iter().position(|old| {
                is_same_profile(old.profile(), shown.profile())
            });
            if let Some(position) = position {
                let old = earlier.swap_remove(position);
                let keeps_bill = admin_key_in(earlier_saved.as_ref(), &old)
                    == admin_key_in(self.saved.as_ref(), &shown);
                shown.take_reads_of(old, keeps_bill);
            }
            shown.estimate_money(self.prices.table());
            if !shown.has_config_dir() {
                shown.forget_reads();
            } else if shown.start_session_read() {
                to_read.push(shown.profile().clone());
            }

            self.shown.push(shown);
        }

        self.hidden = loaded.hidden_profiles;
        self.keyring.set_entries(loaded.keyring_entries);
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
            .find(|(asked, _)| *asked == agent)
            .map(|(_, version)| version.as_str());

        shown.version().or(from_program)
    }

    /// Start asking the program of `agent` for its version; `false` when
    /// it was asked before in this run.
    pub(crate) fn start_version_read(&mut self, agent: AgentId) -> bool {
        if self.versions_asked.contains(&agent) {
            return false;
        }

        self.versions_asked.push(agent);
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
        let prices = self.prices.table();
        let Some(index) = position_of(&self.shown, &loaded.profile) else {
            return;
        };

        self.shown[index].apply_limits(
            loaded.limits,
            loaded.sources,
            loaded.read_at,
            prices,
        );

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
                shown.share_limits(&limits, prices);
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

        shown.apply_spend(loaded.spend, loaded.counted_at, self.prices.table());
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

    /// Take the list prices that were read, and estimate the money of each
    /// API key profile again at them.
    pub(crate) fn apply_prices(&mut self, loaded: LoadedPrices) {
        self.prices.apply(loaded.prices, loaded.read_at);

        let prices = self.prices.table();
        for shown in &mut self.shown {
            shown.estimate_money(prices);
        }
    }

    /// Start a read of the bill of each shown API key profile whose agent
    /// has a bill, whose organization has an admin key, that `which` picks
    /// and that has no read on its way, and return those profiles with the
    /// admin key (spec Decision 9). A profile whose organization has a
    /// read on its way through another profile waits for that read: an
    /// organization is read once. The bill of a hidden organization is not
    /// read.
    pub(crate) fn start_bill_reads(
        &mut self,
        which: Which,
    ) -> Vec<(Profile, AdminKeyRef)> {
        let mut started = Vec::new();

        for index in 0..self.shown.len() {
            let shown = &self.shown[index];
            let Some(admin_key) = self.admin_key_of(shown).cloned() else {
                continue;
            };
            let is_hidden = shown.organization().is_some_and(|organization| {
                self.is_account_hidden(organization)
            });
            let is_wanted = !is_hidden
                && shown.is_api_key()
                && self.bill_agents.contains(&shown.profile().agent())
                && shown.bill().reads().is_wanted(which);
            let is_read_by_another =
                shown.organization().is_some_and(|organization| {
                    self.shown.iter().any(|other| {
                        other.bill().reads().is_reading()
                            && other.pays_through(organization)
                    })
                });
            if !is_wanted || is_read_by_another {
                continue;
            }

            let shown = &mut self.shown[index];
            if shown.start_bill_read() {
                started.push((shown.profile().clone(), admin_key));
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
fn admin_key_in<'a>(
    saved: Option<&'a SavedProfiles>,
    shown: &ProfileState,
) -> Option<&'a AdminKeyRef> {
    saved?.admin_key_of(shown.organization()?)
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
