use iced::Task;

use super::QuickLaunchWizardError;
use super::model::build_command;
use super::state::{QuickLaunchWizardEditorState, QuickLaunchWizardMode};
use crate::app::Event as AppEvent;
use crate::features::quick_launch::{
    QuickLaunch, QuickLaunchEvent, QuickLaunchType,
    QuickLaunchWizardSaveRequest, QuickLaunchWizardSaveTarget,
};
use crate::features::tab::TabEvent;
use crate::state::State;

/// Events emitted by the quick launch editor UI.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchWizardEvent {
    InitializeCreate {
        parent_path: Vec<String>,
    },
    InitializeEdit {
        path: Vec<String>,
        command: Box<QuickLaunch>,
    },
    TabClosed,
    Cancel,
    Save,
    SetError {
        message: String,
    },
    UpdateTitle(String),
    UpdateProgram(String),
    UpdateHost(String),
    UpdateUser(String),
    UpdatePort(String),
    UpdateIdentityFile(String),
    UpdateWorkingDirectory(String),
    AddArg,
    RemoveArg(usize),
    UpdateArg {
        index: usize,
        value: String,
    },
    AddEnv,
    RemoveEnv(usize),
    UpdateEnvKey {
        index: usize,
        value: String,
    },
    UpdateEnvValue {
        index: usize,
        value: String,
    },
    AddExtraArg,
    RemoveExtraArg(usize),
    UpdateExtraArg {
        index: usize,
        value: String,
    },
    SelectCommandType(QuickLaunchType),
}

/// Runtime dependencies used by quick launch editor reducer.
pub(crate) struct QuickLaunchWizardDeps {
    pub(crate) tab_id: u64,
}

/// Reduce quick launch editor events from a tab-scoped editor instance.
pub(crate) fn quick_launch_wizard_reducer(
    state: &mut State,
    deps: QuickLaunchWizardDeps,
    event: QuickLaunchWizardEvent,
) -> Task<AppEvent> {
    use QuickLaunchWizardEvent::*;

    match event {
        InitializeCreate { parent_path } => {
            state
                .quick_launch_wizard
                .initialize_create(deps.tab_id, parent_path);
            Task::none()
        },
        InitializeEdit { path, command } => {
            state.quick_launch_wizard.initialize_edit(
                deps.tab_id,
                path,
                &command,
            );
            Task::none()
        },
        TabClosed => {
            state.quick_launch_wizard.remove_tab(deps.tab_id);
            Task::none()
        },
        Cancel => Task::done(AppEvent::Tab(TabEvent::CloseTab {
            tab_id: deps.tab_id,
        })),
        Save => reduce_save(state, deps.tab_id),
        SetError { message } => {
            apply_editor_error(state, deps.tab_id, message);
            Task::none()
        },
        other => reduce_editor_fields(state, deps.tab_id, other),
    }
}

/// Apply a validation/runtime error to the tab editor, if present.
fn apply_editor_error(state: &mut State, tab_id: u64, message: String) {
    if let Some(editor) = editor_mut(state, tab_id) {
        editor.set_error(message);
    }
}

fn reduce_save(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    let request = match build_save_request(state, tab_id) {
        Ok(Some(request)) => request,
        Ok(None) => return Task::none(),
        Err(err) => {
            apply_editor_error(state, tab_id, format!("{err}"));
            return Task::none();
        },
    };

    Task::done(AppEvent::QuickLaunch(
        QuickLaunchEvent::WizardSaveRequested(request),
    ))
}

fn reduce_editor_fields(
    state: &mut State,
    tab_id: u64,
    event: QuickLaunchWizardEvent,
) -> Task<AppEvent> {
    use QuickLaunchWizardEvent::*;

    let Some(editor) = editor_mut(state, tab_id) else {
        return Task::none();
    };

    match event {
        UpdateTitle(value) => editor.set_title(value),
        UpdateProgram(value) => editor.set_program(value),
        UpdateHost(value) => editor.set_host(value),
        UpdateUser(value) => editor.set_user(value),
        UpdatePort(value) => editor.set_port(value),
        UpdateIdentityFile(value) => editor.set_identity_file(value),
        UpdateWorkingDirectory(value) => editor.set_working_directory(value),
        AddArg => editor.add_arg(),
        RemoveArg(index) => editor.remove_arg(index),
        UpdateArg { index, value } => editor.update_arg(index, value),
        AddEnv => editor.add_env(),
        RemoveEnv(index) => editor.remove_env(index),
        UpdateEnvKey { index, value } => editor.update_env_key(index, value),
        UpdateEnvValue { index, value } => {
            editor.update_env_value(index, value)
        },
        AddExtraArg => editor.add_extra_arg(),
        RemoveExtraArg(index) => editor.remove_extra_arg(index),
        UpdateExtraArg { index, value } => {
            editor.update_extra_arg(index, value)
        },
        SelectCommandType(command_type) => {
            if editor.is_create_mode() {
                editor.set_command_type(command_type);
            }
        },
        _ => {},
    }

    editor.clear_error();
    Task::none()
}

