use std::path::{Path, PathBuf};

use otty_agents::{
    Agent, AgentEnv, AgentId, AgentSession, Profile, ProfileStatus, Project,
    SavedProfiles, SourceFailure,
};

use super::limits::ProfileLimits;
use super::profile_dialog::ProfileDialog;
use super::spend::ProfileSpend;
use super::types::{
    AgentList, LoadedLimits, LoadedProfile, LoadedSessions, LoadedSpend,
};

/// The state of Agents mode: the profiles with their sessions, the projects
/// made from them, what is selected, and the open profile dialog.
pub(crate) struct AgentsState {
    agents: AgentList,
    env: AgentEnv,
    is_active: bool,
    is_loading: bool,
    is_reload_wanted: bool,
    is_loaded: bool,
    saved: Option<SavedProfiles>,
    profiles: Vec<ProfileState>,
    hidden: Vec<Profile>,
    is_hidden_list_open: bool,
    projects: Vec<Project>,
    selection: Option<Selection>,
    is_choosing_profile: bool,
    dialog: Option<ProfileDialog>,
    removing: Option<Profile>,
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

    /// The hidden profiles, in their order.
    pub(crate) fn hidden(&self) -> &[Profile] {
        &self.hidden
    }

    /// Whether the hidden profiles are listed.
    pub(crate) fn is_hidden_list_open(&self) -> bool {
        self.is_hidden_list_open
    }

    /// The open profile dialog.
    pub(crate) fn dialog(&self) -> Option<&ProfileDialog> {
        self.dialog.as_ref()
    }

    /// The profile the user is asked to confirm the removal of.
    pub(crate) fn removing(&self) -> Option<&Profile> {
        self.removing.as_ref()
    }

