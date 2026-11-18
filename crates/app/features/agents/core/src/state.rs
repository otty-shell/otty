use std::path::{Path, PathBuf};
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    Agent, AgentEnv, AgentId, AgentSession, BillingKind, MoneyEstimate,
    PriceTable, Profile, ProfileStatus, Project, SavedProfiles, SourceFailure,
};
use otty_ui_kit::components::tag_search::TagSchema;

use super::bill::ProfileBill;
use super::freshness::{self, Which};
use super::limits::ProfileLimits;
use super::menus::{AgentsMenu, MenuPlace, OpenMenu};
use super::new_session_dialog::NewSessionDialog;
use super::prices::ListPrices;
use super::profile_dialog::ProfileDialog;
use super::project_search::ProjectSearch;
use super::reads::ReadKind;
use super::real_failures::ToldProblems;
use super::sessions_table::{self, SessionsTable, TableSession};
use super::spend::ProfileSpend;
use super::stats_dialog::StatsDialog;
use super::top_tab::TopTab;
use super::types::{
    AgentList, LoadedBill, LoadedLimits, LoadedPrices, LoadedProfile,
    LoadedSessions, LoadedSpend,
};

/// The state of Agents mode: the profiles with their sessions, the projects
/// made from them, what is selected, the open profile, statistics and New
/// session dialogs, the kind the start of OTTY reads, and the real failures
/// already told.
pub(crate) struct AgentsState {
    agents: AgentList,
    bill_agents: Vec<AgentId>,
    stats_agents: Vec<AgentId>,
    env: AgentEnv,
    is_active: bool,
    is_loading: bool,
    is_reload_wanted: bool,
    is_loaded: bool,
    profiles_read_at: Option<SystemTime>,
    startup_reads: Option<ReadKind>,
    told: ToldProblems,
    saved: Option<SavedProfiles>,
    profiles: Vec<ProfileState>,
    projects: Vec<Project>,
    selection: Option<Selection>,
    table: SessionsTable,
    dialog: Option<ProfileDialog>,
    removing: Option<Profile>,
    stats: Option<StatsDialog>,
    new_session: Option<NewSessionDialog>,
    kept_new_session_profile: Option<Profile>,
    sessions_wanted: Vec<Profile>,
    prices: ListPrices,
    is_agents_open: bool,
    is_projects_open: bool,
    top_tab: TopTab,
    search: ProjectSearch,
    pointer: MenuPlace,
    has_local_pane: bool,
    local_folder: Option<PathBuf>,
    menu: Option<OpenMenu>,
    time_zone: Option<TimeZone>,
}

impl AgentsState {
    /// The agents OTTY knows.
    pub(crate) fn agents(&self) -> &AgentList {
        &self.agents
    }

    /// The parts of the environment the agents look at: the home folder
    /// for the folder proposed to a new profile.
    pub(crate) fn env(&self) -> &AgentEnv {
        &self.env
    }

    /// What `agents.json` held at the last read; `None` when it could not
    /// be read.
    pub(crate) fn saved(&self) -> Option<&SavedProfiles> {
        self.saved.as_ref()
    }

    /// The open profile dialog.
    pub(crate) fn dialog(&self) -> Option<&ProfileDialog> {
        self.dialog.as_ref()
    }

    /// The profile the user is asked to confirm the removal of.
    pub(crate) fn removing(&self) -> Option<&Profile> {
        self.removing.as_ref()
    }

    /// The open statistics dialog.
    pub(crate) fn stats(&self) -> Option<&StatsDialog> {
        self.stats.as_ref()
    }

    /// The open New session dialog.
    pub(crate) fn new_session(&self) -> Option<&NewSessionDialog> {
        self.new_session.as_ref()
    }

    /// The profile the user picked last in the New session dialog, kept
    /// while OTTY runs.
    pub(crate) fn kept_new_session_profile(&self) -> Option<&Profile> {
        self.kept_new_session_profile.as_ref()
    }

    /// The list prices the money of an API key profile is estimated at,
    /// as their latest read left them.
    pub(crate) fn prices(&self) -> &ListPrices {
        &self.prices
    }