fn build_save_request(
    state: &State,
    tab_id: u64,
) -> Result<Option<QuickLaunchWizardSaveRequest>, QuickLaunchWizardError> {
    let Some(editor) = editor_ref(state, tab_id) else {
        return Ok(None);
    };

    let command = build_command(editor)?;
    let target = match editor.mode().clone() {
        QuickLaunchWizardMode::Create { parent_path } => {
            QuickLaunchWizardSaveTarget::Create { parent_path }
        },
        QuickLaunchWizardMode::Edit { path } => {
            QuickLaunchWizardSaveTarget::Edit { path }
        },
    };

    Ok(Some(QuickLaunchWizardSaveRequest {
        tab_id,
        target,
        command,
    }))
}

fn editor_ref(
    state: &State,
    tab_id: u64,
) -> Option<&QuickLaunchWizardEditorState> {
    state.quick_launch_wizard.editor(tab_id)
}

fn editor_mut(
    state: &mut State,
    tab_id: u64,
) -> Option<&mut QuickLaunchWizardEditorState> {
    state.quick_launch_wizard.editor_mut(tab_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::quick_launch::{CommandSpec, CustomCommand};

    fn deps(tab_id: u64) -> QuickLaunchWizardDeps {
        QuickLaunchWizardDeps { tab_id }
    }

    #[test]
    fn given_missing_tab_when_reducer_receives_event_then_event_is_ignored() {
        let mut state = State::default();

        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(999),
            QuickLaunchWizardEvent::UpdateTitle(String::from("x")),
        );

        assert!(state.quick_launch_wizard.editor(999).is_none());
    }

    #[test]
    fn given_custom_editor_when_field_events_are_reduced_then_state_is_updated()
    {
        let (mut state, tab_id) =
            state_with_editor(QuickLaunchWizardEditorState::new_create(vec![]));

        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::UpdateTitle(String::from("Run")),
        );
        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::UpdateProgram(String::from("cargo")),
        );
        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::AddArg,
        );
        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::UpdateArg {
                index: 0,
                value: String::from("check"),
            },
        );
        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::AddEnv,
        );
        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::UpdateEnvKey {
                index: 0,
                value: String::from("RUST_LOG"),
            },
        );
        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::UpdateEnvValue {
                index: 0,
                value: String::from("debug"),
            },
        );
        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::UpdateWorkingDirectory(String::from(
                "/tmp",
            )),
        );

        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.title(), "Run");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.program(), "cargo");
        assert_eq!(custom.args(), &[String::from("check")]);
        assert_eq!(
            custom.env(),
            &[(String::from("RUST_LOG"), String::from("debug"))],
        );
        assert_eq!(custom.working_directory(), "/tmp");
    }

    #[test]
    fn given_edit_editor_when_selecting_type_then_existing_type_is_preserved() {
        let command = QuickLaunch {
            title: String::from("Run"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("bash"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
        };
        let editor = QuickLaunchWizardEditorState::from_command(
            vec![String::from("Run")],
            &command,
        );
        let (mut state, tab_id) = state_with_editor(editor);

        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::SelectCommandType(QuickLaunchType::Ssh),
        );

        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.command_type(), QuickLaunchType::Custom);
    }

    #[test]
    fn given_invalid_editor_when_save_then_error_is_set() {
        let editor = QuickLaunchWizardEditorState::new_create(vec![]);
        let (mut state, tab_id) = state_with_editor(editor);

        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::Save,
        );

        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.error(), Some("Title is required."));
    }

    #[test]
    fn given_valid_editor_when_save_then_quick_launch_state_is_not_mutated_directly()
     {
        let mut editor = QuickLaunchWizardEditorState::new_create(vec![]);
        editor.set_title(String::from("Run"));
        editor.set_program(String::from("bash"));
        let (mut state, tab_id) = state_with_editor(editor);

        let _ = quick_launch_wizard_reducer(
            &mut state,
            deps(tab_id),
            QuickLaunchWizardEvent::Save,
        );

        assert!(state.quick_launches.data().root.children.is_empty());
        assert!(!state.quick_launches.is_dirty());
        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.error(), None);
    }

    fn state_with_editor(editor: QuickLaunchWizardEditorState) -> (State, u64) {
        let tab_id = 1;
        let mut state = State::default();
        state.quick_launch_wizard.set_editor(tab_id, editor);
        (state, tab_id)
    }
}
