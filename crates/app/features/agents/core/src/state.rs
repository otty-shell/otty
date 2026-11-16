use std::path::{Path, PathBuf};

use otty_agents::{
    Agent, AgentId, AgentSession, Profile, ProfileStatus, Project,
    SourceFailure,
};

use super::types::{AgentList, LoadedProfile, LoadedSessions};

/// The state of Agents mode: the profiles with their sessions, the projects
/// made from them and what is selected.
pub(crate) struct AgentsState {
    agents: AgentList,
    is_active: bool,
    is_loading: bool,
    is_loaded: bool,
    profiles: Vec<ProfileState>,
    projects: Vec<Project>,
    selected_project: Option<PathBuf>,
    is_choosing_profile: bool,
}

impl AgentsState {
    /// The agents OTTY knows.
    pub(crate) fn agents(&self) -> &AgentList {
        &self.agents
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
        self.selected_project.as_deref()
    }

    /// Whether the user is asked which profile starts the new session.
    pub(crate) fn is_choosing_profile(&self) -> bool {
        self.is_choosing_profile
    }

    /// The state of Agents mode before anything was read.
    pub(crate) fn new(agents: AgentList) -> Self {
        Self {
            agents,
            is_active: false,
            is_loading: false,
            is_loaded: false,
            profiles: Vec::new(),
            projects: Vec::new(),
            selected_project: None,
            is_choosing_profile: false,
        }
    }

    /// The selected project, while it is in the list.
    pub(crate) fn selected_project(&self) -> Option<&Project> {
        self.project(self.selected_project.as_deref()?)
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

    /// Take the profiles that were read and return those whose sessions to
    /// read. Hidden profiles are left out. A profile that was shown before
    /// keeps its sessions until they are read again, so the lists do not
    /// go empty during a refresh. A profile whose config folder is gone
    /// has no sessions to read.
    pub(crate) fn apply_profiles(
        &mut self,
        loaded: Vec<LoadedProfile>,
    ) -> Vec<Profile> {
        let mut earlier = std::mem::take(&mut self.profiles);
        let mut to_read = Vec::new();

        for LoadedProfile { profile, status } in loaded {
            if profile.is_hidden() {
                continue;
            }

            let mut shown = ProfileState::new(profile, status);
            let position =
                earlier.iter().position(|old| old.profile == shown.profile);
            if let Some(position) = position {
                shown.take_sessions_of(earlier.swap_remove(position));
            }
            if shown.status == ProfileStatus::ConfigDirNotFound {
                shown.forget_sessions();
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

    /// Select the project of `folder` and drop the choice of a profile.
    pub(crate) fn select_project(&mut self, folder: PathBuf) {
        self.selected_project = Some(folder);
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

/// One shown profile with what was read of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileState {
    profile: Profile,
    status: ProfileStatus,
    sessions: Option<Vec<AgentSession>>,
    failures: Option<Vec<SourceFailure>>,
    missing_folders: Vec<PathBuf>,
    is_reading: bool,
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

    fn new(profile: Profile, status: ProfileStatus) -> Self {
        Self {
            profile,
            status,
            sessions: None,
            failures: None,
            missing_folders: Vec::new(),
            is_reading: false,
        }
    }

    /// Keep what was read of the same profile before.
    fn take_sessions_of(&mut self, earlier: ProfileState) {
        self.sessions = earlier.sessions;
        self.failures = earlier.failures;
        self.missing_folders = earlier.missing_folders;
    }

    fn forget_sessions(&mut self) {
        self.sessions = None;
        self.failures = None;
        self.missing_folders = Vec::new();
    }
}