    /// Whether Agents mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.is_active
    }

    /// Whether the profiles were read at least once.
    pub(crate) fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    /// The profiles shown, without the hidden ones, in their order.
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

    /// Whether the user is asked which profile starts the new session.
    pub(crate) fn is_choosing_profile(&self) -> bool {
        self.is_choosing_profile
    }

    /// The state of Agents mode before anything was read, with the agents
    /// OTTY knows and the environment they look at.
    pub(crate) fn new(agents: AgentList, env: AgentEnv) -> Self {
        Self {
            agents,
            env,
            is_active: false,
            is_loading: false,
            is_reload_wanted: false,
            is_loaded: false,
            saved: None,
            profiles: Vec::new(),
            hidden: Vec::new(),
            is_hidden_list_open: false,
            projects: Vec::new(),
            selection: None,
            is_choosing_profile: false,
            dialog: None,
            removing: None,
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

    /// Every profile, shown and hidden: the names and folders a new or
    /// edited profile is checked against.
    pub(crate) fn all_profiles(&self) -> Vec<Profile> {
        self.profiles
            .iter()
            .map(|shown| shown.profile.clone())
            .chain(self.hidden.iter().cloned())
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

    /// The agent with the id `id`.
    pub(crate) fn agent(&self, id: AgentId) -> Option<&dyn Agent> {
        self.agents
            .iter()
            .find(|agent| agent.descriptor().id() == id)
            .map(|agent| agent.as_ref())
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

    /// The profiles a new session can start with, in their order.
    pub(crate) fn startable_profiles(&self) -> Vec<&Profile> {
        self.profiles
            .iter()
            .map(ProfileState::profile)
            .filter(|profile| self.can_start(profile))
            .collect()
    }

    /// Make Agents mode active or not. Leaving it drops the choice of a
    /// profile.
    pub(crate) fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active;
        if !is_active {
            self.is_choosing_profile = false;
        }
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

    /// Take what `agents.json` held at the read that just ended.
    pub(crate) fn set_saved(&mut self, saved: Option<SavedProfiles>) {
        self.saved = saved;
    }

    /// Select the profile `profile` and drop the choice of a profile for a
    /// new session.
    pub(crate) fn select_profile(&mut self, profile: &Profile) {
        self.select_named(profile.agent(), profile.name());
    }

    /// Select the profile of `agent` named `name`, which may be read only
    /// later, and drop the choice of a profile for a new session.
    pub(crate) fn select_named(&mut self, agent: AgentId, name: &str) {
        self.selection = Some(Selection::Profile {
            agent,
            name: name.to_string(),
        });
        self.is_choosing_profile = false;
    }

    /// List the hidden profiles, or stop listing them.
    pub(crate) fn toggle_hidden_list(&mut self) {
        self.is_hidden_list_open = !self.is_hidden_list_open;
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

    /// Take the profiles that were read and return those whose sessions to
    /// read. Hidden profiles are left out. A profile that was shown before
    /// keeps its sessions and its limits until they are read again, so the
    /// lists do not go empty during a refresh. A profile whose config
    /// folder is gone has nothing to read.
    pub(crate) fn apply_profiles(
        &mut self,
        loaded: Vec<LoadedProfile>,
    ) -> Vec<Profile> {
        let mut earlier = std::mem::take(&mut self.profiles);
        let mut to_read = Vec::new();
        self.hidden = Vec::new();

        for LoadedProfile { profile, status } in loaded {
            if profile.is_hidden() {
                self.hidden.push(profile);
                continue;
            }

            let mut shown = ProfileState::new(profile, status);
            let position =
                earlier.iter().position(|old| old.profile == shown.profile);
            if let Some(position) = position {
                shown.take_reads_of(earlier.swap_remove(position));
            }
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

    /// Start a read of the limits of each shown profile that is connected
    /// and has none on its way, and return those profiles.
    pub(crate) fn start_limit_reads(&mut self) -> Vec<Profile> {
        self.profiles
            .iter_mut()
            .filter(|shown| {
                matches!(shown.status, ProfileStatus::Connected { .. })
            })
            .filter_map(|shown| {
                shown.limits.start_reading().then(|| shown.profile.clone())
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

        shown.limits.apply(loaded.limits, loaded.read_at);
        true
    }

    /// Start a count of the tokens of each shown profile whose config
    /// folder exists and that has none on its way, and return those
    /// profiles.
    pub(crate) fn start_spend_reads(&mut self) -> Vec<Profile> {
        self.profiles
            .iter_mut()
            .filter(|shown| shown.status != ProfileStatus::ConfigDirNotFound)
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
        true
    }

    /// Select the project of `folder` and drop the choice of a profile.
    pub(crate) fn select_project(&mut self, folder: PathBuf) {
        self.selection = Some(Selection::Project(folder));
        self.is_choosing_profile = false;
    }

    /// Ask which profile starts the new session, or stop asking.
    pub(crate) fn set_choosing_profile(&mut self, is_choosing: bool) {
        self.is_choosing_profile = is_choosing;
    }

    /// Make the projects again from the sessions of every profile.
    fn rebuild_projects(&mut self) {
        let read = self.profiles.iter().filter_map(|shown| {
            let sessions = shown.sessions.as_deref()?;
            Some((&shown.profile, sessions))
        });

        self.projects = otty_agents::projects(read);
    }
}

#[cfg(test)]
impl AgentsState {
    /// Whether the profiles are being read.
    pub(crate) fn is_loading(&self) -> bool {
        self.is_loading
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
        }
    }

    /// Keep what was read of the same profile before: its sessions, its
    /// limits and its tokens.
    fn take_reads_of(&mut self, earlier: ProfileState) {
        self.sessions = earlier.sessions;
        self.failures = earlier.failures;
        self.missing_folders = earlier.missing_folders;
        self.limits = earlier.limits;
        self.spend = earlier.spend;
    }

    /// Forget what was read of a profile whose config folder is gone.
    fn forget_reads(&mut self) {
        self.sessions = None;
        self.failures = None;
        self.missing_folders = Vec::new();
        self.limits = ProfileLimits::default();
        self.spend = ProfileSpend::default();
    }
}
