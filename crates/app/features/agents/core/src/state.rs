use std::path::{Path, PathBuf};
use std::sync::Arc;

use otty_agents::{
    Account, Agent, AgentEnv, AgentId, Profile, Project, ProjectSession,
    ProviderRef, Registry,
};
use otty_app_domain::plugins::PluginNotes;
use otty_ui_kit::components::context_menu::MenuPlace;
use otty_ui_kit::components::tag_search::TagSchema;

use super::add_account_dialog::AddAccountDialog;
use super::menus::{AgentsMenu, OpenMenu};
use super::model::EntryIcon;
use super::new_session_dialog::NewSessionDialog;
use super::profile_dialog::ProfileDialog;
use super::profile_state::ProfileState;
use super::profiles::Profiles;
use super::project_search::ProjectSearch;
use super::reads::StartWait;
use super::real_failures::ToldProblems;
use super::rename_dialog::RenameDialog;
use super::sessions_table::{self, SessionsTable, TableSession};
use super::stats_dialog::StatsDialog;
use super::types::{AgentList, AgentReaders};
use super::usage::UsageChoice;

/// The state of Agents mode: the profiles and what was read of them, what
/// is selected, the open profile, keyring, Rename, statistics and New
/// session dialogs, the menus and sections of the screen, whether the hidden
/// accounts and profiles are listed, what the start of OTTY waits for, and
/// the real failures already told.
pub(crate) struct AgentsState {
    agents: AgentList,
    registry: Arc<Registry>,
    plugin_notes: PluginNotes,
    stats_agents: Vec<AgentId>,
    env: AgentEnv,
    is_active: bool,
    start_wait: Option<StartWait>,
    told: ToldProblems,
    profiles: Profiles,
    selection: Option<Selection>,
    table: SessionsTable,
    dialog: Option<ProfileDialog>,
    removing: Option<Profile>,
    fix_tabs: Vec<u64>,
    is_keyring_dialog_open: bool,
    is_keyring_given: bool,
    rename: Option<RenameDialog>,
    add_account: Option<AddAccountDialog>,
    stats: Option<StatsDialog>,
    new_session: Option<NewSessionDialog>,
    is_found_report_open: bool,
    was_found_report_opened: bool,
    kept_new_session_profile: Option<Profile>,
    is_accounts_open: bool,
    is_agents_open: bool,
    is_projects_open: bool,
    is_showing_hidden_accounts: bool,
    is_showing_hidden_profiles: bool,
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

    /// The agents and providers OTTY knows right now: the names, icons,
    /// usage pages and hosts the mode shows.
    pub(crate) fn registry(&self) -> &Registry {
        &self.registry
    }

    /// What the mode knows of the installed plugins besides the registry:
    /// the parts that are off and the updates that wait.
    pub(crate) fn plugin_notes(&self) -> &PluginNotes {
        &self.plugin_notes
    }

