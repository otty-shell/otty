use iced::Task;

use crate::app::Event as AppEvent;
use crate::features::quick_commands::domain;
use crate::features::quick_commands::model::{
    CommandSpec, CustomCommand, EnvVar, NodePath, QuickCommand,
    QuickCommandFolder, QuickCommandNode, SshCommand,
};
use crate::features::tab::{TabContent, TabEvent, TabItem};
use crate::state::State;

/// Events emitted by the quick command editor UI.
#[derive(Debug, Clone)]
pub(crate) enum QuickCommandEditorEvent {
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
    SelectCommandType(QuickCommandType),
}

/// Supported quick command types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickCommandType {
    Custom,
    Ssh,
}

impl std::fmt::Display for QuickCommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            QuickCommandType::Custom => "Custom",
            QuickCommandType::Ssh => "SSH",
        };
        write!(f, "{label}")
    }
}

/// Mode for the editor tab.
#[derive(Debug, Clone)]
pub(crate) enum QuickCommandEditorMode {
    Create { parent_path: NodePath },
    Edit { path: NodePath },
}

/// Editor state stored in a tab.
#[derive(Debug, Clone)]
pub(crate) struct QuickCommandEditorState {
    pub(crate) mode: QuickCommandEditorMode,
    pub(crate) command_type: QuickCommandType,
    pub(crate) title: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) working_directory: String,
    pub(crate) host: String,
    pub(crate) port: String,
    pub(crate) user: String,
    pub(crate) identity_file: String,
    pub(crate) extra_args: Vec<String>,
    pub(crate) error: Option<String>,
}

impl QuickCommandEditorState {
    pub(crate) fn new_create(parent_path: NodePath) -> Self {
        Self {
            mode: QuickCommandEditorMode::Create { parent_path },
            command_type: QuickCommandType::Custom,
            title: String::new(),
            program: String::new(),
            args: Vec::new(),
            env: Vec::new(),
            working_directory: String::new(),
            host: String::new(),
            port: String::from("22"),
            user: String::new(),
            identity_file: String::new(),
            extra_args: Vec::new(),
            error: None,
        }
    }

    /// Build an editor state from an existing command.
    pub(crate) fn from_command(path: NodePath, command: &QuickCommand) -> Self {
        match &command.spec {
            CommandSpec::Custom { custom } => Self {
                mode: QuickCommandEditorMode::Edit { path },
                command_type: QuickCommandType::Custom,
                title: command.title.clone(),
                program: custom.program.clone(),
                args: custom.args.clone(),
                env: custom
                    .env
                    .iter()
                    .map(|EnvVar { key, value }| (key.clone(), value.clone()))
                    .collect(),
                working_directory: custom
                    .working_directory
                    .clone()
                    .unwrap_or_default(),
                host: String::new(),
                port: String::from("22"),
                user: String::new(),
                identity_file: String::new(),
                extra_args: Vec::new(),
                error: None,
            },
            CommandSpec::Ssh { ssh } => Self {
                mode: QuickCommandEditorMode::Edit { path },
                command_type: QuickCommandType::Ssh,
                title: command.title.clone(),
                program: String::new(),
                args: Vec::new(),
                env: Vec::new(),
                working_directory: String::new(),
                host: ssh.host.clone(),
                port: ssh.port.to_string(),
                user: ssh.user.clone().unwrap_or_default(),
                identity_file: ssh.identity_file.clone().unwrap_or_default(),
                extra_args: ssh.extra_args.clone(),
                error: None,
            },
        }
    }
}

