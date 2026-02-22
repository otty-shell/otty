use iced::Task;

use crate::app::Event as AppEvent;
use crate::features::quick_launches::{
    QuickLaunch, QuickLaunchNode, QuickLaunchType,
};
use crate::features::tab::{TabContent, TabEvent};
use crate::state::State;

use super::QuickLaunchEditorError;
use super::model::{build_command, validate_unique_title};
use super::state::{QuickLaunchEditorMode, QuickLaunchEditorState};

/// Events emitted by the quick launch editor UI.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEditorEvent {
    Cancel,
    Save,
    UpdateTitle(String),
    UpdateProgram(String),
    UpdateHost(String),
    UpdateUser(String),
    UpdatePort(String),
    UpdateIdentityFile(String),
    UpdateWorkingDirectory(String),
    AddArg,
    RemoveArg(usize),
    UpdateArg { index: usize, value: String },
    AddEnv,
    RemoveEnv(usize),
    UpdateEnvKey { index: usize, value: String },
    UpdateEnvValue { index: usize, value: String },
    AddExtraArg,
    RemoveExtraArg(usize),
    UpdateExtraArg { index: usize, value: String },
    SelectCommandType(QuickLaunchType),
}

/// Handle events from a quick launch editor tab.
pub(crate) fn quick_launch_editor_reducer(
    state: &mut State,
    tab_id: u64,
    event: QuickLaunchEditorEvent,
) -> Task<AppEvent> {
    use QuickLaunchEditorEvent::*;

    match event {
        Cancel => Task::done(AppEvent::Tab(TabEvent::CloseTab { tab_id })),
        Save => reduce_save(state, tab_id),
        other => reduce_editor_fields(state, tab_id, other),
    }
}

#[derive(Debug)]
struct SaveDraft {
    mode: QuickLaunchEditorMode,
    command: QuickLaunch,
}

fn reduce_save(state: &mut State, tab_id: u64) -> Task<AppEvent> {
    let draft = match build_save_draft(state, tab_id) {
        Ok(Some(draft)) => draft,
        Ok(None) => return Task::none(),
        Err(err) => {
            set_editor_error(state, tab_id, err);
            return Task::none();
        },
    };

    if let Err(err) = apply_save(state, draft) {
        set_editor_error(state, tab_id, err);
        return Task::none();
    }

    Task::done(AppEvent::Tab(TabEvent::CloseTab { tab_id }))
}

