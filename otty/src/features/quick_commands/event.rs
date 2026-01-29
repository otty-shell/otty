use std::collections::HashMap;

use iced::{Point, Task, widget::operation};
use otty_libterm::pty::SSHAuth;
use otty_ui_term::settings::{
    LocalSessionOptions, SSHSessionOptions, SessionKind, Settings,
};

use crate::app::Event as AppEvent;
use crate::features::quick_commands::editor::{
    open_create_editor_tab, open_edit_editor_tab,
};
use crate::features::tab::{QuickCommandErrorState, TabContent, TabItem};
use crate::features::terminal::event::create_terminal_tab_with_session;
use crate::state::State;

use super::model::{
    CommandSpec, CustomCommand, EnvVar, NodePath, QuickCommand,
    QuickCommandFolder, QuickCommandNode, SshCommand,
};
use super::state::{
    ContextMenuState, ContextMenuTarget, DragState, DropTarget, InlineEditKind,
    InlineEditState,
};

/// Events emitted by the quick commands sidebar tree.
#[derive(Debug, Clone)]
pub(crate) enum QuickCommandsEvent {
    CursorMoved { position: Point },
    HoverEntered { path: NodePath },
    HoverLeft { path: NodePath },
    NodePressed { path: NodePath },
    NodeReleased { path: NodePath },
    NodeRightClicked { path: NodePath },
    BackgroundRightClicked,
    BackgroundPressed,
    BackgroundReleased,
    HeaderCreateFolder,
    HeaderCreateCommand,
    ContextMenuAction(ContextMenuAction),
    ContextMenuDismiss,
    InlineEditChanged(String),
    InlineEditSubmit,
}

/// Actions that can be triggered from the context menu.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ContextMenuAction {
    Edit,
    Rename,
    Duplicate,
    Remove,
    Delete,
    CreateFolder,
    CreateCommand,
}

pub(crate) fn quick_commands_reducer(
    state: &mut State,
    terminal_settings: &Settings,
    event: QuickCommandsEvent,
) -> Task<AppEvent> {
    use QuickCommandsEvent::*;

    match event {
        CursorMoved { position } => {
            state.quick_commands.cursor = position;
            update_drag_state(state);
            Task::none()
        },
        HoverEntered { path } => {
            state.quick_commands.hovered = Some(path);
            update_drag_drop_target(state);
            Task::none()
        },
        HoverLeft { path } => {
            if state
                .quick_commands
                .hovered
                .as_ref()
                .map(|current| current == &path)
                .unwrap_or(false)
            {
                state.quick_commands.hovered = None;
                update_drag_drop_target(state);
            }
            Task::none()
        },
        BackgroundPressed => {
            state.quick_commands.context_menu = None;
            state.quick_commands.inline_edit = None;
            state.quick_commands.selected = None;
            Task::none()
        },
        BackgroundReleased => {
            if state
                .quick_commands
                .drag
                .as_ref()
                .map(|drag| drag.active)
                .unwrap_or(false)
            {
                state.quick_commands.drop_target = Some(DropTarget::Root);
            }
            if finish_drag(state) {
                return Task::none();
            }
            state.quick_commands.pressed = None;
            Task::none()
        },
        BackgroundRightClicked => {
            state.quick_commands.context_menu = Some(ContextMenuState {
                target: ContextMenuTarget::Background,
                cursor: state.quick_commands.cursor,
            });
            state.quick_commands.selected = None;
            Task::none()
        },
        NodeRightClicked { path } => {
            let Some(node) = state.quick_commands.data.node(&path) else {
                return Task::none();
            };
            let target = match node {
                QuickCommandNode::Folder(_) => ContextMenuTarget::Folder(path),
                QuickCommandNode::Command(_) => {
                    ContextMenuTarget::Command(path)
                },
            };
            state.quick_commands.context_menu = Some(ContextMenuState {
                target,
                cursor: state.quick_commands.cursor,
            });
            Task::none()
        },
        ContextMenuDismiss => {
            state.quick_commands.context_menu = None;
            Task::none()
        },
        HeaderCreateFolder => {
            let parent = selected_parent_path(state);
            begin_inline_create_folder(state, parent);
            focus_inline_edit(state)
        },
        HeaderCreateCommand => {
            let parent = selected_parent_path(state);
            open_create_command_tab(state, parent)
        },
        NodePressed { path } => {
            state.quick_commands.pressed = Some(path.clone());
            state.quick_commands.selected = Some(path.clone());
            state.quick_commands.drag = Some(DragState {
                source: path,
                origin: state.quick_commands.cursor,
                active: false,
            });
            Task::none()
        },
        NodeReleased { path } => {
            if finish_drag(state) {
                return Task::none();
            }
            let clicked = state
                .quick_commands
                .pressed
                .as_ref()
                .map(|pressed| pressed == &path)
                .unwrap_or(false);
            state.quick_commands.pressed = None;
            if clicked {
                return handle_node_left_click(state, terminal_settings, path);
            }
            Task::none()
        },
        InlineEditChanged(value) => {
            if let Some(edit) = state.quick_commands.inline_edit.as_mut() {
                edit.value = value;
                edit.error = None;
            }
            Task::none()
        },
        InlineEditSubmit => {
            apply_inline_edit(state);
            Task::none()
        },
        ContextMenuAction(action) => {
            handle_context_menu_action(state, terminal_settings, action)
        },
    }
}