    /// Whether Agents mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.is_active
    }

    /// Whether the profiles were read at least once.
    pub(crate) fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    /// Whether the profiles are being read.
    pub(crate) fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// The kind of read the start of OTTY waits for while the mode is away;
    /// `None` once the start is read through or the mode opened.
    pub(crate) fn startup_reads(&self) -> Option<ReadKind> {
        self.startup_reads
    }

    /// Whether the latest read of the profiles ended less than the period
    /// of the timer before `now`.
    pub(crate) fn are_profiles_fresh(&self, now: SystemTime) -> bool {
        self.profiles_read_at
            .is_some_and(|read_at| freshness::is_fresh(read_at, now))
    }

    /// Whether a read of the `kind` is on its way for any profile.
    pub(crate) fn is_reading_any(&self, kind: ReadKind) -> bool {
        match kind {
            ReadKind::Profiles => self.is_loading,
            ReadKind::Sessions => {
                self.profiles.iter().any(|shown| shown.is_reading)
            },
            ReadKind::Limits => {
                self.profiles.iter().any(|shown| shown.limits.is_reading())
            },
            ReadKind::Spend => {
                self.profiles.iter().any(|shown| shown.spend.is_reading())
            },
            ReadKind::Prices => self.prices.is_reading(),
            ReadKind::Bills => {
                self.profiles.iter().any(|shown| shown.bill.is_reading())
            },
        }
    }

    /// The profiles, in their order.
    pub(crate) fn profiles(&self) -> &[ProfileState] {
        &self.profiles
    }

    /// The projects of every profile's sessions, the latest first.
    pub(crate) fn projects(&self) -> &[Project] {
        &self.projects
    }

    /// The folder of the selected project, if one was selected.
    pub(crate) fn selected_folder(&self) -> Option<&Path> {
        match &self.selection {
            Some(Selection::Project(folder)) => Some(folder),
            _ => None,
        }
    }

    /// The sessions table: the search of each project, the sort and the
    /// selected session.
    pub(crate) fn table(&self) -> &SessionsTable {
        &self.table
    }

    /// Whether the tab in front of Terminal mode is a terminal tab on this
    /// computer with a focused pane, as the frame told with the latest
    /// intent.
    pub(crate) fn has_local_pane(&self) -> bool {
        self.has_local_pane
    }

    /// The folder of the focused pane of the tab in front on this
    /// computer, as the frame told with the latest intent: the New session
    /// dialog opens there.
    pub(crate) fn local_folder(&self) -> Option<&Path> {
        self.local_folder.as_deref()
    }

    /// Whether the Profiles section of the left panel shows its rows.
    pub(crate) fn is_agents_open(&self) -> bool {
        self.is_agents_open
    }

    /// Whether the Projects section of the left panel shows its rows.
    pub(crate) fn is_projects_open(&self) -> bool {
        self.is_projects_open
    }

    /// What the top section of every dashboard lists.
    pub(crate) fn top_tab(&self) -> TopTab {
        self.top_tab
    }

    /// The search field of the projects and its choices.
    pub(crate) fn search(&self) -> &ProjectSearch {
        &self.search
    }

    /// The menu that is open, with where it opened.
    pub(crate) fn menu(&self) -> Option<&OpenMenu> {
        self.menu.as_ref()
    }

    /// The user's time zone, as the latest read of the profiles that
    /// could read it told; `None` before that, and the times are then
    /// written in UTC.
    pub(crate) fn time_zone(&self) -> Option<&TimeZone> {
        self.time_zone.as_ref()
    }

    /// The state of Agents mode before anything was read, with the agents
    /// OTTY knows and the environment they look at.
    pub(crate) fn new(agents: AgentList, env: AgentEnv) -> Self {
        let bill_agents = agents
            .iter()
            .filter(|agent| agent.bill_source().is_some())
            .map(|agent| agent.descriptor().id())
            .collect();
        let stats_agents = agents
            .iter()
            .filter(|agent| agent.stats_source().is_some())
            .map(|agent| agent.descriptor().id())
            .collect();

        Self {
            agents,
            bill_agents,
            stats_agents,
            env,
            is_active: false,
            is_loading: false,
            is_reload_wanted: false,
            is_loaded: false,
            profiles_read_at: None,
            startup_reads: None,
            told: ToldProblems::default(),
            saved: None,
            profiles: Vec::new(),
            projects: Vec::new(),
            selection: None,
            table: SessionsTable::default(),
            dialog: None,
            removing: None,
            stats: None,
            new_session: None,
            kept_new_session_profile: None,
            sessions_wanted: Vec::new(),
            prices: ListPrices::default(),
            is_agents_open: true,
            is_projects_open: true,
            top_tab: TopTab::default(),
            search: ProjectSearch::default(),
            pointer: MenuPlace::default(),
            has_local_pane: false,
            local_folder: None,
            menu: None,
            time_zone: None,
        }
    }

    /// The selected project, while it is in the list.
    pub(crate) fn selected_project(&self) -> Option<&Project> {
        self.project(self.selected_folder()?)
    }

    /// The selected profile, while it is shown.
    pub(crate) fn selected_profile(&self) -> Option<&ProfileState> {
        let Some(Selection::Profile { agent, name }) = &self.selection else {
            return None;
        };

        self.profiles.iter().find(|shown| {
            shown.profile.agent() == *agent && shown.profile.name() == name
        })
    }

    /// Every profile: the names and folders a new or edited profile is
    /// checked against.
    pub(crate) fn all_profiles(&self) -> Vec<Profile> {
        self.profiles
            .iter()
            .map(|shown| shown.profile.clone())
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
        self.profiles.iter().any(|profile| {
            profile
                .missing_folders
                .iter()
                .any(|missing| missing == folder)
        })
    }

    /// The name of the agent with the id `id`, such as "Codex"; the id
    /// itself for an agent OTTY does not know.
    pub(crate) fn agent_name(&self, id: AgentId) -> &'static str {
        self.agent(id)
            .map_or(id.as_str(), |agent| agent.descriptor().name())
    }

    /// The sessions of the selected project as its table shows them: the
    /// ones its search lets through, in the order of the sort. Empty while
    /// no project is selected.
    pub(crate) fn shown_sessions(&self) -> Vec<TableSession<'_>> {
        let Some(project) = self.selected_project() else {
            return Vec::new();
        };

        let sessions = self.table_sessions(project);
        let search = self.table.search(project.folder());
        sessions_table::shown(sessions, search, self.table.sort())
    }

    /// What the search of the selected project's table looks in: its keys
    /// and every session. `None` while no project is selected.
    pub(crate) fn session_schema(&self) -> Option<TagSchema> {
        let project = self.selected_project()?;

        let sessions = self.table_sessions(project);
        Some(sessions_table::schema(sessions, self.table.sort()))
    }

    /// The agent with the id `id`.
    pub(crate) fn agent(&self, id: AgentId) -> Option<&dyn Agent> {
        self.agents
            .iter()
            .find(|agent| agent.descriptor().id() == id)
            .map(|agent| agent.as_ref())
    }

    /// Whether the agent with the id `id` can read the real bill of an API
    /// key profile with an admin key (spec Decision 9).
    pub(crate) fn has_bill(&self, id: AgentId) -> bool {
        self.bill_agents.contains(&id)
    }

    /// Whether the agent with the id `id` reads the statistics of a session
    /// and of a project (spec Decision 14).
    pub(crate) fn has_stats(&self, id: AgentId) -> bool {
        self.stats_agents.contains(&id)
    }

    /// The shown profile `profile`, with what was read of it.
    pub(crate) fn profile_state(
        &self,
        profile: &Profile,
    ) -> Option<&ProfileState> {
        self.profiles.iter().find(|shown| shown.profile == *profile)
    }

    /// The name of the icon of the agent with the id `id`; empty for an
    /// agent OTTY does not know, which gets the icon of the mode.
    pub(crate) fn icon_of(&self, id: AgentId) -> &'static str {
        self.agent(id).map_or("", |agent| agent.descriptor().icon())
    }

    /// Whether a session of `profile` can be started: the profile is shown
    /// and its program was found.
    pub(crate) fn can_start(&self, profile: &Profile) -> bool {
        self.profiles.iter().any(|shown| {
            shown.profile == *profile
                && shown.status != ProfileStatus::ProgramNotFound
        })
    }

    /// Whether `profile` can log in: it is not logged in and its program
    /// was found.
    pub(crate) fn can_log_in(&self, profile: &Profile) -> bool {
        self.profiles.iter().any(|shown| {
            shown.profile == *profile
                && shown.status == ProfileStatus::NotLoggedIn
        })
    }

    /// The profiles a new session can start with, in their order.
    pub(crate) fn startable_profiles(&self) -> Vec<&Profile> {
        self.profiles
            .iter()
            .map(ProfileState::profile)
            .filter(|profile| self.can_start(profile))
            .collect()
    }

    /// Make Agents mode active or not. Leaving it closes the open menu.
    pub(crate) fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active;
        if !is_active {
            self.menu = None;
        }
    }

    /// The sessions table, to change it.
    pub(crate) fn table_mut(&mut self) -> &mut SessionsTable {
        &mut self.table
    }

    /// Open the Profiles section, or close it.
    pub(crate) fn toggle_agents_section(&mut self) {
        self.is_agents_open = !self.is_agents_open;
    }

    /// Open the Projects section, or close it.
    pub(crate) fn toggle_projects_section(&mut self) {
        self.is_projects_open = !self.is_projects_open;
    }

    /// List `tab` in the top section of every dashboard.
    pub(crate) fn show_top(&mut self, tab: TopTab) {
        self.top_tab = tab;
    }

    /// The search field of the projects, to change it.
    pub(crate) fn search_mut(&mut self) -> &mut ProjectSearch {
        &mut self.search
    }

    /// Take where the pointer is and how big the main area is, as the frame
    /// told with the latest intent; the next menu opens there.
    pub(crate) fn set_pointer(&mut self, pointer: MenuPlace) {
        self.pointer = pointer;
    }

    /// Take whether the tab in front of Terminal mode is a terminal tab on
    /// this computer with a focused pane, as the frame told with the latest
    /// intent; Continue in a split splits that pane.
    pub(crate) fn set_has_local_pane(&mut self, has_local_pane: bool) {
        self.has_local_pane = has_local_pane;
    }

    /// Take the folder of the focused pane of the tab in front on this
    /// computer, as the frame told with the latest intent.
    pub(crate) fn set_local_folder(&mut self, folder: Option<PathBuf>) {
        self.local_folder = folder;
    }

    /// Open `menu` at the pointer, in place of the menu that was open.
    pub(crate) fn open_menu(&mut self, menu: AgentsMenu) {
        self.menu = Some(OpenMenu::new(menu, self.pointer));
    }

    /// Close the open menu; `false` when none was open.
    pub(crate) fn close_menu(&mut self) -> bool {
        self.menu.take().is_some()
    }

    /// Wait at the start of OTTY for the reads of `kind`; `None` ends the
    /// start.
    pub(crate) fn set_startup_reads(&mut self, kind: Option<ReadKind>) {
        self.startup_reads = kind;
    }

    /// The real failures already told, to tell the new ones.
    pub(crate) fn told_mut(&mut self) -> &mut ToldProblems {
        &mut self.told
    }

    /// Start a read of the profiles; `false` when one is on its way.
    pub(crate) fn start_loading(&mut self) -> bool {
        if self.is_loading {
            return false;
        }

        self.is_loading = true;
        true
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

    /// Keep the user's `time_zone` for every written time; `None`, a time
    /// zone that could not be read, keeps the one known.
    pub(crate) fn remember_time_zone(&mut self, time_zone: Option<TimeZone>) {
        if time_zone.is_some() {
            self.time_zone = time_zone;
        }
    }

    /// Keep when the read of the profiles that just ended ended.
    pub(crate) fn remember_profiles_read_at(&mut self, read_at: SystemTime) {
        self.profiles_read_at = Some(read_at);
    }

    /// Take what `agents.json` held at the read that just ended.
    pub(crate) fn set_saved(&mut self, saved: Option<SavedProfiles>) {
        self.saved = saved;
    }

    /// Select the profile `profile`.
    pub(crate) fn select_profile(&mut self, profile: &Profile) {
        self.select_named(profile.agent(), profile.name());
    }

    /// Forget the selected profile or project when the latest read no
    /// longer has it, so the first profile can take its place.
    pub(crate) fn forget_selection_that_left(&mut self) {
        let is_gone = match &self.selection {
            Some(Selection::Profile { .. }) => {
                self.selected_profile().is_none()
            },
            Some(Selection::Project(_)) => self.selected_project().is_none(),
            None => false,
        };

        if is_gone {
            self.selection = None;
        }
    }

    /// Select the first profile of the list while nothing is selected, so
    /// the mode opens on its dashboard (spec Decision 10). A selection the
    /// user made stays.
    pub(crate) fn select_first_profile_if_none(&mut self) {
        if self.selection.is_some() {
            return;
        }
        let Some(first) = self.profiles.first() else {
            return;
        };

        self.selection = Some(Selection::Profile {
            agent: first.profile.agent(),
            name: first.profile.name().to_string(),
        });
    }

    /// Select the profile of `agent` named `name`, which may be read only
    /// later.
    pub(crate) fn select_named(&mut self, agent: AgentId, name: &str) {
        self.selection = Some(Selection::Profile {
            agent,
            name: name.to_string(),
        });
    }

    /// Open the profile dialog in place of the one that was open.
    pub(crate) fn open_dialog(&mut self, dialog: ProfileDialog) {
        self.dialog = Some(dialog);
    }

    /// The open profile dialog, to change it.
    pub(crate) fn dialog_mut(&mut self) -> Option<&mut ProfileDialog> {
        self.dialog.as_mut()
    }

    /// Close the profile dialog and return it.
    pub(crate) fn take_dialog(&mut self) -> Option<ProfileDialog> {
        self.dialog.take()
    }

    /// Ask to confirm the removal of `profile`.
    pub(crate) fn ask_removal(&mut self, profile: Profile) {
        self.removing = Some(profile);
    }

    /// Stop asking about a removal and return the profile it was about.
    pub(crate) fn take_removing(&mut self) -> Option<Profile> {
        self.removing.take()
    }

    /// Open the statistics dialog in place of the one that was open.
    pub(crate) fn open_stats(&mut self, dialog: StatsDialog) {
        self.stats = Some(dialog);
    }

    /// The open statistics dialog, to fill it.
    pub(crate) fn stats_mut(&mut self) -> Option<&mut StatsDialog> {
        self.stats.as_mut()
    }

    /// Close the statistics dialog; `false` when none was open.
    pub(crate) fn close_stats(&mut self) -> bool {
        self.stats.take().is_some()
    }

    /// Open the New session dialog in place of the one that was open.
    pub(crate) fn open_new_session(&mut self, dialog: NewSessionDialog) {
        self.new_session = Some(dialog);
    }

    /// The open New session dialog, to change it.
    pub(crate) fn new_session_mut(&mut self) -> Option<&mut NewSessionDialog> {
        self.new_session.as_mut()
    }

    /// Close the New session dialog; `false` when none was open.
    pub(crate) fn close_new_session(&mut self) -> bool {
        self.new_session.take().is_some()
    }

    /// Keep `profile` as the one the New session dialog proposes next.
    pub(crate) fn keep_new_session_profile(&mut self, profile: Profile) {
        self.kept_new_session_profile = Some(profile);
    }

    /// Read the sessions of `profile` again the next time Agents mode
    /// opens: a session it started may have made a new project.
    pub(crate) fn read_sessions_later(&mut self, profile: Profile) {
        if !self.sessions_wanted.contains(&profile) {
            self.sessions_wanted.push(profile);
        }
    }

    /// Start a read of the sessions of each profile [`Self::read_sessions_later`]
    /// asked for that is shown, whose config folder is there and that has
    /// no read on its way, and return those profiles.
    pub(crate) fn start_wanted_session_reads(&mut self) -> Vec<Profile> {
        let wanted = std::mem::take(&mut self.sessions_wanted);

        self.profiles
            .iter_mut()
            .filter(|shown| {
                wanted.contains(&shown.profile)
                    && shown.status != ProfileStatus::ConfigDirNotFound
                    && !shown.is_reading
            })
            .map(|shown| {
                shown.is_reading = true;
                shown.profile.clone()
            })
            .collect()
    }

    /// Take the profiles that were read and return those whose sessions to
    /// read. A profile that was shown before
    /// keeps its sessions and its limits until they are read again, so the
    /// lists do not go empty during a refresh. A profile whose config
    /// folder is gone has nothing to read.
    pub(crate) fn apply_profiles(
        &mut self,
        loaded: Vec<LoadedProfile>,
    ) -> Vec<Profile> {
        let mut earlier = std::mem::take(&mut self.profiles);
        let mut to_read = Vec::new();

        for LoadedProfile { profile, status } in loaded {
            let mut shown = ProfileState::new(profile, status);
            let position =
                earlier.iter().position(|old| old.profile == shown.profile);
            if let Some(position) = position {
                shown.take_reads_of(earlier.swap_remove(position));
            }
            shown.estimate_money(self.prices.table());
            if shown.status == ProfileStatus::ConfigDirNotFound {
                shown.forget_reads();
            } else {
                shown.is_reading = true;
                to_read.push(shown.profile.clone());
            }

            self.profiles.push(shown);
        }

        self.is_loading = false;
        self.is_loaded = true;
        self.sessions_wanted.clear();
        self.rebuild_projects();

        to_read
    }

    /// Take the sessions of one profile; `false` when the profile is no
    /// longer shown.
    ///
    /// A failure keeps the sessions read before: the last good answer
    /// stays on screen. No failures at all means the agent can not list
    /// sessions: the profile has none.
    pub(crate) fn apply_sessions(&mut self, loaded: LoadedSessions) -> bool {
        let shown = self
            .profiles
            .iter_mut()
            .find(|shown| shown.profile == loaded.profile);
        let Some(shown) = shown else {
            return false;
        };

        shown.is_reading = false;
        match loaded.sessions {
            Ok(sessions) => {
                shown.sessions = Some(sessions);
                shown.failures = None;
                shown.missing_folders = loaded.missing_folders;
            },
            Err(failures) if failures.is_empty() => {
                shown.sessions = Some(Vec::new());
                shown.failures = None;
                shown.missing_folders = Vec::new();
            },
            Err(failures) => shown.failures = Some(failures),
        }

        self.rebuild_projects();
        true
    }

    /// Start a read of the limits of each shown profile that is
    /// connected, that `which` picks and that has none on its way, and
    /// return those profiles. The read may start the agent's program only
    /// while the mode is on screen. An API key profile has no limits to
    /// read: its money comes from its tokens (spec Decision 9).
    pub(crate) fn start_limit_reads(&mut self, which: Which) -> Vec<Profile> {
        let may_start_programs = self.is_active;

        self.profiles
            .iter_mut()
            .filter(|shown| {
                matches!(shown.status, ProfileStatus::Connected { .. })
                    && !shown.is_api_key()
                    && shown.limits.is_wanted(which)
            })
            .filter_map(|shown| {
                shown
                    .limits
                    .start_reading(may_start_programs)
                    .then(|| shown.profile.clone())
            })
            .collect()
    }

    /// Take the limits of one profile; `false` when the profile is no
    /// longer shown.
    pub(crate) fn apply_limits(&mut self, loaded: LoadedLimits) -> bool {
        let shown = self
            .profiles
            .iter_mut()
            .find(|shown| shown.profile == loaded.profile);
        let Some(shown) = shown else {
            return false;
        };

        shown
            .limits
            .apply(loaded.limits, loaded.sources, loaded.read_at);
        true
    }

    /// Start a count of the tokens of each shown profile whose config
    /// folder exists, that `which` picks and that has none on its way, and
    /// return those profiles.
    pub(crate) fn start_spend_reads(&mut self, which: Which) -> Vec<Profile> {
        self.profiles
            .iter_mut()
            .filter(|shown| {
                shown.status != ProfileStatus::ConfigDirNotFound
                    && shown.spend.is_wanted(which)
            })
            .filter_map(|shown| {
                shown.spend.start_reading().then(|| shown.profile.clone())
            })
            .collect()
    }

    /// Take the tokens of one profile; `false` when the profile is no
    /// longer shown.
    pub(crate) fn apply_spend(&mut self, loaded: LoadedSpend) -> bool {
        let shown = self
            .profiles
            .iter_mut()
            .find(|shown| shown.profile == loaded.profile);
        let Some(shown) = shown else {
            return false;
        };

        shown.spend.apply(loaded.spend, loaded.counted_at);
        shown.estimate_money(self.prices.table());
        true
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
        let uses_estimate = self.profiles.iter().any(ProfileState::is_api_key);
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
        for shown in &mut self.profiles {
            shown.estimate_money(prices);
        }
    }

    /// Start a read of the bill of each shown API key profile whose agent
    /// has a bill, that has an admin key, that `which` picks and that has
    /// no read on its way, and return those profiles (spec Decision 9).
    pub(crate) fn start_bill_reads(&mut self, which: Which) -> Vec<Profile> {
        let bill_agents = &self.bill_agents;

        self.profiles
            .iter_mut()
            .filter(|shown| {
                shown.is_api_key()
                    && shown.profile.admin_key().is_some()
                    && bill_agents.contains(&shown.profile.agent())
                    && shown.bill.is_wanted(which)
            })
            .filter_map(|shown| {
                shown.bill.start_reading().then(|| shown.profile.clone())
            })
            .collect()
    }

    /// Take the bill of one profile; `false` when the profile is no longer
    /// shown.
    pub(crate) fn apply_bill(&mut self, loaded: LoadedBill) -> bool {
        let shown = self
            .profiles
            .iter_mut()
            .find(|shown| shown.profile == loaded.profile);
        let Some(shown) = shown else {
            return false;
        };

        shown.bill.apply(loaded.bill, loaded.read_at);
        true
    }

    /// Select the project of `folder`.
    pub(crate) fn select_project(&mut self, folder: PathBuf) {
        self.selection = Some(Selection::Project(folder));
    }

    /// Make the projects again from the sessions of every profile.
    fn rebuild_projects(&mut self) {
        let read = self.profiles.iter().filter_map(|shown| {
            let sessions = shown.sessions.as_deref()?;
            Some((&shown.profile, sessions))
        });

        self.projects = otty_agents::projects(read);
    }

    /// Every session of `project`, newest first, with the names of their
    /// agents.
    fn table_sessions<'a>(
        &self,
        project: &'a Project,
    ) -> Vec<TableSession<'a>> {
        sessions_table::table_sessions(project, |id| self.agent_name(id))
    }
}

