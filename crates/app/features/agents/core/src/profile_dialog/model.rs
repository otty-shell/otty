//! View model of the open profile dialog: its fields, the checks shown next
//! to them and its buttons.

use std::fmt;

use otty_agents::{AgentId, ProfileInputError};
use otty_app_i18n::{self as i18n, Key};

use super::event::FolderChoice;
use super::folder::FolderProblem;
use crate::folder_field::FolderSuggestions;
use crate::state::AgentsState;

/// The open profile dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileDialogViewModel<'a> {
    /// "Add profile" or "Edit profile".
    pub(crate) title: &'static str,
    /// The agents to pick from; empty while a profile is edited.
    pub(crate) agents: Vec<AgentOption>,
    /// Whether the list of agents is open under the agent field.
    pub(crate) is_agent_list_open: bool,
    /// The picked agent.
    pub(crate) agent: Option<AgentOption>,
    pub(crate) name: &'a str,
    /// What is wrong with the name, beyond being empty.
    pub(crate) name_error: Option<&'static str>,
    /// How the folder is given; `None` while a profile is edited, which
    /// has only the path.
    pub(crate) folder_choice: Option<FolderChoice>,
    /// The typed path, or the path of the new folder.
    pub(crate) path: &'a str,
    /// What is wrong with the folder, beyond having no path.
    pub(crate) path_error: Option<&'static str>,
    /// The line under the path: OTTY creates the new folder, or the typed
    /// folder must already hold the agent's files.
    pub(crate) path_hint: Option<&'static str>,
    /// The folders the typed path may stand for, shown under the field;
    /// none for the new folder.
    pub(crate) suggestions: Vec<String>,
    /// The suggestion highlighted from the keyboard.
    pub(crate) highlighted: Option<usize>,
    /// Why the last save failed.
    pub(crate) error: Option<&'a str>,
    pub(crate) submit_label: &'static str,
    /// Whether Add or Save can be pressed.
    pub(crate) can_submit: bool,
}

impl<'a> ProfileDialogViewModel<'a> {
    /// Build the open profile dialog; `None` when none is open.
    pub(crate) fn from_state(state: &'a AgentsState) -> Option<Self> {
        let dialog = state.dialog()?;
        let home = state.env().home();
        let checks = dialog.checks(
            home,
            &state.profiles().all(),
            state.plugins_folder(),
        );
        let is_adding = dialog.editing().is_none();

        let agent_options: Vec<AgentOption> = state
            .agents()
            .iter()
            .map(|agent| {
                let id = agent.entry().id();
                let entry = state.registry().agent(id);
                AgentOption {
                    id: id.clone(),
                    name: state.agent_name(id).to_string(),
                    provider: entry
                        .map(|entry| entry.provider_name().to_string())
                        .unwrap_or_default(),
                }
            })
            .collect();
        let agent = agent_options
            .iter()
            .find(|option| option.id == *dialog.agent())
            .cloned();
        let title = match dialog.editing() {
            Some(_) => i18n::t(Key::AgentsEditProfileTitle),
            None => i18n::t(Key::AgentsAddProfile),
        };
        let name_error = match checks.name_problem() {
            Some(ProfileInputError::NameTaken) => {
                Some(i18n::t(Key::AgentsErrorNameTaken))
            },
            _ => None,
        };
        let path_error = match checks.folder() {
            Err(FolderProblem::NotFull) => {
                Some(i18n::t(Key::AgentsErrorFolderNotFull))
            },
            Err(FolderProblem::Taken) => {
                Some(i18n::t(Key::AgentsErrorFolderTaken))
            },
            Err(FolderProblem::OwnLogin) => {
                Some(i18n::t(Key::AgentsErrorFolderIsOwnLogin))
            },
            _ => None,
        };
        let path_hint = match dialog.folder_choice() {
            FolderChoice::Create if is_adding => {
                Some(i18n::t(Key::AgentsCreateFolderHint))
            },
            FolderChoice::Typed if is_adding => {
                Some(i18n::t(Key::AgentsTypedFolderHint))
            },
            _ => None,
        };
        let submit = if is_adding {
            Key::ButtonAdd
        } else {
            Key::ButtonSave
        };

        Some(Self {
            title,
            agents: if is_adding { agent_options } else { Vec::new() },
            is_agent_list_open: dialog.is_agent_list_open(),
            agent,
            name: dialog.name(),
            name_error,
            folder_choice: is_adding.then(|| dialog.folder_choice()),
            path: dialog.path_text(),
            path_error,
            path_hint,
            suggestions: dialog.suggestions(home),
            highlighted: dialog
                .typed_folder()
                .and_then(FolderSuggestions::highlighted),
            error: dialog.error(),
            submit_label: i18n::t(submit),
            can_submit: checks.can_submit(),
        })
    }
}