fn handle_node_left_click(
    state: &mut State,
    terminal_settings: &Settings,
    path: NodePath,
) -> Task<AppEvent> {
    let Some(node) = state.quick_commands.data.node(&path).cloned() else {
        return Task::none();
    };

    if matches!(state.quick_commands.inline_edit.as_ref(), Some(edit) if inline_edit_matches(edit, &path))
    {
        return Task::none();
    }

    state.quick_commands.inline_edit = None;
    state.quick_commands.context_menu = None;
    state.quick_commands.selected = Some(path.clone());

    match node {
        QuickCommandNode::Folder(_) => {
            toggle_folder_expanded(state, &path);
            persist_quick_commands(state);
            Task::none()
        },
        QuickCommandNode::Command(command) => {
            launch_quick_command(state, terminal_settings, &command)
        },
    }
}

fn handle_context_menu_action(
    state: &mut State,
    _terminal_settings: &Settings,
    action: ContextMenuAction,
) -> Task<AppEvent> {
    let Some(menu) = state.quick_commands.context_menu.clone() else {
        return Task::none();
    };

    state.quick_commands.context_menu = None;

    match action {
        ContextMenuAction::Edit => match menu.target {
            ContextMenuTarget::Command(path) => {
                open_edit_command_tab(state, path)
            },
            _ => Task::none(),
        },
        ContextMenuAction::Rename => match menu.target {
            ContextMenuTarget::Command(path)
            | ContextMenuTarget::Folder(path) => {
                begin_inline_rename(state, path);
                focus_inline_edit(state)
            },
            ContextMenuTarget::Background => Task::none(),
        },
        ContextMenuAction::Duplicate => match menu.target {
            ContextMenuTarget::Command(path) => {
                duplicate_command(state, &path);
                Task::none()
            },
            _ => Task::none(),
        },
        ContextMenuAction::Remove => match menu.target {
            ContextMenuTarget::Command(path) => {
                remove_node(state, &path);
                Task::none()
            },
            _ => Task::none(),
        },
        ContextMenuAction::Delete => match menu.target {
            ContextMenuTarget::Folder(path) => {
                remove_node(state, &path);
                Task::none()
            },
            _ => Task::none(),
        },
        ContextMenuAction::CreateFolder => {
            let parent = selected_parent_path(state);
            begin_inline_create_folder(state, parent);
            focus_inline_edit(state)
        },
        ContextMenuAction::CreateCommand => {
            open_create_command_tab(state, Vec::new())
        },
    }
}

