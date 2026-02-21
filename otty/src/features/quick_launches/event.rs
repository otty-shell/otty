use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use iced::{Point, Task, widget::operation};
use otty_libterm::pty::SSHAuth;
use otty_ui_term::settings::{
    LocalSessionOptions, SSHSessionOptions, SessionKind, Settings,
};

use crate::app::Event as AppEvent;
use crate::features::quick_launches::editor::{
    open_create_editor_tab, open_edit_editor_tab,
};
use crate::features::tab::{QuickLaunchErrorState, TabContent, TabItem};
use crate::features::terminal::event::{
    focus_active_terminal, insert_terminal_tab, settings_for_session,
};
use crate::features::terminal::term::{TerminalKind, TerminalState};
use crate::state::State;

use super::domain;
use super::model::{
    CommandSpec, CustomCommand, EnvVar, NodePath, QuickLaunch,
    QuickLaunchFolder, QuickLaunchNode, SshCommand,
};
use super::state::{
    ContextMenuState, ContextMenuTarget, DragState, DropTarget, InlineEditKind,
    InlineEditState, LaunchInfo,
};

/// Events emitted by the quick launches sidebar tree.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEvent {
    CursorMoved { position: Point },
    NodeHovered { path: Option<NodePath> },
    NodePressed { path: NodePath },
    NodeReleased { path: NodePath },
    NodeRightClicked { path: NodePath },
    BackgroundRightClicked,
    BackgroundPressed,
    BackgroundReleased,
    HeaderCreateFolder,
    HeaderCreateCommand,
    DeleteSelected,
    ContextMenuAction(ContextMenuAction),
    ContextMenuDismiss,
    InlineEditChanged(String),
    InlineEditSubmit,
}

pub(crate) const QUICK_LAUNCHES_TICK_MS: u64 = 200;
const QUICK_LAUNCH_SSH_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub(crate) struct QuickLaunchContext {
    pub(crate) path: NodePath,
    pub(crate) launch_id: u64,
    pub(crate) tab_id: u64,
    pub(crate) terminal_id: u64,
    pub(crate) title: String,
    pub(crate) settings: Box<Settings>,
    pub(crate) command: Box<QuickLaunch>,
}

impl fmt::Debug for QuickLaunchContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuickLaunchContext")
            .field("path", &self.path)
            .field("launch_id", &self.launch_id)
            .field("tab_id", &self.tab_id)
            .field("terminal_id", &self.terminal_id)
            .field("title", &self.title)
            .finish()
    }
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
    Kill,
}

