//! What Agents mode read of the profiles: each shown profile with what was
//! read of it, the projects made from their sessions, the list prices of
//! the money estimate, what `agents.json` held, the user's time zone, and
//! the reads of the profiles themselves (spec Decisions 5, 9, 10 and 16).

use std::path::Path;
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{AgentId, Profile, ProfileStatus, Project, SavedProfiles};

use super::freshness::{ReadState, Which};
use super::prices::ListPrices;
use super::profile_state::ProfileState;
use super::reads::ReadKind;
use super::types::{
    LoadedBill, LoadedLimits, LoadedPrices, LoadedProfile, LoadedProfiles,
    LoadedSessions, LoadedSpend,
};

/// The profiles and what was read of them.
pub(crate) struct Profiles {
    shown: Vec<ProfileState>,
    projects: Vec<Project>,
    prices: ListPrices,
    saved: Option<SavedProfiles>,
    time_zone: Option<TimeZone>,
    reads: ReadState,
    is_reload_wanted: bool,
    sessions_wanted: Vec<Profile>,
    bill_agents: Vec<AgentId>,
}

impl Profiles {
    /// The profiles, in their order.
    pub(crate) fn shown(&self) -> &[ProfileState] {
        &self.shown
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
            ReadKind::Limits => shown.any(|state| state.limits().is_reading()),
            ReadKind::Spend => {
                shown.any(|state| state.spend().reads().is_reading())
            },
            ReadKind::Prices => self.prices.reads().is_reading(),
            ReadKind::Bills => {
                shown.any(|state| state.bill().reads().is_reading())
            },
        }
    }

    /// The shown profile `profile`, with what was read of it.
    pub(crate) fn get(&self, profile: &Profile) -> Option<&ProfileState> {
        self.shown.iter().find(|shown| shown.profile() == profile)
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

    /// Every profile: the names and folders a new or edited profile is
    /// checked against.
    pub(crate) fn all(&self) -> Vec<&Profile> {
        self.shown.iter().map(ProfileState::profile).collect()
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

    /// Nothing read yet; `bill_agents` are the agents whose bill can be
    /// read.
    pub(crate) fn new(bill_agents: Vec<AgentId>) -> Self {
        Self {
            shown: Vec::new(),
            projects: Vec::new(),
            prices: ListPrices::default(),
            saved: None,
            time_zone: None,
            reads: ReadState::default(),
            is_reload_wanted: false,
            sessions_wanted: Vec::new(),
            bill_agents,
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

    /// Whether a read must follow the one that just ended, and forget it.
    pub(crate) fn take_reload_wanted(&mut self) -> bool {
        std::mem::take(&mut self.is_reload_wanted)
    }

    /// Take the profiles that were read and return those whose sessions
    /// to read. A profile that was shown before keeps its sessions and its
    /// limits until they are read again, so the lists do not go empty
    /// during a refresh. A profile whose config folder is gone has nothing
    /// to read. A time zone that could not be read keeps the one known.
    pub(crate) fn apply_profiles(
        &mut self,
        loaded: LoadedProfiles,
    ) -> Vec<Profile> {
        let mut earlier = std::mem::take(&mut self.shown);
        let mut to_read = Vec::new();

        for LoadedProfile { profile, status } in loaded.profiles {
            let mut shown = ProfileState::new(profile, status);
            let position = earlier
                .iter()
                .position(|old| old.profile() == shown.profile());
            if let Some(position) = position {
                shown.take_reads_of(earlier.swap_remove(position));
            }
            shown.estimate_money(self.prices.table());
            if !shown.has_config_dir() {
                shown.forget_reads();
            } else if shown.start_session_read() {
                to_read.push(shown.profile().clone());
            }

            self.shown.push(shown);
        }

        self.saved = loaded.saved;
        if loaded.time_zone.is_some() {
            self.time_zone = loaded.time_zone;
        }
        self.reads.end_at(loaded.read_at);
        self.sessions_wanted.clear();
        self.rebuild_projects();

        to_read
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
                wanted.contains(shown.profile()) && shown.has_config_dir()
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
    /// money comes from its tokens (spec Decision 9).
    pub(crate) fn start_limit_reads(
        &mut self,
        which: Which,
        may_start_programs: bool,
    ) -> Vec<Profile> {
        self.shown
            .iter_mut()
            .filter(|shown| {
                matches!(shown.status(), ProfileStatus::Connected { .. })
                    && !shown.files_say_api_key()
                    && shown.limits().is_wanted(which)
            })
            .filter_map(|shown| {
                shown
                    .start_limit_read(may_start_programs)
                    .then(|| shown.profile().clone())
            })
            .collect()
    }

    /// Take the limits of one shown profile.
    pub(crate) fn apply_limits(&mut self, loaded: LoadedLimits) {
        let Some(shown) = shown_mut(&mut self.shown, &loaded.profile) else {
            return;
        };

        shown.apply_limits(
            loaded.limits,
            loaded.sources,
            loaded.read_at,
            self.prices.table(),
        );
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
    /// has a bill, that has an admin key, that `which` picks and that has
    /// no read on its way, and return those profiles (spec Decision 9).
    pub(crate) fn start_bill_reads(&mut self, which: Which) -> Vec<Profile> {
        let bill_agents = &self.bill_agents;

        self.shown
            .iter_mut()
            .filter(|shown| {
                shown.is_api_key()
                    && shown.profile().admin_key().is_some()
                    && bill_agents.contains(&shown.profile().agent())
                    && shown.bill().reads().is_wanted(which)
            })
            .filter_map(|shown| {
                shown.start_bill_read().then(|| shown.profile().clone())
            })
            .collect()
    }

    /// Take the bill of one shown profile.
    pub(crate) fn apply_bill(&mut self, loaded: LoadedBill) {
        let Some(shown) = shown_mut(&mut self.shown, &loaded.profile) else {
            return;
        };

        shown.apply_bill(loaded.bill, loaded.read_at);
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

#[cfg(test)]
impl Profiles {
    /// Estimate the money at `prices`, as if LiteLLM gave them just now.
    pub(crate) fn set_prices(&mut self, prices: otty_agents::PriceTable) {
        let fetched =
            otty_agents::FetchedPrices::new(prices, SystemTime::now());
        let answer = std::iter::once((
            otty_agents::SourceId::LiteLlmPrices,
            Ok(fetched),
        ));

        self.apply_prices(LoadedPrices {
            prices: otty_agents::fixtures::walked(answer),
            read_at: SystemTime::now(),
        });
    }
}

/// The profile `profile` among `shown`, to take what was read of it.
fn shown_mut<'a>(
    shown: &'a mut [ProfileState],
    profile: &Profile,
) -> Option<&'a mut ProfileState> {
    shown.iter_mut().find(|shown| shown.profile() == profile)
}