fn begin_inline_create_folder(state: &mut State, parent_path: NodePath) {
    if !parent_path.is_empty()
        && let Some(QuickCommandNode::Folder(folder)) =
            state.quick_commands.data.node_mut(&parent_path)
    {
        folder.expanded = true;
    }

    let edit = InlineEditState {
        kind: InlineEditKind::CreateFolder { parent_path },
        value: String::new(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.quick_commands.inline_edit = Some(edit);
    state.quick_commands.context_menu = None;
}

fn begin_inline_rename(state: &mut State, path: NodePath) {
    let Some(node) = state.quick_commands.data.node(&path) else {
        return;
    };

    let edit = InlineEditState {
        kind: InlineEditKind::Rename { path },
        value: node.title().to_string(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.quick_commands.inline_edit = Some(edit);
}

fn inline_edit_matches(edit: &InlineEditState, path: &[String]) -> bool {
    match &edit.kind {
        InlineEditKind::Rename { path: edit_path } => edit_path == path,
        InlineEditKind::CreateFolder { .. } => false,
    }
}

fn apply_inline_edit(state: &mut State) {
    let Some(edit) = state.quick_commands.inline_edit.take() else {
        return;
    };

    match edit.kind {
        InlineEditKind::CreateFolder { parent_path } => {
            let Some(parent) =
                state.quick_commands.data.folder_mut(&parent_path)
            else {
                return;
            };
            match validate_title(&edit.value, parent, None) {
                Ok(title) => {
                    parent.children.push(QuickCommandNode::Folder(
                        QuickCommandFolder {
                            title,
                            expanded: true,
                            children: Vec::new(),
                        },
                    ));
                    persist_quick_commands(state);
                },
                Err(message) => {
                    state.quick_commands.inline_edit = Some(InlineEditState {
                        kind: InlineEditKind::CreateFolder { parent_path },
                        value: edit.value,
                        error: Some(message),
                        id: edit.id,
                    });
                },
            }
        },
        InlineEditKind::Rename { path } => {
            let Some(parent) =
                state.quick_commands.data.parent_folder_mut(&path)
            else {
                return;
            };
            let current_title = path.last().cloned().unwrap_or_default();
            match validate_title(&edit.value, parent, Some(&current_title)) {
                Ok(title) => {
                    if let Some(node) =
                        state.quick_commands.data.node_mut(&path)
                    {
                        *node.title_mut() = title;
                        if state
                            .quick_commands
                            .selected
                            .as_ref()
                            .map(|selected| selected == &path)
                            .unwrap_or(false)
                        {
                            let mut updated = path.clone();
                            if let Some(last) = updated.last_mut() {
                                *last = node.title().to_string();
                            }
                            state.quick_commands.selected = Some(updated);
                        }
                        persist_quick_commands(state);
                    }
                },
                Err(message) => {
                    state.quick_commands.inline_edit = Some(InlineEditState {
                        kind: InlineEditKind::Rename { path },
                        value: edit.value,
                        error: Some(message),
                        id: edit.id,
                    });
                },
            }
        },
    }
}

fn validate_title(
    raw: &str,
    parent: &QuickCommandFolder,
    current: Option<&str>,
) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(String::from("Title cannot be empty."));
    }

    let conflicts = match current {
        Some(existing) => trimmed != existing && parent.contains_title(trimmed),
        None => parent.contains_title(trimmed),
    };

    if conflicts {
        return Err(String::from("Title already exists in this folder."));
    }

    Ok(trimmed.to_string())
}

fn toggle_folder_expanded(state: &mut State, path: &[String]) {
    let Some(node) = state.quick_commands.data.node_mut(path) else {
        return;
    };
    if let QuickCommandNode::Folder(folder) = node {
        folder.expanded = !folder.expanded;
    }
}

fn selected_parent_path(state: &State) -> NodePath {
    let Some(selected) = state.quick_commands.selected.as_ref() else {
        return Vec::new();
    };

    let Some(node) = state.quick_commands.data.node(selected) else {
        return Vec::new();
    };

    match node {
        QuickCommandNode::Folder(_) => selected.clone(),
        QuickCommandNode::Command(_) => {
            let mut parent = selected.clone();
            parent.pop();
            parent
        },
    }
}

fn focus_inline_edit(state: &State) -> Task<AppEvent> {
    let Some(edit) = state.quick_commands.inline_edit.as_ref() else {
        return Task::none();
    };

    operation::focus(edit.id.clone())
}

fn update_drag_state(state: &mut State) {
    let (active, source) = {
        let Some(drag) = state.quick_commands.drag.as_mut() else {
            return;
        };

        let dx = state.quick_commands.cursor.x - drag.origin.x;
        let dy = state.quick_commands.cursor.y - drag.origin.y;
        let distance_sq = dx * dx + dy * dy;
        let threshold = 4.0;
        if !drag.active && distance_sq >= threshold * threshold {
            drag.active = true;
        }

        (drag.active, drag.source.clone())
    };

    if active {
        let target = drop_target_from_hover(state);
        if can_drop(state, &source, &target) {
            state.quick_commands.drop_target = Some(target);
        } else {
            state.quick_commands.drop_target = None;
        }
    }
}

fn finish_drag(state: &mut State) -> bool {
    let Some(drag) = state.quick_commands.drag.take() else {
        return false;
    };

    if !drag.active {
        state.quick_commands.drop_target = None;
        return false;
    }

    let target = match state.quick_commands.drop_target.take() {
        Some(target) => target,
        None => return true,
    };
    state.quick_commands.pressed = None;
    move_node(state, drag.source, target);
    true
}

fn drop_target_from_hover(state: &State) -> DropTarget {
    let Some(hovered) = state.quick_commands.hovered.as_ref() else {
        return DropTarget::Root;
    };

    let Some(node) = state.quick_commands.data.node(hovered) else {
        return DropTarget::Root;
    };

    match node {
        QuickCommandNode::Folder(_) => DropTarget::Folder(hovered.clone()),
        QuickCommandNode::Command(_) => {
            let mut parent = hovered.clone();
            parent.pop();
            if parent.is_empty() {
                DropTarget::Root
            } else {
                DropTarget::Folder(parent)
            }
        },
    }
}

fn update_drag_drop_target(state: &mut State) {
    let Some(drag) = state.quick_commands.drag.as_ref() else {
        return;
    };

    if !drag.active {
        return;
    }

    let target = drop_target_from_hover(state);
    if can_drop(state, &drag.source, &target) {
        state.quick_commands.drop_target = Some(target);
    } else {
        state.quick_commands.drop_target = None;
    }
}

fn move_node(state: &mut State, source: NodePath, target: DropTarget) {
    let Some(node) = state.quick_commands.data.node(&source).cloned() else {
        return;
    };

    let target_path = match target {
        DropTarget::Root => Vec::new(),
        DropTarget::Folder(path) => path,
    };

    let Some((_, source_parent)) = source.split_last() else {
        return;
    };
    if source_parent == target_path.as_slice() {
        return;
    }

    if matches!(node, QuickCommandNode::Folder(_))
        && is_prefix(&source, &target_path)
    {
        state.quick_commands.last_error =
            Some(String::from("Cannot move a folder into itself."));
        return;
    }

    let title = source.last().cloned().unwrap_or_default();
    if let Some(target_folder) = state.quick_commands.data.folder(&target_path)
    {
        if target_folder.contains_title(&title) {
            state.quick_commands.last_error = Some(String::from(
                "Target folder already has an item with that title.",
            ));
            return;
        }
    } else {
        return;
    }

    let moved = {
        let Some(parent_folder) =
            state.quick_commands.data.parent_folder_mut(&source)
        else {
            return;
        };
        let Some(moved) = parent_folder.remove_child(&title) else {
            return;
        };
        moved
    };

    let Some(target_folder) =
        state.quick_commands.data.folder_mut(&target_path)
    else {
        return;
    };

    target_folder.children.push(moved);
    let mut new_path = target_path.clone();
    new_path.push(title);
    state.quick_commands.selected = Some(new_path);
    persist_quick_commands(state);
}

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }

    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn can_drop(state: &State, source: &[String], target: &DropTarget) -> bool {
    let Some(node) = state.quick_commands.data.node(source) else {
        return false;
    };

    let target_path = match target {
        DropTarget::Root => Vec::new(),
        DropTarget::Folder(path) => path.clone(),
    };

    if matches!(node, QuickCommandNode::Folder(_))
        && is_prefix(source, &target_path)
    {
        return false;
    }

    true
}

fn remove_node(state: &mut State, path: &[String]) {
    let Some(parent) = state.quick_commands.data.parent_folder_mut(path) else {
        return;
    };
    let Some(title) = path.last() else {
        return;
    };
    parent.remove_child(title);
    persist_quick_commands(state);
}

fn duplicate_command(state: &mut State, path: &[String]) {
    let Some(node) = state.quick_commands.data.node(path).cloned() else {
        return;
    };
    let QuickCommandNode::Command(command) = node else {
        return;
    };

    let Some(parent) = state.quick_commands.data.parent_folder_mut(path) else {
        return;
    };

    let mut clone = command.clone();
    clone.title = duplicate_title(parent, &command.title);
    parent.children.push(QuickCommandNode::Command(clone));
    persist_quick_commands(state);
}

fn duplicate_title(parent: &QuickCommandFolder, title: &str) -> String {
    let base = format!("{title} copy");
    if !parent.contains_title(&base) {
        return base;
    }

    let mut index = 0;
    loop {
        let candidate = format!("{title} copy ({index})");
        if !parent.contains_title(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn persist_quick_commands(state: &mut State) {
    state.quick_commands.mark_dirty();
    if let Err(err) = state.quick_commands.persist() {
        log::warn!("quick commands save failed: {err}");
        state.quick_commands.last_error = Some(format!("{err}"));
    }
}

fn launch_quick_command(
    state: &mut State,
    terminal_settings: &Settings,
    command: &QuickCommand,
) -> Task<AppEvent> {
    let session = match &command.spec {
        CommandSpec::Custom { custom } => {
            SessionKind::from_local_options(custom_session(custom))
        },
        CommandSpec::Ssh { ssh } => {
            SessionKind::from_ssh_options(ssh_session(ssh))
        },
    };

    create_terminal_tab_with_session(
        state,
        terminal_settings,
        &command.title,
        session,
    )
    .unwrap_or_else(|err| {
        open_error_tab(
            state,
            format!("Failed to launch \"{}\"", command.title),
            err,
        )
    })
}

fn custom_session(custom: &CustomCommand) -> LocalSessionOptions {
    let mut options = LocalSessionOptions::default()
        .with_program(&custom.program)
        .with_args(custom.args.clone());

    if !custom.env.is_empty() {
        let mut envs = HashMap::new();
        for EnvVar { key, value } in &custom.env {
            envs.insert(key.clone(), value.clone());
        }
        options = options.with_envs(envs);
    }

    if let Some(dir) = &custom.working_directory {
        options = options.with_working_directory(dir.into());
    }

    options
}

fn ssh_session(ssh: &SshCommand) -> SSHSessionOptions {
    let host = format!("{}:{}", ssh.host, ssh.port);
    let user = ssh
        .user
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .unwrap_or_default();

    let auth = ssh
        .identity_file
        .clone()
        .filter(|value| !value.trim().is_empty())
        .map(|path| SSHAuth::KeyFile {
            private_key_path: path,
            passphrase: None,
        })
        .unwrap_or_else(|| SSHAuth::Password(String::new()));

    SSHSessionOptions::default()
        .with_host(&host)
        .with_user(&user)
        .with_auth(auth)
}

fn open_create_command_tab(
    state: &mut State,
    parent: NodePath,
) -> Task<AppEvent> {
    open_create_editor_tab(state, parent)
}

fn open_edit_command_tab(state: &mut State, path: NodePath) -> Task<AppEvent> {
    let Some(node) = state.quick_commands.data.node(&path).cloned() else {
        return Task::none();
    };
    let QuickCommandNode::Command(command) = node else {
        return Task::none();
    };

    open_edit_editor_tab(state, path, &command)
}

fn open_error_tab(
    state: &mut State,
    title: String,
    message: String,
) -> Task<AppEvent> {
    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;

    state.tab_items.insert(
        tab_id,
        TabItem {
            id: tab_id,
            title: title.clone(),
            content: TabContent::QuickCommandError(QuickCommandErrorState {
                title,
                message,
            }),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}
