use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

use iced::{Point, Task, widget::operation};
use otty_ui_term::settings::Settings;

use crate::app::Event as AppEvent;
use crate::features::tab::{TabEvent, TabOpenRequest};
use crate::state::State;

use super::errors::QuickLaunchError;
use super::model::{
    NodePath, QuickLaunch, QuickLaunchFolder, QuickLaunchNode,
    quick_launch_error_message,
};
use super::services::prepare_quick_launch_setup;
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
    SetupCompleted(QuickLaunchSetupOutcome),
    Tick,
}

pub(crate) const QUICK_LAUNCHES_TICK_MS: u64 = 200;

#[derive(Clone)]
pub(crate) struct PreparedQuickLaunch {
    pub(crate) path: NodePath,
    pub(crate) launch_id: u64,
    pub(crate) tab_id: u64,
    pub(crate) terminal_id: u64,
    pub(crate) title: String,
    pub(crate) settings: Box<Settings>,
    pub(crate) command: Box<QuickLaunch>,
}

impl fmt::Debug for PreparedQuickLaunch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedQuickLaunch")
            .field("path", &self.path)
            .field("launch_id", &self.launch_id)
            .field("tab_id", &self.tab_id)
            .field("terminal_id", &self.terminal_id)
            .field("title", &self.title)
            .finish()
    }
}

/// Outcome of quick launch setup completion.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchSetupOutcome {
    Prepared(PreparedQuickLaunch),
    Failed {
        path: NodePath,
        launch_id: u64,
        command: Box<QuickLaunch>,
        error: Arc<QuickLaunchError>,
    },
    Canceled {
        path: NodePath,
        launch_id: u64,
    },
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
        SetupCompleted(outcome) => reduce_setup_completed(state, outcome),
        Tick => {
            if !state.quick_launches.launching.is_empty() {
                state.quick_launches.blink_nonce =
                    state.quick_launches.blink_nonce.wrapping_add(1);
            }
            Task::none()
        },
    }
}

fn reduce_setup_completed(
    state: &mut State,
    outcome: QuickLaunchSetupOutcome,
) -> Task<AppEvent> {
    match outcome {
        QuickLaunchSetupOutcome::Prepared(launch) => {
            handle_prepared_quick_launch(state, launch)
        },
        QuickLaunchSetupOutcome::Failed {
            path,
            launch_id,
            command,
            error,
        } => {
            if should_skip_launch_result(state, &path, launch_id) {
                return Task::none();
            }

            let command_title = &command.title;
            request_open_error_tab(
                format!("Failed to launch \"{command_title}\""),
                quick_launch_error_message(&command, error.as_ref()),
            )
        },
        QuickLaunchSetupOutcome::Canceled { path, launch_id } => {
            let _ = should_skip_launch_result(state, &path, launch_id);
            Task::none()
        },
    }
}

fn handle_prepared_quick_launch(
    state: &mut State,
    launch: PreparedQuickLaunch,
) -> Task<AppEvent> {
    let PreparedQuickLaunch {
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

    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::QuickLaunchCommandTerminal {
            tab_id,
            terminal_id,
            title,
            settings,
            command,
        },
    }))
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
            match parent.normalize_title(&edit.value, None) {
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
            match parent.normalize_title(&edit.value, Some(&current_title)) {
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
    state.quick_launches.mark_dirty();

    if cfg!(test) {
        return;
    }

    if let Err(err) = state.quick_launches.persist() {
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

    let tab_id = state.allocate_tab_id();
    let terminal_id = state.allocate_terminal_id();

    Task::perform(
        prepare_quick_launch_setup(
            command,
            path,
            launch_id,
            tab_id,
            terminal_id,
            terminal_settings.clone(),
            cancel,
        ),
        |outcome| AppEvent::QuickLaunchSetupCompleted(Box::new(outcome)),
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

fn open_create_command_tab(
    _state: &mut State,
    parent: NodePath,
) -> Task<AppEvent> {
    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::QuickLaunchEditorCreate {
            parent_path: parent,
        },
    }))
}

fn open_edit_command_tab(state: &mut State, path: NodePath) -> Task<AppEvent> {
    let Some(node) = state.quick_launches.data.node(&path).cloned() else {
        return Task::none();
    };
    let QuickLaunchNode::Command(command) = node else {
        return Task::none();
    };

    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::QuickLaunchEditorEdit {
            path,
            command: Box::new(command),
        },
    }))
}

fn request_open_error_tab(title: String, message: String) -> Task<AppEvent> {
    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::QuickLaunchError { title, message },
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::AtomicBool};
    use std::time::Instant;

    use otty_ui_term::settings::Settings;

    use super::*;
    use crate::state::State;

    fn settings() -> Settings {
        Settings::default()
    }

    fn sample_command() -> QuickLaunch {
        QuickLaunch {
            title: String::from("Demo"),
            spec: super::super::model::CommandSpec::Custom {
                custom: super::super::model::CustomCommand {
                    program: String::from("bash"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
        }
    }

    #[test]
    fn given_selected_folder_when_delete_selected_then_folder_is_removed() {
        let mut state = State::default();
        state
            .quick_launches
            .data
            .root
            .children
            .push(QuickLaunchNode::Folder(QuickLaunchFolder {
                title: String::from("Folder"),
                expanded: true,
                children: Vec::new(),
            }));
        state.quick_launches.selected = Some(vec![String::from("Folder")]);

        let _task = quick_launches_reducer(
            &mut state,
            &settings(),
            QuickLaunchEvent::DeleteSelected,
        );

        assert!(state.quick_launches.data.root.children.is_empty());
        assert!(state.quick_launches.selected.is_none());
    }

    #[test]
    fn given_unknown_node_when_released_then_reducer_ignores_event() {
        let mut state = State::default();

        let _task = quick_launches_reducer(
            &mut state,
            &settings(),
            QuickLaunchEvent::NodeReleased {
                path: vec![String::from("Missing")],
            },
        );

        assert!(state.quick_launches.selected.is_none());
        assert!(state.quick_launches.launching.is_empty());
    }

    #[test]
    fn given_tick_event_with_active_launch_when_reduced_then_blink_nonce_increments()
     {
        let mut state = State::default();
        state.quick_launches.blink_nonce = 10;
        state.quick_launches.launching.insert(
            vec![String::from("Demo")],
            LaunchInfo {
                id: 1,
                started_at: Instant::now(),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let _task = quick_launches_reducer(
            &mut state,
            &settings(),
            QuickLaunchEvent::Tick,
        );

        assert_eq!(state.quick_launches.blink_nonce, 11);
    }

    #[test]
    fn given_failed_setup_completion_when_reduced_then_launch_is_removed() {
        let mut state = State::default();
        let path = vec![String::from("Demo")];
        state.quick_launches.launching.insert(
            path.clone(),
            LaunchInfo {
                id: 9,
                started_at: Instant::now(),
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let outcome = QuickLaunchSetupOutcome::Failed {
            path: path.clone(),
            launch_id: 9,
            command: Box::new(sample_command()),
            error: Arc::new(QuickLaunchError::Validation {
                message: String::from("Program is empty."),
            }),
        };
        let _task = quick_launches_reducer(
            &mut state,
            &settings(),
            QuickLaunchEvent::SetupCompleted(outcome),
        );

        assert!(!state.quick_launches.launching.contains_key(&path));
    }
}
