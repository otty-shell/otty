use std::path::{Path, PathBuf};

use otty_agents::{AgentId, Profile, ProfileInputError, check_profile_name};

use super::event::FolderChoice;
use super::folder::{FolderProblem, check_folder};
use crate::event::AgentsOutcome;
use crate::folder_field::{FolderFieldIntent, FolderSuggestions, full_folder};
use crate::types::FolderAsker;

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
    error: Option<String>,
    is_saving: bool,
}

impl ProfileDialog {
    /// The profile being edited; `None` while a profile is added.
    pub(crate) fn editing(&self) -> Option<&Profile> {
        self.editing.as_ref()
    }

    /// The agent of the profile.
    pub(crate) fn agent(&self) -> &AgentId {
        &self.agent
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
            error: None,
            is_saving: false,
        }
    }

    /// The dialog that edits `profile`: its name and its config folder as
    /// a typed path.
    pub(crate) fn edit(profile: &Profile) -> Self {
        Self {
            name: profile.name().to_string(),
            folder_choice: FolderChoice::Typed,
            typed: FolderSuggestions::new(
                FolderAsker::ProfileDialog,
                profile.config_dir().display().to_string(),
            ),
            ..Self::add(profile.agent().clone())
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

    /// Do what the user did in the config folder field, with `~` standing
    /// for `home`: the typed path with its suggestions, asked about when it
    /// changed, or the path of the new folder, whichever is picked. Typing
    /// the path of the new folder stops proposing it from the name.
    pub(crate) fn change_folder(
        &mut self,
        intent: FolderFieldIntent,
        home: &Path,
    ) -> Vec<AgentsOutcome> {
        match (self.folder_choice, intent) {
            (FolderChoice::Typed, intent) => self.typed.reduce(intent, home),
            (FolderChoice::Create, FolderFieldIntent::Changed(text)) => {
                self.new_path = text;
                self.is_new_path_typed = true;
                Vec::new()
            },
            _ => Vec::new(),
        }
    }

    /// The typed config folder with its suggestions, to take the answer
    /// about it. Whether the folder is there does not matter: a new profile
    /// may name a new folder.
    pub(crate) fn typed_folder_mut(&mut self) -> &mut FolderSuggestions {
        &mut self.typed
    }

    /// What the checks of the name and the config folder find against
    /// every other profile, `profiles`, with `~` standing for `home`, and
    /// whether Add or Save can be pressed.
    pub(crate) fn checks(
        &self,
        home: &Path,
        profiles: &[&Profile],
    ) -> DialogChecks {
        let name_problem =
            check_profile_name(&self.name, self.editing.as_ref(), profiles)
                .err();
        let folder = self.folder(home, profiles);
        let can_submit =
            !self.is_saving && name_problem.is_none() && folder.is_ok();

        DialogChecks {
            name_problem,
            folder,
            can_submit,
        }
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

    /// The config folder typed, with `~` standing for `home` and no slash
    /// at the end (a picked suggestion has one), or what is wrong with it.
    fn folder(
        &self,
        home: &Path,
        profiles: &[&Profile],
    ) -> Result<PathBuf, FolderProblem> {
        let folder =
            full_folder(self.path_text(), home).map_err(FolderProblem::from)?;

        check_folder(&self.agent, &folder, self.editing.as_ref(), profiles)?;
        Ok(folder)
    }

    fn editing_profile(mut self, profile: Profile) -> Self {
        self.editing = Some(profile);
        self
    }
}

/// What the checks of the profile dialog found, once for a frame or a
/// press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DialogChecks {
    name_problem: Option<ProfileInputError>,
    folder: Result<PathBuf, FolderProblem>,
    can_submit: bool,
}

impl DialogChecks {
    /// What is wrong with the name.
    pub(crate) fn name_problem(&self) -> Option<ProfileInputError> {
        self.name_problem
    }

    /// The config folder to save, or what is wrong with it.
    pub(crate) fn folder(&self) -> Result<&Path, FolderProblem> {
        self.folder.as_deref().map_err(|problem| *problem)
    }

    /// Whether Add or Save can be pressed: the name and the folder pass
    /// their checks and no save is on its way.
    pub(crate) fn can_submit(&self) -> bool {
        self.can_submit
    }
}