#[cfg(test)]
impl AgentsState {
    /// Estimate the money at `prices`, as if LiteLLM gave them just now.
    pub(crate) fn set_prices(&mut self, prices: PriceTable) {
        let fetched =
            otty_agents::FetchedPrices::new(prices, SystemTime::now());
        let answer = std::iter::once((
            otty_agents::SourceId::LiteLlmPrices,
            Ok(fetched),
        ));

        self.apply_prices(LoadedPrices {
            prices: otty_agents::walk(answer),
            read_at: SystemTime::now(),
        });
    }
}

/// What the main area shows: a project or a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Selection {
    /// The project in this folder.
    Project(PathBuf),
    /// The profile of this agent with this name. Names are unique, and the
    /// name finds the profile again after the profiles are read again.
    Profile { agent: AgentId, name: String },
}

/// One shown profile with what was read of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileState {
    profile: Profile,
    status: ProfileStatus,
    sessions: Option<Vec<AgentSession>>,
    failures: Option<Vec<SourceFailure>>,
    missing_folders: Vec<PathBuf>,
    is_reading: bool,
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

    /// Whether the sessions are being read.
    pub(crate) fn is_reading(&self) -> bool {
        self.is_reading
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

    fn new(profile: Profile, status: ProfileStatus) -> Self {
        Self {
            profile,
            status,
            sessions: None,
            failures: None,
            missing_folders: Vec::new(),
            is_reading: false,
            limits: ProfileLimits::default(),
            spend: ProfileSpend::default(),
            money: None,
            bill: ProfileBill::default(),
        }
    }

    /// Whether the profile pays per token with an API key, as its files
    /// tell.
    pub(crate) fn is_api_key(&self) -> bool {
        matches!(
            self.status,
            ProfileStatus::Connected {
                billing: Some(BillingKind::ApiKey),
                ..
            }
        )
    }

    /// Estimate the money of an API key profile from its last good count
    /// of tokens at `prices` (spec Decision 9); another profile, and any
    /// profile while there are no prices, has none.
    fn estimate_money(&mut self, prices: Option<&PriceTable>) {
        let is_api_key = self.is_api_key();

        self.money = self
            .spend
            .last_good()
            .filter(|_| is_api_key)
            .zip(prices)
            .map(|((spend, _), prices)| MoneyEstimate::new(spend, prices));
    }

    /// Keep what was read of the same profile before: its sessions, its
    /// limits, its tokens and its bill.
    fn take_reads_of(&mut self, earlier: ProfileState) {
        self.sessions = earlier.sessions;
        self.failures = earlier.failures;
        self.missing_folders = earlier.missing_folders;
        self.limits = earlier.limits;
        self.spend = earlier.spend;
        self.bill = earlier.bill;
    }

    /// Forget what was read of a profile whose config folder is gone.
    fn forget_reads(&mut self) {
        self.sessions = None;
        self.failures = None;
        self.missing_folders = Vec::new();
        self.limits = ProfileLimits::default();
        self.spend = ProfileSpend::default();
        self.money = None;
        self.bill = ProfileBill::default();
    }
}
