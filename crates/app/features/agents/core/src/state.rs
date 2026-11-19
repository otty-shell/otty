use std::path::{Path, PathBuf};

use otty_agents::{
    Account, Agent, AgentEnv, AgentId, Profile, Project, ProjectSession,
};
use otty_ui_kit::components::context_menu::MenuPlace;
use otty_ui_kit::components::tag_search::TagSchema;

use super::menus::{AgentsMenu, OpenMenu};
use super::new_session_dialog::NewSessionDialog;
use super::profile_dialog::ProfileDialog;
use super::profile_state::ProfileState;
use super::profiles::Profiles;
use super::project_search::ProjectSearch;
use super::reads::ReadKind;
use super::real_failures::ToldProblems;
use super::sessions_table::{self, SessionsTable, TableSession};
use super::stats_dialog::StatsDialog;
use super::types::AgentList;
use super::usage::UsageChoice;

/// The state of Agents mode: the profiles and what was read of them, what
/// is selected, the open profile, statistics and New session dialogs, the
/// menus and sections of the screen, the kind the start of OTTY reads, and
/// the real failures already told.
pub(crate) struct AgentsState {
    agents: AgentList,
    env: AgentEnv,
    is_active: bool,
    startup_reads: Option<ReadKind>,
    told: ToldProblems,
    profiles: Profiles,
    selection: Option<Selection>,
    table: SessionsTable,
    dialog: Option<ProfileDialog>,
    removing: Option<Profile>,
    stats: Option<StatsDialog>,
    new_session: Option<NewSessionDialog>,
    is_found_report_open: bool,
    was_found_report_opened: bool,
    kept_new_session_profile: Option<Profile>,
    is_accounts_open: bool,
    is_agents_open: bool,
    is_projects_open: bool,
    usage: UsageChoice,
    search: ProjectSearch,
    pointer: MenuPlace,
    has_local_pane: bool,
    local_folder: Option<PathBuf>,
    menu: Option<OpenMenu>,
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

    /// The profiles and what was read of them.
    pub(crate) fn profiles(&self) -> &Profiles {
        &self.profiles
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

    /// Whether the Found by OTTY report is open.
    pub(crate) fn is_found_report_open(&self) -> bool {
        self.is_found_report_open
    }

    /// Whether Agents mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.is_active
    }

    /// The kind of read the start of OTTY waits for while the mode is away;
    /// `None` once the start is read through or the mode opened.
    pub(crate) fn startup_reads(&self) -> Option<ReadKind> {
        self.startup_reads
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

    /// Whether the Accounts section of the left panel shows its rows.
    pub(crate) fn is_accounts_open(&self) -> bool {
        self.is_accounts_open
    }

    /// Whether the Profiles section of the left panel shows its rows.
    pub(crate) fn is_agents_open(&self) -> bool {
        self.is_agents_open
    }

    /// Whether the Projects section of the left panel shows its rows.
    pub(crate) fn is_projects_open(&self) -> bool {
        self.is_projects_open
    }

    /// What the usage block of every profile page shows.
    pub(crate) fn usage(&self) -> &UsageChoice {
        &self.usage
    }

    /// The search field of the projects and its choices.
    pub(crate) fn search(&self) -> &ProjectSearch {
        &self.search
    }

    /// The menu that is open, with where it opened.
    pub(crate) fn menu(&self) -> Option<&OpenMenu> {
        self.menu.as_ref()
    }

    /// The state of Agents mode before anything was read, with the agents
    /// OTTY knows and the environment they look at.
    pub(crate) fn new(agents: AgentList, env: AgentEnv) -> Self {
        let bill_agents = agents
            .iter()
            .map(|agent| agent.descriptor())
            .filter(|descriptor| descriptor.has_bill())
            .map(|descriptor| descriptor.id())
            .collect();

        Self {
            agents,
            env,
            is_active: false,
            startup_reads: None,
            told: ToldProblems::default(),
            profiles: Profiles::new(bill_agents),
            selection: None,
            table: SessionsTable::default(),
            dialog: None,
            removing: None,
            stats: None,
            new_session: None,
            is_found_report_open: false,
            was_found_report_opened: false,
            kept_new_session_profile: None,
            is_accounts_open: true,
            is_agents_open: true,
            is_projects_open: true,
            usage: UsageChoice::default(),
            search: ProjectSearch::default(),
            pointer: MenuPlace::default(),
            has_local_pane: false,
            local_folder: None,
            menu: None,
        }
    }

    /// The selected project, while it is in the list.
    pub(crate) fn selected_project(&self) -> Option<&Project> {
        self.profiles.project(self.selected_folder()?)
    }

    /// The folder of the selected project, while it is listed and exists:
    /// a session can be started or continued there.
    pub(crate) fn openable_selected_folder(&self) -> Option<&Path> {
        let folder = self.selected_project()?.folder();
        if self.profiles.is_missing(folder) {
            return None;
        }

        Some(folder)
    }

    /// Whether a new session can be started in the selected project: its
    /// folder exists and a profile can start.
    pub(crate) fn can_start_in_selected_project(&self) -> bool {
        self.openable_selected_folder().is_some()
            && !self.profiles.startable().is_empty()
    }

    /// Whether the session of `entry` can be continued in the selected
    /// project: its folder exists, its profile can start and its agent is
    /// known.
    pub(crate) fn can_continue(&self, entry: &ProjectSession) -> bool {
        self.openable_selected_folder().is_some()
            && self.profiles.can_start(entry.profile())
            && self.agent(entry.profile().agent()).is_some()
    }

    /// The selected profile, while it is shown.
    pub(crate) fn selected_profile(&self) -> Option<&ProfileState> {
        let Some(Selection::Profile { agent, name }) = &self.selection else {
            return None;
        };

        self.profiles.named(*agent, name)
    }

    /// Whether All accounts is selected.
    pub(crate) fn is_all_accounts_selected(&self) -> bool {
        self.selection == Some(Selection::AllAccounts)
    }

    /// Whether All profiles is selected.
    pub(crate) fn is_all_profiles_selected(&self) -> bool {
        self.selection == Some(Selection::AllProfiles)
    }

    /// The selected account, while it is found in a shown profile.
    pub(crate) fn selected_account(&self) -> Option<Account> {
        let Some(Selection::Account(selected)) = &self.selection else {
            return None;
        };

        self.profiles
            .accounts()
            .into_iter()
            .find(|account| account.is_same_account(selected))
    }

    /// The profile a new session starts with: the one the user picked
    /// last in the New session dialog, else the selected profile, else
    /// the first one; each only while it can start. `None` while no
    /// profile can start.
    pub(crate) fn new_session_profile(&self) -> Option<&Profile> {
        let can_start = |profile: &&Profile| self.profiles.can_start(profile);
        let kept = self.kept_new_session_profile.as_ref().filter(can_start);
        let selected = self
            .selected_profile()
            .map(ProfileState::profile)
            .filter(can_start);

        kept.or(selected)
            .or_else(|| self.profiles.startable().first().copied())
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
        otty_agents::agent_of(&self.agents, id)
    }

    /// Whether the agent with the id `id` reads the statistics of a session
    /// and of a project (spec Decision 14).
    pub(crate) fn has_stats(&self, id: AgentId) -> bool {
        self.agent(id)
            .is_some_and(|agent| agent.descriptor().has_stats())
    }

    /// The name of the icon of the agent with the id `id`; empty for an
    /// agent OTTY does not know, which gets the icon of the mode.
    pub(crate) fn icon_of(&self, id: AgentId) -> &'static str {
        self.agent(id).map_or("", |agent| agent.descriptor().icon())
    }

