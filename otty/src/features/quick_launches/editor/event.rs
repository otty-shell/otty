use iced::Task;

use crate::app::Event as AppEvent;
use crate::features::quick_launches::model::{
    CommandSpec, CustomCommand, EnvVar, NodePath, QuickLaunch,
    QuickLaunchFolder, QuickLaunchNode, QuickLaunchType, SSH_DEFAULT_PORT,
    SshCommand,
};
use crate::features::tab::{TabContent, TabEvent, TabItem};
use crate::state::State;

use super::model::{QuickLaunchEditorMode, QuickLaunchEditorState};

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

    let Some(editor) = editor_mut(state, tab_id) else {
        return Task::none();
    };

    match event {
        Cancel => {
            return Task::done(AppEvent::Tab(TabEvent::CloseTab { tab_id }));
        },
        Save => {
            let draft = editor.clone();
            let save_result = apply_save(state, draft);
            if let Err(message) = save_result {
                if let Some(editor) = editor_mut(state, tab_id) {
                    editor.error = Some(message);
                }
                return Task::none();
            }
            return Task::done(AppEvent::Tab(TabEvent::CloseTab { tab_id }));
        },
        UpdateTitle(value) => editor.title = value,
        UpdateProgram(value) => {
            if let Some(custom) = editor.custom_mut() {
                custom.program = value;
            }
        },
        UpdateHost(value) => {
            if let Some(ssh) = editor.ssh_mut() {
                ssh.host = value;
            }
        },
        UpdateUser(value) => {
            if let Some(ssh) = editor.ssh_mut() {
                ssh.user = value;
            }
        },
        UpdatePort(value) => {
            if let Some(ssh) = editor.ssh_mut() {
                ssh.port = value;
            }
        },
        UpdateIdentityFile(value) => {
            if let Some(ssh) = editor.ssh_mut() {
                ssh.identity_file = value;
            }
        },
        UpdateWorkingDirectory(value) => {
            if let Some(custom) = editor.custom_mut() {
                custom.working_directory = value;
            }
        },
        AddArg => {
            if let Some(custom) = editor.custom_mut() {
                custom.args.push(String::new());
            }
        },
        RemoveArg(index) => {
            if let Some(custom) = editor.custom_mut()
                && index < custom.args.len()
            {
                custom.args.remove(index);
            }
        },
        UpdateArg { index, value } => {
            if let Some(custom) = editor.custom_mut()
                && let Some(arg) = custom.args.get_mut(index)
            {
                *arg = value;
            }
        },
        AddEnv => {
            if let Some(custom) = editor.custom_mut() {
                custom.env.push((String::new(), String::new()));
            }
        },
        RemoveEnv(index) => {
            if let Some(custom) = editor.custom_mut()
                && index < custom.env.len()
            {
                custom.env.remove(index);
            }
        },
        UpdateEnvKey { index, value } => {
            if let Some(custom) = editor.custom_mut()
                && let Some(pair) = custom.env.get_mut(index)
            {
                pair.0 = value;
            }
        },
        UpdateEnvValue { index, value } => {
            if let Some(custom) = editor.custom_mut()
                && let Some(pair) = custom.env.get_mut(index)
            {
                pair.1 = value;
            }
        },
        AddExtraArg => {
            if let Some(ssh) = editor.ssh_mut() {
                ssh.extra_args.push(String::new());
            }
        },
        RemoveExtraArg(index) => {
            if let Some(ssh) = editor.ssh_mut()
                && index < ssh.extra_args.len()
            {
                ssh.extra_args.remove(index);
            }
        },
        UpdateExtraArg { index, value } => {
            if let Some(ssh) = editor.ssh_mut()
                && let Some(arg) = ssh.extra_args.get_mut(index)
            {
                *arg = value;
            }
        },
        SelectCommandType(command_type) => {
            if matches!(editor.mode, QuickLaunchEditorMode::Create { .. }) {
                editor.set_command_type(command_type);
            }
        },
    }

    editor.error = None;
    Task::none()
}

/// Open a new tab for creating a command under the provided parent path.
pub(crate) fn open_create_editor_tab(
    state: &mut State,
    parent_path: NodePath,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let editor = QuickLaunchEditorState::new_create(parent_path);
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: String::from("Create launch"),
            content: TabContent::QuickLaunchEditor(Box::new(editor)),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}