    /// The registry as the feature shares it with the reads.
    pub(crate) fn registry_arc(&self) -> &Arc<Registry> {
        &self.registry
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

    /// Whether the keyring dialog asks the user to allow the read of the
    /// system keyring.
    pub(crate) fn is_keyring_dialog_open(&self) -> bool {
        self.is_keyring_dialog_open
    }

    /// Whether the plugins were let read the system keyring in this run.
    pub(crate) fn is_keyring_given(&self) -> bool {
        self.is_keyring_given
    }

    /// The open Rename dialog of an account or a profile.
    pub(crate) fn rename(&self) -> Option<&RenameDialog> {
        self.rename.as_ref()
    }

    /// The open Add account or Edit key dialog.
    pub(crate) fn add_account(&self) -> Option<&AddAccountDialog> {
        self.add_account.as_ref()
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

    /// Whether the tab `tab_id` fixes what a profile needs.
    pub(crate) fn is_fix_tab(&self, tab_id: u64) -> bool {
        self.fix_tabs.contains(&tab_id)
    }

    /// Whether Agents mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.is_active
    }

    /// What the start of OTTY waits for while the mode is away; `None`
    /// once the start is read through or the mode opened.
    pub(crate) fn start_wait(&self) -> Option<StartWait> {
        self.start_wait
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

    /// Whether the hidden accounts are listed, marked hidden (All accounts
    /// "⋯" → Show hidden accounts).
    pub(crate) fn is_showing_hidden_accounts(&self) -> bool {
        self.is_showing_hidden_accounts
    }

    /// Whether the hidden profiles are listed, marked hidden (All profiles
    /// "⋯" → Show hidden profiles).
    pub(crate) fn is_showing_hidden_profiles(&self) -> bool {
        self.is_showing_hidden_profiles
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
    /// OTTY knows, the registry that names them and their providers, and
    /// the environment they look at.
    pub(crate) fn new(
        agents: AgentList,
        registry: Arc<Registry>,
        env: AgentEnv,
    ) -> Self {
        let profiles = Profiles::new(Arc::clone(&registry));
        let stats_agents = stats_agents(&agents);

        Self {
            agents,
            registry,
            plugin_notes: PluginNotes::default(),
            stats_agents,
            env,
            is_active: false,
            start_wait: None,
            told: ToldProblems::default(),
            profiles,
            selection: None,
            table: SessionsTable::default(),
            dialog: None,
            removing: None,
            fix_tabs: Vec::new(),
            is_keyring_dialog_open: false,
            is_keyring_given: false,
            rename: None,
            add_account: None,
            stats: None,
            new_session: None,
            is_found_report_open: false,
            was_found_report_opened: false,
            kept_new_session_profile: None,
            is_accounts_open: true,
            is_agents_open: true,
            is_projects_open: true,
            is_showing_hidden_accounts: false,
            is_showing_hidden_profiles: false,
            usage: UsageChoice::default(),
            search: ProjectSearch::default(),
            pointer: MenuPlace::default(),
            has_local_pane: false,
            local_folder: None,
            menu: None,
        }
    }

    /// Read with `readers` from now on, as when the installed plugins
    /// change. What was read stays until it is read again.
    pub(crate) fn replace_readers(&mut self, readers: AgentReaders) {
        self.profiles.set_readers(Arc::clone(&readers.registry));
        self.stats_agents = stats_agents(&readers.agents);
        self.agents = readers.agents;
        self.registry = readers.registry;
        self.env = readers.env;
    }

    pub(crate) fn set_plugin_notes(&mut self, notes: PluginNotes) {
        self.plugin_notes = notes;
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

        self.profiles.named(agent, name)
    }

    /// Whether All accounts is selected.
    pub(crate) fn is_all_accounts_selected(&self) -> bool {
        self.selection == Some(Selection::AllAccounts)
    }

    /// Whether All profiles is selected.
    pub(crate) fn is_all_profiles_selected(&self) -> bool {
        self.selection == Some(Selection::AllProfiles)
    }

    /// The selected hidden profile, while it is hidden.
    pub(crate) fn selected_hidden_profile(&self) -> Option<&Profile> {
        let Some(Selection::Profile { agent, name }) = &self.selection else {
            return None;
        };

        self.profiles.hidden_named(agent, name)
    }

    /// The selected account, hidden or not, while it is found in a shown
    /// profile.
    pub(crate) fn selected_account(&self) -> Option<Account> {
        let Some(Selection::Account(selected)) = &self.selection else {
            return None;
        };

        let accounts = self.profiles.all_accounts();
        let exact = accounts
            .iter()
            .position(|account| account.is_same_account(selected));
        // The same login read by another plugin, or by none: an account
        // the user just added, or one whose plugin left (issue 29).
        let same_login = || {
            accounts
                .iter()
                .position(|account| account.is_same_login(selected))
        };

        let position = exact.or_else(same_login)?;
        accounts.into_iter().nth(position)
    }

    /// The accounts the lists show: those not hidden, and the hidden ones
    /// too while they are shown, in the order of the profiles.
    pub(crate) fn listed_accounts(&self) -> Vec<Account> {
        let mut accounts = self.profiles.all_accounts();

        if !self.is_showing_hidden_accounts {
            accounts
                .retain(|account| !self.profiles.is_account_hidden(account));
        }
        accounts
    }

    /// The hidden profiles the lists show after the others: all of them
    /// while they are shown, else none.
    pub(crate) fn listed_hidden_profiles(&self) -> &[Profile] {
        if !self.is_showing_hidden_profiles {
            return &[];
        }

        self.profiles.hidden_profiles()
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

    /// The name of the agent with the id `id`, such as "Coder", from its
    /// registry entry; the id itself for an agent with no entry.
    pub(crate) fn agent_name<'a>(&'a self, id: &'a AgentId) -> &'a str {
        self.registry
            .agent(id)
            .map_or(id.as_str(), |entry| entry.name())
    }

    /// The name of the provider `provider`, such as "Example Cloud", from the
    /// registry entry that serves it; its id or host for a provider no
    /// entry serves.
    pub(crate) fn provider_name<'a>(
        &'a self,
        provider: &'a ProviderRef,
    ) -> &'a str {
        provider_name(&self.registry, provider)
    }

    /// The icon of the agent with the id `id`: the one of its registry
    /// entry, else the first letter of its name.
    pub(crate) fn agent_icon(&self, id: &AgentId) -> EntryIcon {
        let entry = self.registry.agent(id);

        EntryIcon::of(entry.and_then(|entry| entry.icon()), self.agent_name(id))
    }

    /// The icon of the provider `provider`: the one of the registry entry
    /// that serves it, else the first letter of its id or of the
    /// registrable domain of its host.
    pub(crate) fn provider_icon(&self, provider: &ProviderRef) -> EntryIcon {
        EntryIcon::of_provider(&self.registry, provider)
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
    pub(crate) fn agent(&self, id: &AgentId) -> Option<&dyn Agent> {
        otty_agents::agent_of(&self.agents, id)
    }

    /// Whether the agent with the id `id` reads the statistics of a session
    /// and of a project (spec Decision 14).
    pub(crate) fn has_stats(&self, id: &AgentId) -> bool {
        self.stats_agents.contains(id)
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

    /// Open the Found by OTTY report again, at the user's wish, however
    /// often it was opened before.
    pub(crate) fn reopen_found_report(&mut self) {
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

    /// The choices of the usage block on screen, to change them: those of
    /// the statistics popup while it is open, else those of the profile
    /// pages.
    pub(crate) fn shown_usage_mut(&mut self) -> &mut UsageChoice {
        match self.stats.as_mut() {
            Some(dialog) => dialog.usage_mut(),
            None => &mut self.usage,
        }
    }

    /// The search field of the projects, to change it.
    pub(crate) fn search_mut(&mut self) -> &mut ProjectSearch {
        &mut self.search
    }

    /// List the hidden accounts, marked hidden, or take them out of the
    /// lists.
    pub(crate) fn show_hidden_accounts(&mut self, is_showing: bool) {
        self.is_showing_hidden_accounts = is_showing;
    }

    /// List the hidden profiles, marked hidden, or take them out of the
    /// lists.
    pub(crate) fn show_hidden_profiles(&mut self, is_showing: bool) {
        self.is_showing_hidden_profiles = is_showing;
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

    /// Open `menu` at the pointer, in place of the menu that was open. The
    /// same menu, open already, closes: its trigger was pressed again.
    pub(crate) fn open_menu(&mut self, menu: AgentsMenu) {
        if self.menu.as_ref().is_some_and(|open| open.menu() == &menu) {
            self.menu = None;
            return;
        }

        self.menu = Some(OpenMenu::new(menu, self.pointer));
    }

    /// Close the open menu; `false` when none was open.
    pub(crate) fn close_menu(&mut self) -> bool {
        self.menu.take().is_some()
    }

    /// Wait at the start of OTTY for `wait`; `None` ends the start.
    pub(crate) fn set_start_wait(&mut self, wait: Option<StartWait>) {
        self.start_wait = wait;
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
        self.select_named(profile.agent().clone(), profile.name());
    }

    /// Forget the selected profile, account or project when the latest read
    /// no longer has it, so the first profile can take its place.
    pub(crate) fn forget_selection_that_left(&mut self) {
        let is_gone = match &self.selection {
            Some(Selection::Profile { .. }) => {
                self.selected_profile().is_none()
                    && self.selected_hidden_profile().is_none()
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
            agent: first.profile().agent().clone(),
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

    /// Open the Rename dialog in place of the one that was open.
    pub(crate) fn open_rename(&mut self, dialog: RenameDialog) {
        self.rename = Some(dialog);
    }

    /// The open Rename dialog, to change it.
    pub(crate) fn rename_mut(&mut self) -> Option<&mut RenameDialog> {
        self.rename.as_mut()
    }

    /// Close the Rename dialog and return it.
    pub(crate) fn take_rename(&mut self) -> Option<RenameDialog> {
        self.rename.take()
    }

    /// Open the Add account or Edit key dialog.
    pub(crate) fn open_add_account(&mut self, dialog: AddAccountDialog) {
        self.add_account = Some(dialog);
    }

    /// The open Add account or Edit key dialog, to change it.
    pub(crate) fn add_account_mut(&mut self) -> Option<&mut AddAccountDialog> {
        self.add_account.as_mut()
    }

    /// Close the Add account or Edit key dialog and return it.
    pub(crate) fn take_add_account(&mut self) -> Option<AddAccountDialog> {
        self.add_account.take()
    }

    /// Ask to confirm the removal of `profile`.
    pub(crate) fn ask_removal(&mut self, profile: Profile) {
        self.removing = Some(profile);
    }

    /// Stop asking about a removal and return the profile it was about.
    pub(crate) fn take_removing(&mut self) -> Option<Profile> {
        self.removing.take()
    }

    /// The tab `tab_id` fixes what a profile needs: once it closes, the
    /// profiles are read again.
    pub(crate) fn watch_fix_tab(&mut self, tab_id: u64) {
        self.fix_tabs.push(tab_id);
    }

    /// Forget the tab `tab_id`, which closed; whether it was a tab that
    /// fixes what a profile needs.
    pub(crate) fn forget_fix_tab(&mut self, tab_id: u64) -> bool {
        let count = self.fix_tabs.len();
        self.fix_tabs.retain(|watched| *watched != tab_id);

        self.fix_tabs.len() != count
    }

    /// Open or close the keyring dialog.
    pub(crate) fn set_keyring_dialog_open(&mut self, is_open: bool) {
        self.is_keyring_dialog_open = is_open;
    }

    /// Remember that the plugins were let read the system keyring in this
    /// run.
    pub(crate) fn give_keyring(&mut self) {
        self.is_keyring_given = true;
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
        &'a self,
        project: &'a Project,
    ) -> Vec<TableSession<'a>> {
        sessions_table::table_sessions(project, |id| self.agent_name(id))
    }
}

/// The name of the provider `provider` in `registry`, such as "Example Cloud";
/// its id or host for a provider no entry serves.
pub(crate) fn provider_name<'a>(
    registry: &'a Registry,
    provider: &'a ProviderRef,
) -> &'a str {
    registry
        .provider_of(provider)
        .map_or(provider.as_str(), |entry| entry.name())
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

/// The agents of `agents` whose statistics OTTY can read.
fn stats_agents(agents: &AgentList) -> Vec<AgentId> {
    agents
        .iter()
        .filter(|agent| agent.has_stats())
        .map(|agent| agent.entry().id().clone())
        .collect()
}
