use std::path::{Path, PathBuf};

use otty_agents::{
    AdminKeyRef, AgentId, Dollars, Profile, ProfileInputError,
    check_profile_name,
};

use super::event::FolderChoice;
use super::folder::{FolderProblem, check_folder, full_path};
use super::money_fields::MoneyFields;

/// The open profile dialog: what is typed and picked in it, the folders
/// OTTY found, and how its save went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileDialog {
    editing: Option<Profile>,
    agent: AgentId,
    is_agent_list_open: bool,
    name: String,
    folder_choice: FolderChoice,
    is_folder_choice_picked: bool,
    found: Vec<PathBuf>,
    found_pick: Option<PathBuf>,
    typed_path: String,
    new_path: String,
    is_new_path_typed: bool,
    money: Option<MoneyFields>,
    error: Option<String>,
    is_saving: bool,
}

impl ProfileDialog {
    /// The profile being edited; `None` while a profile is added.
    pub(crate) fn editing(&self) -> Option<&Profile> {
        self.editing.as_ref()
    }

    /// The agent of the profile.
    pub(crate) fn agent(&self) -> AgentId {
        self.agent
    }

    /// Whether the list of agents is open under the agent field.
    pub(crate) fn is_agent_list_open(&self) -> bool {
        self.is_agent_list_open
    }

    /// The name as typed.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// How the config folder is given.
    pub(crate) fn folder_choice(&self) -> FolderChoice {
        self.folder_choice
    }

    /// The folders OTTY found for the agent.
    pub(crate) fn found(&self) -> &[PathBuf] {
        &self.found
    }

    /// The found folder that is picked.
    pub(crate) fn found_pick(&self) -> Option<&Path> {
        self.found_pick.as_deref()
    }

    /// The path text of the picked way: the typed path, or the path of
    /// the new folder. Empty for a found folder.
    pub(crate) fn path_text(&self) -> &str {
        match self.folder_choice {
            FolderChoice::Typed => &self.typed_path,
            FolderChoice::Create => &self.new_path,
            FolderChoice::Found => "",
        }
    }

    /// The admin key and the budget, while an API key profile is edited.
    pub(crate) fn money(&self) -> Option<&MoneyFields> {
        self.money.as_ref()
    }

    /// Why the last save failed, ready to show.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the dialog waits for its save.
    pub(crate) fn is_saving(&self) -> bool {
        self.is_saving
    }

    /// The dialog that adds a profile of `agent`. The folder starts as a
    /// new folder; a found folder is picked once the found ones come.
    pub(crate) fn add(agent: AgentId) -> Self {
        Self {
            editing: None,
            agent,
            is_agent_list_open: false,
            name: String::new(),
            folder_choice: FolderChoice::Create,
            is_folder_choice_picked: false,
            found: Vec::new(),
            found_pick: None,
            typed_path: String::new(),
            new_path: String::new(),
            is_new_path_typed: false,
            money: None,
            error: None,
            is_saving: false,
        }
    }

    /// The dialog that edits `profile`: its name, its config folder as a
    /// typed path, and the `money` fields of an API key profile.
    pub(crate) fn edit(profile: &Profile, money: Option<MoneyFields>) -> Self {
        Self {
            name: profile.name().to_string(),
            folder_choice: FolderChoice::Typed,
            is_folder_choice_picked: true,
            typed_path: profile.config_dir().display().to_string(),
            money,
            ..Self::add(profile.agent())
        }
        .editing_profile(profile.clone())
    }

    /// Open the list of agents under the agent field, or close it.
    pub(crate) fn toggle_agent_list(&mut self) {
        self.is_agent_list_open = !self.is_agent_list_open;
    }

    /// Close the list of agents.
    pub(crate) fn close_agent_list(&mut self) {
        self.is_agent_list_open = false;
    }

    /// Pick another agent for the new profile: the folders found for the
    /// former one go, and the new folder is proposed again with
    /// `proposal`.
    pub(crate) fn pick_agent(
        &mut self,
        agent: AgentId,
        proposal: Option<PathBuf>,
    ) {
        self.agent = agent;
        self.found = Vec::new();
        self.found_pick = None;
        if self.folder_choice == FolderChoice::Found {
            self.folder_choice = FolderChoice::Create;
        }
        self.propose(proposal);
    }

    /// Take the typed name and the folder `proposal` made from it.
    pub(crate) fn set_name(&mut self, name: String, proposal: Option<PathBuf>) {
        self.name = name;
        self.propose(proposal);
    }