/// Open a new tab for editing the provided command.
pub(crate) fn open_edit_editor_tab(
    state: &mut State,
    path: NodePath,
    command: &QuickLaunch,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let command_title = &command.title;
    let title = format!("Edit {command_title}");
    let editor = QuickLaunchEditorState::from_command(path, command);
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title,
            content: TabContent::QuickLaunchEditor(Box::new(editor)),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}

fn editor_mut(
    state: &mut State,
    tab_id: u64,
) -> Option<&mut QuickLaunchEditorState> {
    let tab = state.tab_items.get_mut(&tab_id)?;
    let TabContent::QuickLaunchEditor(editor) = &mut tab.content else {
        return None;
    };
    Some(editor.as_mut())
}

fn apply_save(
    state: &mut State,
    draft: QuickLaunchEditorState,
) -> Result<(), String> {
    let command = build_command(&draft)?;

    match &draft.mode {
        QuickLaunchEditorMode::Create { parent_path } => {
            let Some(parent) =
                state.quick_launches.data.folder_mut(parent_path)
            else {
                return Err(String::from("Missing target folder."));
            };
            validate_unique_title(parent, &command.title, None)?;
            parent.children.push(QuickLaunchNode::Command(command));
        },
        QuickLaunchEditorMode::Edit { path } => {
            let Some(parent) =
                state.quick_launches.data.parent_folder_mut(path)
            else {
                return Err(String::from("Missing parent folder."));
            };
            let current = path.last().map(String::as_str);
            validate_unique_title(parent, &command.title, current)?;
            if let Some(node) = state.quick_launches.data.node_mut(path) {
                *node = QuickLaunchNode::Command(command);
            } else {
                return Err(String::from("Command no longer exists."));
            }
        },
    }

    persist_quick_launches(state)?;
    Ok(())
}

fn build_command(
    editor: &QuickLaunchEditorState,
) -> Result<QuickLaunch, String> {
    let title = editor.title.trim();
    if title.is_empty() {
        return Err(String::from("Title is required."));
    }

    let spec = match editor.command_type() {
        QuickLaunchType::Custom => {
            let Some(custom) = editor.custom() else {
                return Err(String::from("Custom command draft is missing."));
            };
            let program = custom.program.trim();
            if program.is_empty() {
                return Err(String::from("Program is required."));
            }

            let env = custom
                .env
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.trim();
                    if key.is_empty() {
                        return None;
                    }
                    Some(EnvVar {
                        key: key.to_string(),
                        value: value.clone(),
                    })
                })
                .collect::<Vec<_>>();

            let working_directory = custom.working_directory.trim().to_string();

            CommandSpec::Custom {
                custom: CustomCommand {
                    program: program.to_string(),
                    args: custom.args.clone(),
                    env,
                    working_directory: if working_directory.is_empty() {
                        None
                    } else {
                        Some(working_directory)
                    },
                },
            }
        },
        QuickLaunchType::Ssh => {
            let Some(ssh) = editor.ssh() else {
                return Err(String::from("SSH command draft is missing."));
            };
            let host = ssh.host.trim();
            if host.is_empty() {
                return Err(String::from("Host is required."));
            }
            let port_raw = ssh.port.trim();
            let port = if port_raw.is_empty() {
                SSH_DEFAULT_PORT
            } else {
                port_raw
                    .parse::<u16>()
                    .map_err(|_| String::from("Port must be a number."))?
            };

            CommandSpec::Ssh {
                ssh: SshCommand {
                    host: host.to_string(),
                    port,
                    user: optional_string(&ssh.user),
                    identity_file: optional_string(&ssh.identity_file),
                    extra_args: ssh.extra_args.clone(),
                },
            }
        },
    };

    Ok(QuickLaunch {
        title: title.to_string(),
        spec,
    })
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn validate_unique_title(
    parent: &QuickLaunchFolder,
    title: &str,
    current: Option<&str>,
) -> Result<(), String> {
    parent
        .normalize_title(title, current)
        .map(|_| ())
        .map_err(|err| format!("{err}"))
}

fn persist_quick_launches(state: &mut State) -> Result<(), String> {
    state.quick_launches.mark_dirty();
    if let Err(err) = state.quick_launches.persist() {
        log::warn!("quick launches save failed: {err}");
        return Err(format!("{err}"));
    }
    Ok(())
}