fn reduce_editor_fields(
    state: &mut State,
    tab_id: u64,
    event: QuickLaunchEditorEvent,
) -> Task<AppEvent> {
    use QuickLaunchEditorEvent::*;

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

fn build_save_draft(
    state: &State,
    tab_id: u64,
) -> Result<Option<SaveDraft>, QuickLaunchEditorError> {
    let Some(editor) = editor_ref(state, tab_id) else {
        return Ok(None);
    };

    let command = build_command(editor)?;
    Ok(Some(SaveDraft {
        mode: editor.mode().clone(),
        command,
    }))
}

fn apply_save(
    state: &mut State,
    draft: SaveDraft,
) -> Result<(), QuickLaunchEditorError> {
    match draft.mode {
        QuickLaunchEditorMode::Create { parent_path } => {
            let Some(parent) =
                state.quick_launches.data.folder_mut(&parent_path)
            else {
                return Err(QuickLaunchEditorError::MissingTargetFolder);
            };

            validate_unique_title(parent, draft.command.title.as_str(), None)?;
            parent
                .children
                .push(QuickLaunchNode::Command(draft.command));
        },
        QuickLaunchEditorMode::Edit { path } => {
            {
                let Some(parent) =
                    state.quick_launches.data.parent_folder_mut(&path)
                else {
                    return Err(QuickLaunchEditorError::MissingParentFolder);
                };
                let current = path.last().map(String::as_str);
                validate_unique_title(
                    parent,
                    draft.command.title.as_str(),
                    current,
                )?;
            }

            let Some(node) = state.quick_launches.data.node_mut(&path) else {
                return Err(QuickLaunchEditorError::MissingCommand);
            };
            *node = QuickLaunchNode::Command(draft.command);
        },
    }

    persist_quick_launches(state)
}

fn set_editor_error(
    state: &mut State,
    tab_id: u64,
    err: QuickLaunchEditorError,
) {
    if let Some(editor) = editor_mut(state, tab_id) {
        editor.set_error(format!("{err}"));
    }
}

fn editor_ref(state: &State, tab_id: u64) -> Option<&QuickLaunchEditorState> {
    let tab = state.tab_items().get(&tab_id)?;
    let TabContent::QuickLaunchEditor(editor) = &tab.content else {
        return None;
    };
    Some(editor.as_ref())
}

fn editor_mut(
    state: &mut State,
    tab_id: u64,
) -> Option<&mut QuickLaunchEditorState> {
    let tab = state.tab_items_mut().get_mut(&tab_id)?;
    let TabContent::QuickLaunchEditor(editor) = &mut tab.content else {
        return None;
    };
    Some(editor.as_mut())
}

fn persist_quick_launches(
    state: &mut State,
) -> Result<(), QuickLaunchEditorError> {
    state.quick_launches.mark_dirty();

    if cfg!(test) {
        Ok(())
    } else {
        if let Err(source) = state.quick_launches.persist() {
            log::warn!("quick launches save failed: {source}");
            return Err(QuickLaunchEditorError::Persist { source });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::quick_launches::{
        CommandSpec, CustomCommand, QuickLaunchFolder, QuickLaunchNode,
    };
    use crate::features::tab::TabItem;

    #[test]
    fn given_missing_tab_when_reducer_receives_event_then_event_is_ignored() {
        let mut state = State::default();

        let _ = quick_launch_editor_reducer(
            &mut state,
            999,
            QuickLaunchEditorEvent::UpdateTitle(String::from("x")),
        );

        assert!(state.tab.is_empty());
    }

    #[test]
    fn given_custom_editor_when_field_events_are_reduced_then_state_is_updated()
    {
        let (mut state, tab_id) =
            state_with_editor(QuickLaunchEditorState::new_create(vec![]));

        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateTitle(String::from("Run")),
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateProgram(String::from("cargo")),
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::AddArg,
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateArg {
                index: 0,
                value: String::from("check"),
            },
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::AddEnv,
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateEnvKey {
                index: 0,
                value: String::from("RUST_LOG"),
            },
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateEnvValue {
                index: 0,
                value: String::from("debug"),
            },
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateWorkingDirectory(String::from(
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
    fn given_create_editor_when_ssh_events_are_reduced_then_ssh_state_is_updated()
     {
        let (mut state, tab_id) =
            state_with_editor(QuickLaunchEditorState::new_create(vec![]));

        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::SelectCommandType(QuickLaunchType::Ssh),
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateHost(String::from("example.com")),
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdatePort(String::from("2200")),
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateUser(String::from("ubuntu")),
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateIdentityFile(String::from("id_rsa")),
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::AddExtraArg,
        );
        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::UpdateExtraArg {
                index: 0,
                value: String::from("-A"),
            },
        );

        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.command_type(), QuickLaunchType::Ssh);
        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.host(), "example.com");
        assert_eq!(ssh.port(), "2200");
        assert_eq!(ssh.user(), "ubuntu");
        assert_eq!(ssh.identity_file(), "id_rsa");
        assert_eq!(ssh.extra_args(), &[String::from("-A")]);
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
        let editor = QuickLaunchEditorState::from_command(
            vec![String::from("Run")],
            &command,
        );
        let (mut state, tab_id) = state_with_editor(editor);

        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::SelectCommandType(QuickLaunchType::Ssh),
        );

        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.command_type(), QuickLaunchType::Custom);
    }

    #[test]
    fn given_valid_create_editor_when_save_then_command_is_added() {
        let mut editor = QuickLaunchEditorState::new_create(vec![]);
        editor.set_title(String::from("Run"));
        editor.set_program(String::from("bash"));

        let (mut state, tab_id) = state_with_editor(editor);
        state.quick_launches.data.root = QuickLaunchFolder {
            title: String::from("Root"),
            expanded: true,
            children: Vec::new(),
        };

        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::Save,
        );

        assert_eq!(state.quick_launches.data.root.children.len(), 1);
        let QuickLaunchNode::Command(command) =
            &state.quick_launches.data.root.children[0]
        else {
            panic!("expected command node");
        };
        assert_eq!(command.title, "Run");
        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.error(), None);
    }

    #[test]
    fn given_invalid_editor_when_save_then_error_is_set() {
        let editor = QuickLaunchEditorState::new_create(vec![]);
        let (mut state, tab_id) = state_with_editor(editor);

        let _ = quick_launch_editor_reducer(
            &mut state,
            tab_id,
            QuickLaunchEditorEvent::Save,
        );

        let editor = editor_ref(&state, tab_id).expect("editor should exist");
        assert_eq!(editor.error(), Some("Title is required."));
    }

    fn state_with_editor(editor: QuickLaunchEditorState) -> (State, u64) {
        let tab_id = 1;
        let mut state = State::default();
        let _ = state.allocate_tab_id();
        let _ = state.allocate_tab_id();
        state.tab.insert(
            tab_id,
            TabItem {
                id: tab_id,
                title: String::from("Editor"),
                content: TabContent::QuickLaunchEditor(Box::new(editor)),
            },
        );
        state.tab.activate(Some(tab_id));
        (state, tab_id)
    }
}