/// Handle events from a quick command editor tab.
pub(crate) fn quick_command_editor_reducer(
    state: &mut State,
    tab_id: u64,
    event: QuickCommandEditorEvent,
) -> Task<AppEvent> {
    use QuickCommandEditorEvent::*;

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
        UpdateProgram(value) => editor.program = value,
        UpdateHost(value) => editor.host = value,
        UpdateUser(value) => editor.user = value,
        UpdatePort(value) => editor.port = value,
        UpdateIdentityFile(value) => editor.identity_file = value,
        UpdateWorkingDirectory(value) => editor.working_directory = value,
        AddArg => editor.args.push(String::new()),
        RemoveArg(index) => {
            if index < editor.args.len() {
                editor.args.remove(index);
            }
        },
        UpdateArg { index, value } => {
            if let Some(arg) = editor.args.get_mut(index) {
                *arg = value;
            }
        },
        AddEnv => editor.env.push((String::new(), String::new())),
        RemoveEnv(index) => {
            if index < editor.env.len() {
                editor.env.remove(index);
            }
        },
        UpdateEnvKey { index, value } => {
            if let Some(pair) = editor.env.get_mut(index) {
                pair.0 = value;
            }
        },
        UpdateEnvValue { index, value } => {
            if let Some(pair) = editor.env.get_mut(index) {
                pair.1 = value;
            }
        },
        AddExtraArg => editor.extra_args.push(String::new()),
        RemoveExtraArg(index) => {
            if index < editor.extra_args.len() {
                editor.extra_args.remove(index);
            }
        },
        UpdateExtraArg { index, value } => {
            if let Some(arg) = editor.extra_args.get_mut(index) {
                *arg = value;
            }
        },
        SelectCommandType(command_type) => {
            if matches!(editor.mode, QuickCommandEditorMode::Create { .. }) {
                editor.command_type = command_type;
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

    let editor = QuickCommandEditorState::new_create(parent_path);
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: String::from("Create command"),
            content: TabContent::QuickCommandEditor(Box::new(editor)),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}

/// Open a new tab for editing the provided command.
pub(crate) fn open_edit_editor_tab(
    state: &mut State,
    path: NodePath,
    command: &QuickCommand,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    let title = format!("Edit {title}", title = command.title);
    let editor = QuickCommandEditorState::from_command(path, command);
    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title,
            content: TabContent::QuickCommandEditor(Box::new(editor)),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}

fn editor_mut(
    state: &mut State,
    tab_id: u64,
) -> Option<&mut QuickCommandEditorState> {
    let tab = state.tab_items.get_mut(&tab_id)?;
    let TabContent::QuickCommandEditor(editor) = &mut tab.content else {
        return None;
    };
    Some(editor.as_mut())
}

fn apply_save(
    state: &mut State,
    draft: QuickCommandEditorState,
) -> Result<(), String> {
    let command = build_command(&draft)?;

    match &draft.mode {
        QuickCommandEditorMode::Create { parent_path } => {
            let Some(parent) =
                state.quick_commands.data.folder_mut(parent_path)
            else {
                return Err(String::from("Missing target folder."));
            };
            validate_unique_title(parent, &command.title, None)?;
            parent.children.push(QuickCommandNode::Command(command));
        },
        QuickCommandEditorMode::Edit { path } => {
            let Some(parent) =
                state.quick_commands.data.parent_folder_mut(path)
            else {
                return Err(String::from("Missing parent folder."));
            };
            let current = path.last().map(String::as_str);
            validate_unique_title(parent, &command.title, current)?;
            if let Some(node) = state.quick_commands.data.node_mut(path) {
                *node = QuickCommandNode::Command(command);
            } else {
                return Err(String::from("Command no longer exists."));
            }
        },
    }

    persist_quick_commands(state)?;
    Ok(())
}

fn build_command(
    editor: &QuickCommandEditorState,
) -> Result<QuickCommand, String> {
    let title = editor.title.trim();
    if title.is_empty() {
        return Err(String::from("Title is required."));
    }

    let spec = match editor.command_type {
        QuickCommandType::Custom => {
            let program = editor.program.trim();
            if program.is_empty() {
                return Err(String::from("Program is required."));
            }

            let env = editor
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

            let working_directory = editor.working_directory.trim().to_string();

            CommandSpec::Custom {
                custom: CustomCommand {
                    program: program.to_string(),
                    args: editor.args.clone(),
                    env,
                    working_directory: if working_directory.is_empty() {
                        None
                    } else {
                        Some(working_directory)
                    },
                },
            }
        },
        QuickCommandType::Ssh => {
            let host = editor.host.trim();
            if host.is_empty() {
                return Err(String::from("Host is required."));
            }
            let port_raw = editor.port.trim();
            let port = if port_raw.is_empty() {
                22
            } else {
                port_raw
                    .parse::<u16>()
                    .map_err(|_| String::from("Port must be a number."))?
            };

            CommandSpec::Ssh {
                ssh: SshCommand {
                    host: host.to_string(),
                    port,
                    user: optional_string(&editor.user),
                    identity_file: optional_string(&editor.identity_file),
                    extra_args: editor.extra_args.clone(),
                },
            }
        },
    };

    Ok(QuickCommand {
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
    parent: &QuickCommandFolder,
    title: &str,
    current: Option<&str>,
) -> Result<(), String> {
    domain::normalize_title(title, parent, current)
        .map(|_| ())
        .map_err(|err| format!("{err}"))
}

fn persist_quick_commands(state: &mut State) -> Result<(), String> {
    if let Err(err) = domain::persist_dirty(&mut state.quick_commands) {
        log::warn!("quick commands save failed: {err}");
        return Err(format!("{err}"));
    }
    Ok(())
}