    /// Pick how the config folder is given. A found folder can be picked
    /// only when there is one.
    pub(crate) fn pick_folder_choice(&mut self, choice: FolderChoice) {
        if choice == FolderChoice::Found && self.found.is_empty() {
            return;
        }

        self.folder_choice = choice;
        self.is_folder_choice_picked = true;
        if choice == FolderChoice::Found && self.found_pick.is_none() {
            self.found_pick = self.found.first().cloned();
        }
    }

    /// Pick one of the found folders.
    pub(crate) fn pick_found(&mut self, folder: PathBuf) {
        if !self.found.contains(&folder) {
            return;
        }

        self.folder_choice = FolderChoice::Found;
        self.is_folder_choice_picked = true;
        self.found_pick = Some(folder);
    }

    /// Take the typed path of the picked way. Typing the path of the new
    /// folder stops proposing it from the name.
    pub(crate) fn set_path(&mut self, text: String) {
        match self.folder_choice {
            FolderChoice::Typed => self.typed_path = text,
            FolderChoice::Create => {
                self.new_path = text;
                self.is_new_path_typed = true;
            },
            FolderChoice::Found => {},
        }
    }

    /// The admin key and the budget fields, to change them; `None` while
    /// the profile has none.
    pub(crate) fn money_mut(&mut self) -> Option<&mut MoneyFields> {
        self.money.as_mut()
    }

    /// Take the folders found for `agent`; those of another agent are
    /// dropped. While the user has not picked how to give the folder, the
    /// first found folder is picked.
    pub(crate) fn take_found(&mut self, agent: AgentId, folders: Vec<PathBuf>) {
        if agent != self.agent {
            return;
        }

        self.found = folders;
        self.found_pick = self.found.first().cloned();
        if !self.is_folder_choice_picked && self.found_pick.is_some() {
            self.folder_choice = FolderChoice::Found;
        }
    }

    /// What is wrong with the name, checked against every other profile.
    pub(crate) fn name_problem(
        &self,
        profiles: &[Profile],
    ) -> Option<ProfileInputError> {
        check_profile_name(&self.name, self.editing.as_ref(), profiles).err()
    }

    /// The config folder picked or typed, with `~` standing for `home`, or
    /// what is wrong with it.
    pub(crate) fn folder(
        &self,
        home: &Path,
        profiles: &[Profile],
    ) -> Result<PathBuf, FolderProblem> {
        let folder = match self.folder_choice {
            FolderChoice::Found => {
                self.found_pick.clone().ok_or(FolderProblem::Empty)?
            },
            _ => full_path(self.path_text(), home)?,
        };

        check_folder(self.agent, &folder, self.editing.as_ref(), profiles)?;
        Ok(folder)
    }

    /// The admin key and the budget to save: from the fields of an API
    /// key profile, else what the edited profile had, so an edit of
    /// another profile keeps them. An agent with no bill keeps its admin
    /// key as it was.
    pub(crate) fn money_settings(
        &self,
        home: &Path,
    ) -> (Option<AdminKeyRef>, Option<Dollars>) {
        let saved_key = self.editing.as_ref().and_then(Profile::admin_key);
        let saved_budget =
            self.editing.as_ref().and_then(Profile::monthly_budget);
        let Some(fields) = &self.money else {
            return (saved_key.cloned(), saved_budget);
        };

        let admin_key = if fields.has_admin_key() {
            fields.admin_key(home).ok().flatten()
        } else {
            saved_key.cloned()
        };

        (admin_key, fields.budget().ok().flatten())
    }

    /// Whether Add or Save can be pressed: the name, the folder and the
    /// money fields pass their checks and no save is on its way.
    pub(crate) fn can_submit(&self, home: &Path, profiles: &[Profile]) -> bool {
        !self.is_saving
            && self.name_problem(profiles).is_none()
            && self.folder(home, profiles).is_ok()
            && self.money.as_ref().is_none_or(|money| money.is_valid(home))
    }

    /// Mark the save as started.
    pub(crate) fn start_saving(&mut self) {
        self.is_saving = true;
        self.error = None;
    }

    /// The save failed: show why and let the user try again.
    pub(crate) fn fail(&mut self, error: String) {
        self.is_saving = false;
        self.error = Some(error);
    }

    /// Propose the path of the new folder from the name, while the user
    /// has not typed one.
    fn propose(&mut self, proposal: Option<PathBuf>) {
        if self.editing.is_some() || self.is_new_path_typed {
            return;
        }

        self.new_path = proposal
            .map(|path| path.display().to_string())
            .unwrap_or_default();
    }

    fn editing_profile(mut self, profile: Profile) -> Self {
        self.editing = Some(profile);
        self
    }
}