/// An agent to pick in the dialog, shown by its name with the provider of
/// its models as the note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentOption {
    pub(crate) id: AgentId,
    pub(crate) name: String,
    pub(crate) provider: String,
}

impl fmt::Display for AgentOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// A way to give the folder, shown by its word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FolderChoiceOption(pub(crate) FolderChoice);

impl fmt::Display for FolderChoiceOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self.0 {
            FolderChoice::Typed => Key::AgentsFolderTyped,
            FolderChoice::Create => Key::AgentsFolderCreate,
        };

        f.write_str(i18n::t(key))
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use otty_agents::SavedProfiles;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::folder_field::FolderFieldIntent;
    use crate::profile_dialog::ProfileDialogIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        PLUGINS_FOLDER, claude, claude_id, codex, connected, folder_looked_up,
        folder_query, with_saved_profiles,
    };
    use crate::types::FolderAsker;

    fn open_add() -> AgentsState {
        let mut state = with_saved_profiles(
            &[(claude(), connected()), (codex(), connected())],
            SavedProfiles::default(),
        )
        .with_plugins_folder(PathBuf::from(PLUGINS_FOLDER));
        let _ = reduce(&mut state, AgentsIntent::OpenAddProfile);
        state
    }

    fn send(state: &mut AgentsState, intent: ProfileDialogIntent) {
        let _ = reduce(state, AgentsIntent::Dialog(intent));
    }

    #[test]
    fn the_add_dialog_offers_the_agents_and_a_new_folder() {
        let state = open_add();

        let vm = ProfileDialogViewModel::from_state(&state).expect("open");

        assert_eq!(vm.title, i18n::t(Key::AgentsAddProfile));
        let names: Vec<_> =
            vm.agents.iter().map(|agent| agent.name.as_str()).collect();
        assert_eq!(names, ["Claude Code", "Codex"]);
        let providers: Vec<_> = vm
            .agents
            .iter()
            .map(|agent| agent.provider.as_str())
            .collect();
        assert_eq!(providers, ["Anthropic", "OpenAI"]);
        assert!(!vm.is_agent_list_open);
        assert_eq!(vm.agent.map(|agent| agent.id), Some(claude_id()));
        assert_eq!(vm.folder_choice, Some(FolderChoice::Create));
        assert_eq!(vm.path_hint, Some(i18n::t(Key::AgentsCreateFolderHint)));
        assert_eq!(vm.submit_label, i18n::t(Key::ButtonAdd));
        assert!(!vm.can_submit);
        assert_eq!(vm.name_error, None, "an empty name only blocks Add");
    }

    #[test]
    fn the_checks_show_next_to_their_fields() {
        let mut state = open_add();
        send(
            &mut state,
            ProfileDialogIntent::NameChanged(String::from("Codex")),
        );
        send(
            &mut state,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
        );
        send(
            &mut state,
            ProfileDialogIntent::Folder(FolderFieldIntent::Changed(
                String::from("work"),
            )),
        );
        let relative =
            ProfileDialogViewModel::from_state(&state).map(|vm| vm.path_error);
        send(
            &mut state,
            ProfileDialogIntent::Folder(FolderFieldIntent::Changed(
                String::from("~/.claude"),
            )),
        );

        let vm = ProfileDialogViewModel::from_state(&state).expect("open");

        assert_eq!(vm.name_error, Some(i18n::t(Key::AgentsErrorNameTaken)));
        assert_eq!(
            relative,
            Some(Some(i18n::t(Key::AgentsErrorFolderNotFull)))
        );
        assert_eq!(vm.path_error, Some(i18n::t(Key::AgentsErrorFolderTaken)));
        assert_eq!(vm.path_hint, Some(i18n::t(Key::AgentsTypedFolderHint)));
        assert!(!vm.can_submit);
    }

    /// Issue 13 of spec 2026-11-23-accounts-apart-from-profiles: Remove of
    /// an own login deletes its folder, so a folder inside
    /// `<plugins folder>/*/accounts/` can not be a profile, added or
    /// edited.
    #[test]
    fn a_folder_inside_an_own_login_folder_can_not_be_a_profile() {
        let own = "~/.local/share/otty/plugins/openai/accounts/6f0c1e2a";
        let mut adding = open_add();
        send(
            &mut adding,
            ProfileDialogIntent::NameChanged(String::from("Codex · Own")),
        );
        send(
            &mut adding,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
        );
        send(
            &mut adding,
            ProfileDialogIntent::Folder(FolderFieldIntent::Changed(
                own.to_string(),
            )),
        );
        let mut editing = open_add();
        send(&mut editing, ProfileDialogIntent::Cancelled);
        let _ = reduce(&mut editing, AgentsIntent::EditProfile(codex()));
        send(
            &mut editing,
            ProfileDialogIntent::Folder(FolderFieldIntent::Changed(format!(
                "{own}/.codex"
            ))),
        );

        let pressed = reduce(
            &mut adding,
            AgentsIntent::Dialog(ProfileDialogIntent::Submitted),
        );

        for state in [&adding, &editing] {
            let vm = ProfileDialogViewModel::from_state(state).expect("open");
            assert_eq!(
                vm.path_error,
                Some(i18n::t(Key::AgentsErrorFolderIsOwnLogin))
            );
            assert!(!vm.can_submit);
        }
        assert_eq!(
            i18n::t(Key::AgentsErrorFolderIsOwnLogin),
            "This is a private login folder of OTTY, it can not be a profile."
        );
        assert!(pressed.is_empty(), "{pressed:?}");
    }

    #[test]
    fn the_edit_dialog_has_the_path_and_saves() {
        let mut state = open_add();
        send(&mut state, ProfileDialogIntent::Cancelled);
        let _ = reduce(&mut state, AgentsIntent::EditProfile(codex()));

        let vm = ProfileDialogViewModel::from_state(&state).expect("open");

        assert_eq!(vm.title, i18n::t(Key::AgentsEditProfileTitle));
        assert!(vm.agents.is_empty());
        assert_eq!(vm.folder_choice, None);
        assert_eq!(vm.path, "/home/user/.codex");
        assert_eq!(vm.submit_label, i18n::t(Key::ButtonSave));
        assert!(vm.can_submit);
        assert_eq!(
            FolderChoiceOption(FolderChoice::Typed).to_string(),
            i18n::t(Key::AgentsFolderTyped)
        );
    }

    /// Issue 38: the typed config folder shows its suggestions, and the
    /// answer that it is no folder is no error: a new profile may name a
    /// folder that is not there yet.
    #[test]
    fn the_typed_folder_shows_its_suggestions_and_a_missing_one_is_no_error() {
        let mut state = open_add();
        send(
            &mut state,
            ProfileDialogIntent::NameChanged(String::from("Claude Code · C")),
        );
        send(
            &mut state,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
        );
        send(
            &mut state,
            ProfileDialogIntent::Folder(FolderFieldIntent::Changed(
                String::from("~/.c"),
            )),
        );
        let _ = reduce(
            &mut state,
            folder_looked_up(
                folder_query(
                    FolderAsker::ProfileDialog,
                    "/home/user/.c",
                    "/home/user",
                    ".c",
                ),
                false,
                &["/home/user/.claude-work"],
            ),
        );

        let vm = ProfileDialogViewModel::from_state(&state).expect("open");

        assert_eq!(vm.suggestions, vec![String::from("~/.claude-work/")]);
        assert_eq!(vm.path_error, None);
        assert!(vm.can_submit);
    }
}