    /// The profiles and what was read of them, to start reads and take
    /// their answers.
    pub(crate) fn profiles_mut(&mut self) -> &mut Profiles {
        &mut self.profiles
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

    /// Open the Found by OTTY report, unless it was opened before in this
    /// run.
    pub(crate) fn open_found_report(&mut self) {
        if self.was_found_report_opened {
            return;
        }

        self.is_found_report_open = true;
        self.was_found_report_opened = true;
    }

    /// Close the Found by OTTY report; `false` when it was not open.
    pub(crate) fn close_found_report(&mut self) -> bool {
        std::mem::take(&mut self.is_found_report_open)
    }

    /// Open the Accounts section, or close it.
    pub(crate) fn toggle_accounts_section(&mut self) {
        self.is_accounts_open = !self.is_accounts_open;
    }

    /// Open the Profiles section, or close it.
    pub(crate) fn toggle_agents_section(&mut self) {
        self.is_agents_open = !self.is_agents_open;
    }

    /// Open the Projects section, or close it.
    pub(crate) fn toggle_projects_section(&mut self) {
        self.is_projects_open = !self.is_projects_open;
    }

    /// The choices of the usage block, to change them.
    pub(crate) fn usage_mut(&mut self) -> &mut UsageChoice {
        &mut self.usage
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

    /// Select All accounts.
    pub(crate) fn select_all_accounts(&mut self) {
        self.selection = Some(Selection::AllAccounts);
    }

    /// Select All profiles.
    pub(crate) fn select_all_profiles(&mut self) {
        self.selection = Some(Selection::AllProfiles);
    }

    /// Select the account `account`.
    pub(crate) fn select_account(&mut self, account: Account) {
        self.selection = Some(Selection::Account(account));
    }

    /// Select the profile `profile`.
    pub(crate) fn select_profile(&mut self, profile: &Profile) {
        self.usage.forget_rows();
        self.select_named(profile.agent(), profile.name());
    }

    /// Forget the selected profile, account or project when the latest read
    /// no longer has it, so the first profile can take its place.
    pub(crate) fn forget_selection_that_left(&mut self) {
        let is_gone = match &self.selection {
            Some(Selection::Profile { .. }) => {
                self.selected_profile().is_none()
            },
            Some(Selection::Project(_)) => self.selected_project().is_none(),
            Some(Selection::Account(_)) => self.selected_account().is_none(),
            _ => false,
        };

        if is_gone {
            self.selection = None;
        }
    }

    /// Select the first profile of the list while nothing is selected, so
    /// the mode opens on its page (spec Decision 10). A selection the
    /// user made stays.
    pub(crate) fn select_first_profile_if_none(&mut self) {
        if self.selection.is_some() {
            return;
        }
        let Some(first) = self.profiles.shown().first() else {
            return;
        };

        self.selection = Some(Selection::Profile {
            agent: first.profile().agent(),
            name: first.profile().name().to_string(),
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

    /// Select the project of `folder`.
    pub(crate) fn select_project(&mut self, folder: PathBuf) {
        self.selection = Some(Selection::Project(folder));
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

/// What the main area shows: a project, a profile, an account, or All
/// accounts or All profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Selection {
    /// Every account.
    AllAccounts,
    /// Every profile.
    AllProfiles,
    /// This account, found again by its provider and identity after the
    /// profiles are read again.
    Account(Account),
    /// The project in this folder.
    Project(PathBuf),
    /// The profile of this agent with this name. Names are unique, and the
    /// name finds the profile again after the profiles are read again.
    Profile { agent: AgentId, name: String },
}
