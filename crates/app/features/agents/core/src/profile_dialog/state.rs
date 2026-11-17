use std::path::{Path, PathBuf};

use otty_agents::{
    AdminKeyRef, AgentId, Dollars, Profile, ProfileInputError,
    check_profile_name,
};

use super::event::FolderChoice;
use super::folder::{FolderProblem, check_folder, full_path};
use super::money_fields::MoneyFields;
use crate::folder_suggestions::FolderSuggestions;
use crate::types::{FolderAsker, LoadedFolder};

/// The open profile dialog: what is typed and picked in it, and how its
/// save went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileDialog {
    editing: Option<Profile>,
    agent: AgentId,
    is_agent_list_open: bool,
    name: String,
    folder_choice: FolderChoice,
    typed: FolderSuggestions,
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

    /// The path text of the picked way: the typed path, or the path of
    /// the new folder.
    pub(crate) fn path_text(&self) -> &str {
        match self.folder_choice {
            FolderChoice::Typed => self.typed.text(),
            FolderChoice::Create => &self.new_path,
        }
    }

    /// The typed config folder with its suggestions, while it is picked.
    pub(crate) fn typed_folder(&self) -> Option<&FolderSuggestions> {
        (self.folder_choice == FolderChoice::Typed).then_some(&self.typed)
    }

    /// The folders the typed config folder may stand for; none while the
    /// new folder is picked.
    pub(crate) fn suggestions(&self, home: &Path) -> Vec<String> {
        self.typed_folder()
            .map(|typed| typed.suggestions(home))
            .unwrap_or_default()
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
    /// new folder.
    pub(crate) fn add(agent: AgentId) -> Self {
        Self {
            editing: None,
            agent,
            is_agent_list_open: false,
            name: String::new(),
            folder_choice: FolderChoice::Create,
            typed: FolderSuggestions::new(
                FolderAsker::ProfileDialog,
                String::new(),
            ),
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
            typed: FolderSuggestions::new(
                FolderAsker::ProfileDialog,
                profile.config_dir().display().to_string(),
            ),
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

    /// Pick another agent for the new profile: the new folder is proposed
    /// again with `proposal`.
    pub(crate) fn pick_agent(
        &mut self,
        agent: AgentId,
        proposal: Option<PathBuf>,
    ) {
        self.agent = agent;
        self.propose(proposal);
    }

    /// Take the typed name and the folder `proposal` made from it.
    pub(crate) fn set_name(&mut self, name: String, proposal: Option<PathBuf>) {
        self.name = name;
        self.propose(proposal);
    }

    /// Pick how the config folder is given; the suggestions of the typed
    /// path close.
    pub(crate) fn pick_folder_choice(&mut self, choice: FolderChoice) {
        self.folder_choice = choice;
        self.typed.close();
    }

    /// Take the typed path of the picked way. Typing the path of the new
    /// folder stops proposing it from the name.
    pub(crate) fn set_path(&mut self, text: String) {
        match self.folder_choice {
            FolderChoice::Typed => self.typed.set_text(text),
            FolderChoice::Create => {
                self.new_path = text;
                self.is_new_path_typed = true;
            },
        }
    }

    /// The typed config folder with its suggestions, to change it, while
    /// it is picked.
    pub(crate) fn typed_folder_mut(
        &mut self,
    ) -> Option<&mut FolderSuggestions> {
        (self.folder_choice == FolderChoice::Typed).then_some(&mut self.typed)
    }

    /// Take the answer of the look-up of the typed config folder; `false`
    /// when it is about an older text and is dropped. Whether the folder
    /// is there does not matter: a new profile may name a new folder.
    pub(crate) fn apply_lookup(
        &mut self,
        loaded: LoadedFolder,
        home: &Path,
    ) -> bool {
        self.typed.apply(loaded, home)
    }

    /// The admin key and the budget fields, to change them; `None` while
    /// the profile has none.
    pub(crate) fn money_mut(&mut self) -> Option<&mut MoneyFields> {
        self.money.as_mut()
    }

    /// What is wrong with the name, checked against every other profile.
    pub(crate) fn name_problem(
        &self,
        profiles: &[Profile],
    ) -> Option<ProfileInputError> {
        check_profile_name(&self.name, self.editing.as_ref(), profiles).err()
    }

    /// The config folder typed, with `~` standing for `home` and no slash
    /// at the end (a picked suggestion has one), or what is wrong with it.
    pub(crate) fn folder(
        &self,
        home: &Path,
        profiles: &[Profile],
    ) -> Result<PathBuf, FolderProblem> {
        let folder: PathBuf =
            full_path(self.path_text(), home)?.components().collect();

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