pub(crate) fn quick_launches_reducer(
    state: &mut State,
    terminal_settings: &Settings,
    event: QuickLaunchEvent,
) -> Task<AppEvent> {
    use QuickLaunchEvent::*;

    match event {
        CursorMoved { position } => {
            state.quick_launches.cursor = position;
            state.sidebar.cursor = position;
            update_drag_state(state);
            Task::none()
        },
        NodeHovered { path } => {
            if state.sidebar.is_resizing() {
                return Task::none();
            }
            state.quick_launches.hovered = path;
            update_drag_drop_target(state);
            Task::none()
        },
        BackgroundPressed => {
            state.quick_launches.context_menu = None;
            state.quick_launches.inline_edit = None;
            state.quick_launches.selected = None;
            Task::none()
        },
        BackgroundReleased => {
            if state
                .quick_launches
                .drag
                .as_ref()
                .map(|drag| drag.active)
                .unwrap_or(false)
            {
                state.quick_launches.drop_target = Some(DropTarget::Root);
            }
            if finish_drag(state) {
                return Task::none();
            }
            state.quick_launches.pressed = None;
            Task::none()
        },
        BackgroundRightClicked => {
            state.quick_launches.context_menu = Some(ContextMenuState {
                target: ContextMenuTarget::Background,
                cursor: state.sidebar.cursor,
            });
            state.quick_launches.selected = None;
            Task::none()
        },
        NodeRightClicked { path } => {
            let selected_path = path.clone();
            let Some(node) = state.quick_launches.data.node(&path) else {
                return Task::none();
            };
            let target = match node {
                QuickLaunchNode::Folder(_) => ContextMenuTarget::Folder(path),
                QuickLaunchNode::Command(_) => ContextMenuTarget::Command(path),
            };
            state.quick_launches.context_menu = Some(ContextMenuState {
                target,
                cursor: state.sidebar.cursor,
            });
            state.quick_launches.selected = Some(selected_path);
            Task::none()
        },
        ContextMenuDismiss => {
            state.quick_launches.context_menu = None;
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
        DeleteSelected => {
            let Some(path) = state.quick_launches.selected.clone() else {
                return Task::none();
            };
            let Some(node) = state.quick_launches.data.node(&path) else {
                return Task::none();
            };
            if matches!(node, QuickLaunchNode::Folder(_)) {
                remove_node(state, &path);
                state.quick_launches.selected = None;
            }
            Task::none()
        },
        NodePressed { path } => {
            state.quick_launches.pressed = Some(path.clone());
            state.quick_launches.selected = Some(path.clone());
            state.quick_launches.drag = Some(DragState {
                source: path,
                origin: state.quick_launches.cursor,
                active: false,
            });
            Task::none()
        },
        NodeReleased { path } => {
            if finish_drag(state) {
                return Task::none();
            }
            let clicked = state
                .quick_launches
                .pressed
                .as_ref()
                .map(|pressed| pressed == &path)
                .unwrap_or(false);
            state.quick_launches.pressed = None;
            if clicked {
                return handle_node_left_click(state, terminal_settings, path);
            }
            Task::none()
        },
        InlineEditChanged(value) => {
            if let Some(edit) = state.quick_launches.inline_edit.as_mut() {
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

pub(crate) fn handle_quick_launch(
    state: &mut State,
    launch: QuickLaunchContext,
) -> Task<AppEvent> {
    let QuickLaunchContext {
        path,
        launch_id,
        tab_id,
        terminal_id,
        title,
        settings,
        command,
    } = launch;

    if should_skip_launch_result(state, &path, launch_id) {
        return Task::none();
    }

    let (terminal, focus_task) = match TerminalState::new(
        tab_id,
        title,
        terminal_id,
        *settings,
        TerminalKind::Command,
    ) {
        Ok(result) => result,
        Err(err) => {
            let command_title = &command.title;
            return open_error_tab(
                state,
                format!("Failed to launch \"{command_title}\""),
                quick_launch_error_message(&command, &err),
            );
        },
    };
    let insert_task =
        insert_terminal_tab(state, tab_id, terminal, focus_task, false);
    let focus_active_task = focus_active_terminal(state);
    Task::batch(vec![insert_task, focus_active_task])
}

fn handle_node_left_click(
    state: &mut State,
    terminal_settings: &Settings,
    path: NodePath,
) -> Task<AppEvent> {
    let Some(node) = state.quick_launches.data.node(&path).cloned() else {
        return Task::none();
    };

    if matches!(state.quick_launches.inline_edit.as_ref(), Some(edit) if inline_edit_matches(edit, &path))
    {
        return Task::none();
    }

    state.quick_launches.inline_edit = None;
    state.quick_launches.context_menu = None;
    state.quick_launches.selected = Some(path.clone());

    match node {
        QuickLaunchNode::Folder(_) => {
            toggle_folder_expanded(state, &path);
            persist_quick_launches(state);
            Task::none()
        },
        QuickLaunchNode::Command(command) => {
            launch_quick_launch(state, terminal_settings, path, command)
        },
    }
}

fn handle_context_menu_action(
    state: &mut State,
    _terminal_settings: &Settings,
    action: ContextMenuAction,
) -> Task<AppEvent> {
    let Some(menu) = state.quick_launches.context_menu.clone() else {
        return Task::none();
    };

    state.quick_launches.context_menu = None;

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
            let parent = match menu.target {
                ContextMenuTarget::Folder(path) => path,
                ContextMenuTarget::Command(path) => {
                    let mut parent = path.clone();
                    parent.pop();
                    parent
                },
                ContextMenuTarget::Background => Vec::new(),
            };
            begin_inline_create_folder(state, parent);
            focus_inline_edit(state)
        },
        ContextMenuAction::CreateCommand => match menu.target {
            ContextMenuTarget::Folder(path) => {
                open_create_command_tab(state, path)
            },
            ContextMenuTarget::Command(path) => {
                let parent = path[..path.len().saturating_sub(1)].to_vec();
                open_create_command_tab(state, parent)
            },
            ContextMenuTarget::Background => {
                open_create_command_tab(state, Vec::new())
            },
        },
        ContextMenuAction::Kill => match menu.target {
            ContextMenuTarget::Command(path) => {
                kill_command_launch(state, &path);
                Task::none()
            },
            _ => Task::none(),
        },
    }
}

fn begin_inline_create_folder(state: &mut State, parent_path: NodePath) {
    if !parent_path.is_empty()
        && let Some(QuickLaunchNode::Folder(folder)) =
            state.quick_launches.data.node_mut(&parent_path)
    {
        folder.expanded = true;
    }

    let edit = InlineEditState {
        kind: InlineEditKind::CreateFolder { parent_path },
        value: String::new(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.quick_launches.inline_edit = Some(edit);
    state.quick_launches.context_menu = None;
}

fn begin_inline_rename(state: &mut State, path: NodePath) {
    let Some(node) = state.quick_launches.data.node(&path) else {
        return;
    };

    let edit = InlineEditState {
        kind: InlineEditKind::Rename { path },
        value: node.title().to_string(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.quick_launches.inline_edit = Some(edit);
}

fn inline_edit_matches(edit: &InlineEditState, path: &[String]) -> bool {
    match &edit.kind {
        InlineEditKind::Rename { path: edit_path } => edit_path == path,
        InlineEditKind::CreateFolder { .. } => false,
    }
}

fn apply_inline_edit(state: &mut State) {
    let Some(edit) = state.quick_launches.inline_edit.take() else {
        return;
    };

    match edit.kind {
        InlineEditKind::CreateFolder { parent_path } => {
            let Some(parent) =
                state.quick_launches.data.folder_mut(&parent_path)
            else {
                return;
            };
            match domain::normalize_title(&edit.value, parent, None) {
                Ok(title) => {
                    parent.children.push(QuickLaunchNode::Folder(
                        QuickLaunchFolder {
                            title,
                            expanded: true,
                            children: Vec::new(),
                        },
                    ));
                    persist_quick_launches(state);
                },
                Err(err) => {
                    state.quick_launches.inline_edit = Some(InlineEditState {
                        kind: InlineEditKind::CreateFolder { parent_path },
                        value: edit.value,
                        error: Some(format!("{err}")),
                        id: edit.id,
                    });
                },
            }
        },
        InlineEditKind::Rename { path } => {
            let Some(parent) =
                state.quick_launches.data.parent_folder_mut(&path)
            else {
                return;
            };
            let current_title = path.last().cloned().unwrap_or_default();
            match domain::normalize_title(
                &edit.value,
                parent,
                Some(&current_title),
            ) {
                Ok(title) => {
                    let mut renamed_path = path.clone();
                    if let Some(last) = renamed_path.last_mut() {
                        *last = title.clone();
                    }

                    if let Some(node) =
                        state.quick_launches.data.node_mut(&path)
                    {
                        *node.title_mut() = title;
                    }

                    update_launching_paths(state, &path, &renamed_path);
                    if state
                        .quick_launches
                        .selected
                        .as_ref()
                        .map(|selected| selected == &path)
                        .unwrap_or(false)
                    {
                        state.quick_launches.selected = Some(renamed_path);
                    }

                    persist_quick_launches(state);
                },
                Err(err) => {
                    state.quick_launches.inline_edit = Some(InlineEditState {
                        kind: InlineEditKind::Rename { path },
                        value: edit.value,
                        error: Some(format!("{err}")),
                        id: edit.id,
                    });
                },
            }
        },
    }
}

fn toggle_folder_expanded(state: &mut State, path: &[String]) {
    let Some(node) = state.quick_launches.data.node_mut(path) else {
        return;
    };
    if let QuickLaunchNode::Folder(folder) = node {
        folder.expanded = !folder.expanded;
    }
}

fn selected_parent_path(state: &State) -> NodePath {
    let Some(selected) = state.quick_launches.selected.as_ref() else {
        return Vec::new();
    };

    let Some(node) = state.quick_launches.data.node(selected) else {
        return Vec::new();
    };

    match node {
        QuickLaunchNode::Folder(_) => selected.clone(),
        QuickLaunchNode::Command(_) => {
            let mut parent = selected.clone();
            parent.pop();
            parent
        },
    }
}

fn focus_inline_edit(state: &State) -> Task<AppEvent> {
    let Some(edit) = state.quick_launches.inline_edit.as_ref() else {
        return Task::none();
    };

    operation::focus(edit.id.clone())
}

fn update_drag_state(state: &mut State) {
    let (active, source) = {
        let Some(drag) = state.quick_launches.drag.as_mut() else {
            return;
        };

        let dx = state.quick_launches.cursor.x - drag.origin.x;
        let dy = state.quick_launches.cursor.y - drag.origin.y;
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
            state.quick_launches.drop_target = Some(target);
        } else {
            state.quick_launches.drop_target = None;
        }
    }
}

fn finish_drag(state: &mut State) -> bool {
    let Some(drag) = state.quick_launches.drag.take() else {
        return false;
    };

    if !drag.active {
        state.quick_launches.drop_target = None;
        return false;
    }

    let target = match state.quick_launches.drop_target.take() {
        Some(target) => target,
        None => return true,
    };
    state.quick_launches.pressed = None;
    move_node(state, drag.source, target);
    true
}

fn drop_target_from_hover(state: &State) -> DropTarget {
    let Some(hovered) = state.quick_launches.hovered.as_ref() else {
        return DropTarget::Root;
    };

    let Some(node) = state.quick_launches.data.node(hovered) else {
        return DropTarget::Root;
    };

    match node {
        QuickLaunchNode::Folder(_) => DropTarget::Folder(hovered.clone()),
        QuickLaunchNode::Command(_) => {
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
    let Some(drag) = state.quick_launches.drag.as_ref() else {
        return;
    };

    if !drag.active {
        return;
    }

    let target = drop_target_from_hover(state);
    if can_drop(state, &drag.source, &target) {
        state.quick_launches.drop_target = Some(target);
    } else {
        state.quick_launches.drop_target = None;
    }
}

fn move_node(state: &mut State, source: NodePath, target: DropTarget) {
    let Some(node) = state.quick_launches.data.node(&source).cloned() else {
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

    if matches!(node, QuickLaunchNode::Folder(_))
        && is_prefix(&source, &target_path)
    {
        log::warn!(
            "quick launches move failed: cannot move folder into itself"
        );
        return;
    }

    let title = source.last().cloned().unwrap_or_default();
    if let Some(target_folder) = state.quick_launches.data.folder(&target_path)
    {
        if target_folder.contains_title(&title) {
            log::warn!(
                "quick launches move failed: target already contains title"
            );
            return;
        }
    } else {
        return;
    }

    let moved = {
        let Some(parent_folder) =
            state.quick_launches.data.parent_folder_mut(&source)
        else {
            return;
        };
        let Some(moved) = parent_folder.remove_child(&title) else {
            return;
        };
        moved
    };

    let Some(target_folder) =
        state.quick_launches.data.folder_mut(&target_path)
    else {
        return;
    };

    target_folder.children.push(moved);
    let mut new_path = target_path.clone();
    new_path.push(title);
    update_launching_paths(state, &source, &new_path);
    state.quick_launches.selected = Some(new_path);
    persist_quick_launches(state);
}

fn update_launching_paths(
    state: &mut State,
    source: &[String],
    new_path: &[String],
) {
    let mut moved: Vec<(NodePath, LaunchInfo)> = Vec::new();
    for (path, info) in &state.quick_launches.launching {
        if is_prefix(source, path) {
            moved.push((path.clone(), info.clone()));
        }
    }

    if moved.is_empty() {
        return;
    }

    for (path, _) in &moved {
        state.quick_launches.launching.remove(path);
    }

    for (old_path, info) in moved {
        let mut updated = new_path.to_vec();
        updated.extend_from_slice(&old_path[source.len()..]);
        state.quick_launches.launching.insert(updated, info);
    }
}

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }

    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn can_drop(state: &State, source: &[String], target: &DropTarget) -> bool {
    let Some(node) = state.quick_launches.data.node(source) else {
        return false;
    };

    let target_path = match target {
        DropTarget::Root => Vec::new(),
        DropTarget::Folder(path) => path.clone(),
    };

    if matches!(node, QuickLaunchNode::Folder(_))
        && is_prefix(source, &target_path)
    {
        return false;
    }

    true
}

fn remove_node(state: &mut State, path: &[String]) {
    let Some(parent) = state.quick_launches.data.parent_folder_mut(path) else {
        return;
    };
    let Some(title) = path.last() else {
        return;
    };
    parent.remove_child(title);
    persist_quick_launches(state);
}

fn kill_command_launch(state: &mut State, path: &[String]) {
    if let Some(info) = state.quick_launches.launching.get(path) {
        info.cancel.store(true, Ordering::Relaxed);
        state.quick_launches.canceled_launches.insert(info.id);
    }
}

fn remove_launch_by_id(state: &mut State, launch_id: u64) {
    let path =
        state
            .quick_launches
            .launching
            .iter()
            .find_map(|(path, info)| {
                if info.id == launch_id {
                    Some(path.clone())
                } else {
                    None
                }
            });

    if let Some(path) = path {
        state.quick_launches.launching.remove(&path);
    }
}

fn duplicate_command(state: &mut State, path: &[String]) {
    let Some(node) = state.quick_launches.data.node(path).cloned() else {
        return;
    };
    let QuickLaunchNode::Command(command) = node else {
        return;
    };

    let Some(parent) = state.quick_launches.data.parent_folder_mut(path) else {
        return;
    };

    let mut clone = command.clone();
    clone.title = duplicate_title(parent, &command.title);
    parent.children.push(QuickLaunchNode::Command(clone));
    persist_quick_launches(state);
}

fn duplicate_title(parent: &QuickLaunchFolder, title: &str) -> String {
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

fn persist_quick_launches(state: &mut State) {
    if let Err(err) = domain::persist_dirty(&mut state.quick_launches) {
        log::warn!("quick launches save failed: {err}");
    }
}

fn launch_quick_launch(
    state: &mut State,
    terminal_settings: &Settings,
    path: NodePath,
    command: QuickLaunch,
) -> Task<AppEvent> {
    if state.quick_launches.launching.contains_key(&path) {
        return Task::none();
    }

    if let Some(validation_error) = validate_quick_launch(&command) {
        let command_title = &command.title;
        return open_error_tab(
            state,
            format!("Failed to launch \"{command_title}\""),
            quick_launch_error_message(
                &command,
                &format!("Validation failed: {validation_error}"),
            ),
        );
    }

    let launch_id = state.quick_launches.next_launch_id;
    state.quick_launches.next_launch_id =
        state.quick_launches.next_launch_id.wrapping_add(1);
    let cancel = Arc::new(AtomicBool::new(false));
    state.quick_launches.launching.insert(
        path.clone(),
        LaunchInfo {
            id: launch_id,
            started_at: Instant::now(),
            cancel: cancel.clone(),
        },
    );

    let tab_id = state.next_tab_id;
    state.next_tab_id += 1;
    let terminal_id = state.next_terminal_id;
    state.next_terminal_id += 1;

    let session = command_session(&command, &cancel);
    let settings = settings_for_session(terminal_settings, session);
    let title = command.title.clone();

    Task::perform(
        async move {
            QuickLaunchContext {
                path,
                launch_id,
                tab_id,
                terminal_id,
                title,
                settings: Box::new(settings),
                command: Box::new(command),
            }
        },
        |result| AppEvent::QuickLaunchFinished(Box::new(result)),
    )
}

fn should_skip_launch_result(
    state: &mut State,
    path: &[String],
    launch_id: u64,
) -> bool {
    if state.quick_launches.canceled_launches.remove(&launch_id) {
        remove_launch_by_id(state, launch_id);
        return true;
    }

    if let Some(info) = state.quick_launches.launching.get(path)
        && info.id != launch_id
    {
        return true;
    }

    remove_launch_by_id(state, launch_id);
    false
}

fn command_session(
    command: &QuickLaunch,
    cancel: &Arc<AtomicBool>,
) -> SessionKind {
    match &command.spec {
        CommandSpec::Custom { custom } => {
            SessionKind::from_local_options(custom_session(custom))
        },
        CommandSpec::Ssh { ssh } => {
            SessionKind::from_ssh_options(ssh_session(ssh, cancel))
        },
    }
}

fn validate_quick_launch(command: &QuickLaunch) -> Option<String> {
    match &command.spec {
        CommandSpec::Custom { custom } => validate_custom_command(custom).err(),
        CommandSpec::Ssh { ssh } => validate_ssh_command(ssh).err(),
    }
}

fn validate_custom_command(custom: &CustomCommand) -> Result<(), String> {
    let program = custom.program.trim();
    if program.is_empty() {
        return Err(String::from("Program is empty."));
    }

    let _program_path = find_program_path(program)?;

    if let Some(dir) = custom.working_directory.as_deref() {
        let dir = dir.trim();
        if dir.is_empty() {
            return Err(String::from("Working directory is empty."));
        }

        let expanded = expand_tilde(dir);
        let path = Path::new(&expanded);
        if !path.exists() {
            return Err(format!("Working directory not found: {}", expanded));
        }
        if !path.is_dir() {
            return Err(format!(
                "Working directory is not a directory: {}",
                expanded
            ));
        }
    }

    Ok(())
}

fn validate_ssh_command(ssh: &SshCommand) -> Result<(), String> {
    if ssh.host.trim().is_empty() {
        return Err(String::from("Host is empty."));
    }

    if ssh.port == 0 {
        return Err(String::from("Port must be greater than 0."));
    }

    if let Some(identity) = ssh.identity_file.as_deref() {
        let identity = identity.trim();
        if !identity.is_empty() {
            let expanded = expand_tilde(identity);
            let path = Path::new(&expanded);
            if !path.exists() {
                return Err(format!("Identity file not found: {}", expanded));
            }
            if !path.is_file() {
                return Err(format!(
                    "Identity file is not a file: {}",
                    expanded
                ));
            }
        }
    }

    Ok(())
}

fn find_program_path(program: &str) -> Result<PathBuf, String> {
    let program = program.trim();
    let has_separator = program.contains('/') || program.contains('\\');
    let is_explicit = has_separator || program.starts_with('~');

    if is_explicit {
        let expanded = expand_tilde(program);
        let path = PathBuf::from(&expanded);
        return validate_program_path(&path, &expanded);
    }

    let paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    for dir in paths {
        let candidate = dir.join(program);
        if is_executable_path(&candidate) {
            return Ok(candidate);
        }
    }

    Err(format!("Program not found in PATH: {program}"))
}

fn validate_program_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    if !path.exists() {
        return Err(format!("Program not found: {label}"));
    }
    if path.is_dir() {
        return Err(format!("Program is a directory: {label}"));
    }
    if !is_executable_path(path) {
        return Err(format!("Program is not executable: {label}"));
    }
    Ok(path.to_path_buf())
}

fn is_executable_path(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| String::from("~"));
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

fn quick_launch_error_message(command: &QuickLaunch, err: &str) -> String {
    match &command.spec {
        CommandSpec::Custom { custom } => {
            let program = custom.program.as_str();
            let args = if custom.args.is_empty() {
                String::from("<none>")
            } else {
                custom.args.join(" ")
            };
            let cwd = custom
                .working_directory
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<default>");
            let env = if custom.env.is_empty() {
                String::from("<none>")
            } else {
                custom
                    .env
                    .iter()
                    .map(|entry| format!("{}={}", entry.key, entry.value))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            format!(
                "Type: Custom\nProgram: {program}\nArgs: {args}\nWorking dir: {cwd}\nEnv: {env}\nError: {err}"
            )
        },
        CommandSpec::Ssh { ssh } => {
            let host = ssh.host.as_str();
            let port = ssh.port;
            let user = ssh
                .user
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<default>");
            let identity = ssh
                .identity_file
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<default>");
            let extra_args = if ssh.extra_args.is_empty() {
                String::from("<none>")
            } else {
                ssh.extra_args.join(" ")
            };

            format!(
                "Type: SSH\nHost: {host}\nPort: {port}\nUser: {user}\nIdentity file: {identity}\nExtra args: {extra_args}\nError: {err}"
            )
        },
    }
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

fn ssh_session(
    ssh: &SshCommand,
    cancel: &Arc<AtomicBool>,
) -> SSHSessionOptions {
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
        .with_timeout(QUICK_LAUNCH_SSH_TIMEOUT)
        .with_cancel_token(cancel.clone())
}

fn open_create_command_tab(
    state: &mut State,
    parent: NodePath,
) -> Task<AppEvent> {
    open_create_editor_tab(state, parent)
}

fn open_edit_command_tab(state: &mut State, path: NodePath) -> Task<AppEvent> {
    let Some(node) = state.quick_launches.data.node(&path).cloned() else {
        return Task::none();
    };
    let QuickLaunchNode::Command(command) = node else {
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
            content: TabContent::QuickLaunchError(QuickLaunchErrorState {
                title,
                message,
            }),
        },
    );
    state.active_tab_id = Some(tab_id);

    Task::none()
}
