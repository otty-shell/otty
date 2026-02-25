use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::widget::operation;
use iced::{Point, Task};
use otty_ui_term::settings::Settings;

#[cfg(test)]
use super::errors::QuickLaunchError;
use super::model::{
    NodePath, PreparedQuickLaunch, QuickLaunch, QuickLaunchFile,
    QuickLaunchFolder, QuickLaunchNode, QuickLaunchSetupOutcome,
    QuickLaunchWizardSaveRequest, QuickLaunchWizardSaveTarget,
    quick_launch_error_message,
};
use super::services::prepare_quick_launch_setup;
use super::state::{
    ContextMenuState, ContextMenuTarget, DragState, DropTarget, InlineEditKind,
    InlineEditState, LaunchInfo, QuickLaunchErrorState,
};
use super::storage::save_quick_launches;
use crate::app::Event as AppEvent;
use crate::features::quick_launch::model::ContextMenuAction;
use crate::features::quick_launch_wizard::QuickLaunchWizardEvent;
use crate::features::tab::{TabEvent, TabOpenRequest};
use crate::state::State;

/// Events emitted by the quick launches sidebar tree.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEvent {
    OpenErrorTab {
        tab_id: u64,
        title: String,
        message: String,
    },
    TabClosed {
        tab_id: u64,
    },
    CursorMoved {
        position: Point,
    },
    NodeHovered {
        path: Option<NodePath>,
    },
    NodePressed {
        path: NodePath,
    },
    NodeReleased {
        path: NodePath,
    },
    NodeRightClicked {
        path: NodePath,
    },
    BackgroundRightClicked,
    BackgroundPressed,
    BackgroundReleased,
    HeaderCreateFolder,
    HeaderCreateCommand,
    DeleteSelected,
    ContextMenuAction(ContextMenuAction),
    ContextMenuDismiss,
    CancelInlineEdit,
    ResetInteractionState,
    InlineEditChanged(String),
    InlineEditSubmit,
    WizardSaveRequested(QuickLaunchWizardSaveRequest),
    SetupCompleted(QuickLaunchSetupOutcome),
    PersistCompleted,
    PersistFailed(String),
    Tick,
}

pub(crate) const QUICK_LAUNCHES_TICK_MS: u64 = 200;
const LAUNCH_ICON_DELAY_MS: u64 = 1_000;
const LAUNCH_ICON_BLINK_MS: u128 = 500;

/// Runtime dependencies used by the quick launches reducer.
pub(crate) struct QuickLaunchesDeps<'a> {
    pub(crate) terminal_settings: &'a Settings,
}

pub(crate) fn quick_launches_reducer(
    state: &mut State,
    deps: QuickLaunchesDeps<'_>,
    event: QuickLaunchEvent,
) -> Task<AppEvent> {
    use QuickLaunchEvent::*;

    match event {
        OpenErrorTab {
            tab_id,
            title,
            message,
        } => {
            state.quick_launches.set_error_tab(
                tab_id,
                QuickLaunchErrorState::new(title, message),
            );
            Task::none()
        },
        TabClosed { tab_id } => {
            state.quick_launches.remove_error_tab(tab_id);
            Task::none()
        },
        CursorMoved { position } => {
            state.quick_launches.set_cursor(position);
            state.sidebar.cursor = position;
            update_drag_state(state);
            Task::none()
        },
        NodeHovered { path } => {
            if state.sidebar.is_resizing() {
                return Task::none();
            }
            state.quick_launches.set_hovered_path(path);
            update_drag_drop_target(state);
            Task::none()
        },
        BackgroundPressed => {
            state.quick_launches.clear_context_menu();
            state.quick_launches.clear_inline_edit();
            state.quick_launches.clear_selected_path();
            Task::none()
        },
        BackgroundReleased => {
            if state
                .quick_launches
                .drag()
                .map(|drag| drag.active)
                .unwrap_or(false)
            {
                state.quick_launches.set_drop_target(Some(DropTarget::Root));
            }
            if finish_drag(state) {
                return Task::none();
            }
            state.quick_launches.clear_pressed_path();
            Task::none()
        },
        BackgroundRightClicked => {
            state
                .quick_launches
                .set_context_menu(Some(ContextMenuState {
                    target: ContextMenuTarget::Background,
                    cursor: state.sidebar.cursor,
                }));
            state.quick_launches.clear_selected_path();
            Task::none()
        },
        NodeRightClicked { path } => {
            let selected_path = path.clone();
            let Some(node) = state.quick_launches.data().node(&path) else {
                return Task::none();
            };
            let target = match node {
                QuickLaunchNode::Folder(_) => ContextMenuTarget::Folder(path),
                QuickLaunchNode::Command(_) => ContextMenuTarget::Command(path),
            };
            state
                .quick_launches
                .set_context_menu(Some(ContextMenuState {
                    target,
                    cursor: state.sidebar.cursor,
                }));
            state.quick_launches.set_selected_path(Some(selected_path));
            Task::none()
        },
        ContextMenuDismiss => {
            state.quick_launches.clear_context_menu();
            Task::none()
        },
        CancelInlineEdit => {
            state.quick_launches.clear_inline_edit();
            Task::none()
        },
        ResetInteractionState => {
            state.quick_launches.set_hovered_path(None);
            state.quick_launches.clear_pressed_path();
            state.quick_launches.clear_drag();
            state.quick_launches.clear_drop_target();
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
            let Some(path) = state.quick_launches.selected_path_cloned() else {
                return Task::none();
            };
            let Some(node) = state.quick_launches.data().node(&path) else {
                return Task::none();
            };
            if matches!(node, QuickLaunchNode::Folder(_)) {
                remove_node(state, &path);
                state.quick_launches.clear_selected_path();
            }
            Task::none()
        },
        NodePressed { path } => {
            state.quick_launches.set_pressed_path(Some(path.clone()));
            state.quick_launches.set_selected_path(Some(path.clone()));
            state.quick_launches.set_drag(Some(DragState {
                source: path,
                origin: state.quick_launches.cursor(),
                active: false,
            }));
            Task::none()
        },
        NodeReleased { path } => {
            if finish_drag(state) {
                return Task::none();
            }
            let clicked = state
                .quick_launches
                .pressed_path()
                .map(|pressed| pressed == &path)
                .unwrap_or(false);
            state.quick_launches.clear_pressed_path();
            if clicked {
                return handle_node_left_click(
                    state,
                    deps.terminal_settings,
                    path,
                );
            }
            Task::none()
        },
        InlineEditChanged(value) => {
            if let Some(edit) = state.quick_launches.inline_edit_mut() {
                edit.value = value;
                edit.error = None;
            }
            Task::none()
        },
        InlineEditSubmit => apply_inline_edit(state),
        WizardSaveRequested(request) => {
            apply_editor_save_request(state, request)
        },
        ContextMenuAction(action) => {
            handle_context_menu_action(state, deps.terminal_settings, action)
        },
        SetupCompleted(outcome) => reduce_setup_completed(state, outcome),
        PersistCompleted => {
            state.quick_launches.complete_persist();
            Task::none()
        },
        PersistFailed(message) => {
            state.quick_launches.fail_persist();
            log::warn!("quick launches save failed: {message}");
            Task::none()
        },
        Tick => {
            let mut tasks = Vec::new();
            if state.quick_launches.has_active_launches() {
                state.quick_launches.advance_blink_nonce();
                update_launch_indicators(state);
            }
            if state.quick_launches.is_dirty()
                && !state.quick_launches.is_persist_in_flight()
            {
                state.quick_launches.begin_persist();
                tasks.push(request_persist_quick_launches(
                    state.quick_launches.data().clone(),
                ));
            }

            if tasks.is_empty() {
                Task::none()
            } else {
                Task::batch(tasks)
            }
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
        title,
        settings,
        command,
    } = launch;

    if should_skip_launch_result(state, &path, launch_id) {
        return Task::none();
    }

    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::QuickLaunchCommandTerminal {
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
    let Some(node) = state.quick_launches.data().node(&path).cloned() else {
        return Task::none();
    };

    if matches!(state.quick_launches.inline_edit(), Some(edit) if inline_edit_matches(edit, &path))
    {
        return Task::none();
    }

    state.quick_launches.clear_inline_edit();
    state.quick_launches.clear_context_menu();
    state.quick_launches.set_selected_path(Some(path.clone()));

    match node {
        QuickLaunchNode::Folder(_) => {
            toggle_folder_expanded(state, &path);
            persist_quick_launches(state)
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
    let Some(menu) = state.quick_launches.context_menu_cloned() else {
        return Task::none();
    };

    state.quick_launches.clear_context_menu();

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
            state.quick_launches.data_mut().node_mut(&parent_path)
    {
        folder.expanded = true;
    }

    let edit = InlineEditState {
        kind: InlineEditKind::CreateFolder { parent_path },
        value: String::new(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.quick_launches.set_inline_edit(Some(edit));
    state.quick_launches.clear_context_menu();
}

fn begin_inline_rename(state: &mut State, path: NodePath) {
    let Some(node) = state.quick_launches.data().node(&path) else {
        return;
    };

    let edit = InlineEditState {
        kind: InlineEditKind::Rename { path },
        value: node.title().to_string(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.quick_launches.set_inline_edit(Some(edit));
}

fn inline_edit_matches(edit: &InlineEditState, path: &[String]) -> bool {
    match &edit.kind {
        InlineEditKind::Rename { path: edit_path } => edit_path == path,
        InlineEditKind::CreateFolder { .. } => false,
    }
}

fn apply_inline_edit(state: &mut State) -> Task<AppEvent> {
    let Some(edit) = state.quick_launches.take_inline_edit() else {
        return Task::none();
    };

    match edit.kind {
        InlineEditKind::CreateFolder { parent_path } => {
            let Some(parent) =
                state.quick_launches.data_mut().folder_mut(&parent_path)
            else {
                return Task::none();
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
                    persist_quick_launches(state)
                },
                Err(err) => {
                    state.quick_launches.set_inline_edit(Some(
                        InlineEditState {
                            kind: InlineEditKind::CreateFolder { parent_path },
                            value: edit.value,
                            error: Some(format!("{err}")),
                            id: edit.id,
                        },
                    ));
                    Task::none()
                },
            }
        },
        InlineEditKind::Rename { path } => {
            let Some(parent) =
                state.quick_launches.data_mut().parent_folder_mut(&path)
            else {
                return Task::none();
            };
            let current_title = path.last().cloned().unwrap_or_default();
            match parent.normalize_title(&edit.value, Some(&current_title)) {
                Ok(title) => {
                    let mut renamed_path = path.clone();
                    if let Some(last) = renamed_path.last_mut() {
                        *last = title.clone();
                    }

                    if let Some(node) =
                        state.quick_launches.data_mut().node_mut(&path)
                    {
                        *node.title_mut() = title;
                    }

                    update_launching_paths(state, &path, &renamed_path);
                    if state
                        .quick_launches
                        .selected_path()
                        .map(|selected| selected == &path)
                        .unwrap_or(false)
                    {
                        state
                            .quick_launches
                            .set_selected_path(Some(renamed_path));
                    }

                    persist_quick_launches(state)
                },
                Err(err) => {
                    state.quick_launches.set_inline_edit(Some(
                        InlineEditState {
                            kind: InlineEditKind::Rename { path },
                            value: edit.value,
                            error: Some(format!("{err}")),
                            id: edit.id,
                        },
                    ));
                    Task::none()
                },
            }
        },
    }
}

fn apply_editor_save_request(
    state: &mut State,
    request: QuickLaunchWizardSaveRequest,
) -> Task<AppEvent> {
    match request.target {
        QuickLaunchWizardSaveTarget::Create { parent_path } => {
            let Some(parent) =
                state.quick_launches.data_mut().folder_mut(&parent_path)
            else {
                return request_editor_error(
                    request.tab_id,
                    "Missing target folder.",
                );
            };

            let title =
                match parent.normalize_title(&request.command.title, None) {
                    Ok(title) => title,
                    Err(err) => {
                        return request_editor_error(
                            request.tab_id,
                            &format!("{err}"),
                        );
                    },
                };

            let mut command = request.command;
            command.title = title;
            parent.children.push(QuickLaunchNode::Command(command));
        },
        QuickLaunchWizardSaveTarget::Edit { path } => {
            {
                let Some(parent) =
                    state.quick_launches.data_mut().parent_folder_mut(&path)
                else {
                    return request_editor_error(
                        request.tab_id,
                        "Missing parent folder.",
                    );
                };
                let current = path.last().map(String::as_str);
                if let Err(err) =
                    parent.normalize_title(&request.command.title, current)
                {
                    return request_editor_error(
                        request.tab_id,
                        &format!("{err}"),
                    );
                }
            }

            let Some(node) = state.quick_launches.data_mut().node_mut(&path)
            else {
                return request_editor_error(
                    request.tab_id,
                    "Command no longer exists.",
                );
            };
            *node = QuickLaunchNode::Command(request.command);
        },
    }

    state.quick_launches.mark_dirty();
    Task::done(AppEvent::Tab(TabEvent::CloseTab {
        tab_id: request.tab_id,
    }))
}

fn request_editor_error(tab_id: u64, message: &str) -> Task<AppEvent> {
    Task::done(AppEvent::QuickLaunchWizard {
        tab_id,
        event: QuickLaunchWizardEvent::SetError {
            message: String::from(message),
        },
    })
}

fn toggle_folder_expanded(state: &mut State, path: &[String]) {
    let Some(node) = state.quick_launches.data_mut().node_mut(path) else {
        return;
    };
    if let QuickLaunchNode::Folder(folder) = node {
        folder.expanded = !folder.expanded;
    }
}

fn selected_parent_path(state: &State) -> NodePath {
    let Some(selected) = state.quick_launches.selected_path() else {
        return Vec::new();
    };

    let Some(node) = state.quick_launches.data().node(selected) else {
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
    let Some(edit) = state.quick_launches.inline_edit() else {
        return Task::none();
    };

    operation::focus(edit.id.clone())
}

fn update_drag_state(state: &mut State) {
    let (active, source) = {
        let cursor = state.quick_launches.cursor();
        let Some(drag) = state.quick_launches.drag_mut() else {
            return;
        };

        let dx = cursor.x - drag.origin.x;
        let dy = cursor.y - drag.origin.y;
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
            state.quick_launches.set_drop_target(Some(target));
        } else {
            state.quick_launches.clear_drop_target();
        }
    }
}

fn finish_drag(state: &mut State) -> bool {
    let Some(drag) = state.quick_launches.take_drag() else {
        return false;
    };

    if !drag.active {
        state.quick_launches.clear_drop_target();
        return false;
    }

    let target = match state.quick_launches.take_drop_target() {
        Some(target) => target,
        None => return true,
    };
    state.quick_launches.clear_pressed_path();
    move_node(state, drag.source, target);
    true
}

fn drop_target_from_hover(state: &State) -> DropTarget {
    let Some(hovered) = state.quick_launches.hovered_path() else {
        return DropTarget::Root;
    };

    let Some(node) = state.quick_launches.data().node(hovered) else {
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
    let Some(drag) = state.quick_launches.drag() else {
        return;
    };

    if !drag.active {
        return;
    }

    let target = drop_target_from_hover(state);
    if can_drop(state, &drag.source, &target) {
        state.quick_launches.set_drop_target(Some(target));
    } else {
        state.quick_launches.clear_drop_target();
    }
}

fn move_node(state: &mut State, source: NodePath, target: DropTarget) {
    let Some(node) = state.quick_launches.data().node(&source).cloned() else {
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
    if let Some(target_folder) =
        state.quick_launches.data().folder(&target_path)
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
            state.quick_launches.data_mut().parent_folder_mut(&source)
        else {
            return;
        };
        let Some(moved) = parent_folder.remove_child(&title) else {
            return;
        };
        moved
    };

    let Some(target_folder) =
        state.quick_launches.data_mut().folder_mut(&target_path)
    else {
        return;
    };

    target_folder.children.push(moved);
    let mut new_path = target_path.clone();
    new_path.push(title);
    update_launching_paths(state, &source, &new_path);
    state.quick_launches.set_selected_path(Some(new_path));
    let _task = persist_quick_launches(state);
}

fn update_launching_paths(
    state: &mut State,
    source: &[String],
    new_path: &[String],
) {
    let mut moved: Vec<(NodePath, LaunchInfo)> = Vec::new();
    for (path, info) in state.quick_launches.launching() {
        if is_prefix(source, path) {
            moved.push((path.clone(), info.clone()));
        }
    }

    if moved.is_empty() {
        return;
    }

    for (path, _) in &moved {
        let _ = state.quick_launches.remove_launch(path);
    }

    for (old_path, info) in moved {
        let mut updated = new_path.to_vec();
        updated.extend_from_slice(&old_path[source.len()..]);
        state.quick_launches.launching_mut().insert(updated, info);
    }
}

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }

    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn can_drop(state: &State, source: &[String], target: &DropTarget) -> bool {
    let Some(node) = state.quick_launches.data().node(source) else {
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
    let Some(parent) = state.quick_launches.data_mut().parent_folder_mut(path)
    else {
        return;
    };
    let Some(title) = path.last() else {
        return;
    };
    parent.remove_child(title);
    let _task = persist_quick_launches(state);
}

fn kill_command_launch(state: &mut State, path: &[String]) {
    state.quick_launches.cancel_launch(path);
}

fn remove_launch_by_id(state: &mut State, launch_id: u64) {
    let path =
        state
            .quick_launches
            .launching()
            .iter()
            .find_map(|(path, info)| {
                if info.id == launch_id {
                    Some(path.clone())
                } else {
                    None
                }
            });

    if let Some(path) = path {
        let _ = state.quick_launches.remove_launch(&path);
    }
}

fn duplicate_command(state: &mut State, path: &[String]) {
    let Some(node) = state.quick_launches.data().node(path).cloned() else {
        return;
    };
    let QuickLaunchNode::Command(command) = node else {
        return;
    };

    let Some(parent) = state.quick_launches.data_mut().parent_folder_mut(path)
    else {
        return;
    };

    let mut clone = command.clone();
    clone.title = duplicate_title(parent, &command.title);
    parent.children.push(QuickLaunchNode::Command(clone));
    let _task = persist_quick_launches(state);
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

fn persist_quick_launches(state: &mut State) -> Task<AppEvent> {
    state.quick_launches.mark_dirty();
    Task::none()
}

fn request_persist_quick_launches(data: QuickLaunchFile) -> Task<AppEvent> {
    Task::perform(
        async move {
            match save_quick_launches(&data) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("{err}")),
            }
        },
        |result| match result {
            Ok(()) => AppEvent::QuickLaunch(QuickLaunchEvent::PersistCompleted),
            Err(message) => {
                AppEvent::QuickLaunch(QuickLaunchEvent::PersistFailed(message))
            },
        },
    )
}

fn launch_quick_launch(
    state: &mut State,
    terminal_settings: &Settings,
    path: NodePath,
    command: QuickLaunch,
) -> Task<AppEvent> {
    if state.quick_launches.is_launching(&path) {
        return Task::none();
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let launch_id = state
        .quick_launches
        .begin_launch(path.clone(), cancel.clone());

    Task::perform(
        prepare_quick_launch_setup(
            command,
            path,
            launch_id,
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
    if state.quick_launches.take_canceled_launch(launch_id) {
        remove_launch_by_id(state, launch_id);
        return true;
    }

    if let Some(info) = state.quick_launches.launch_info(path)
        && info.id != launch_id
    {
        return true;
    }

    remove_launch_by_id(state, launch_id);
    false
}

fn update_launch_indicators(state: &mut State) {
    let blink_nonce = state.quick_launches.blink_nonce();
    for info in state.quick_launches.launching_mut().values_mut() {
        info.launch_ticks = info.launch_ticks.wrapping_add(1);
        info.is_indicator_highlighted =
            should_highlight_launch_indicator(info.launch_ticks, blink_nonce);
    }
}

fn should_highlight_launch_indicator(
    launch_ticks: u64,
    blink_nonce: u64,
) -> bool {
    let launch_age_ms = launch_ticks.saturating_mul(QUICK_LAUNCHES_TICK_MS);
    if launch_age_ms < LAUNCH_ICON_DELAY_MS {
        return false;
    }

    let blink_step = (blink_nonce as u128 * QUICK_LAUNCHES_TICK_MS as u128)
        / LAUNCH_ICON_BLINK_MS;
    blink_step.is_multiple_of(2)
}

fn open_create_command_tab(
    _state: &mut State,
    parent: NodePath,
) -> Task<AppEvent> {
    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::QuickLaunchWizardCreate {
            parent_path: parent,
        },
    }))
}

fn open_edit_command_tab(state: &mut State, path: NodePath) -> Task<AppEvent> {
    let Some(node) = state.quick_launches.data().node(&path).cloned() else {
        return Task::none();
    };
    let QuickLaunchNode::Command(command) = node else {
        return Task::none();
    };

    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::QuickLaunchWizardEdit {
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
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use otty_ui_term::settings::Settings;

    use super::*;
    use crate::state::State;

    fn deps(settings: &Settings) -> QuickLaunchesDeps<'_> {
        QuickLaunchesDeps {
            terminal_settings: settings,
        }
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
        state.quick_launches.data_mut().root.children.push(
            QuickLaunchNode::Folder(QuickLaunchFolder {
                title: String::from("Folder"),
                expanded: true,
                children: Vec::new(),
            }),
        );
        state
            .quick_launches
            .set_selected_path(Some(vec![String::from("Folder")]));

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::DeleteSelected,
        );

        assert!(state.quick_launches.data().root.children.is_empty());
        assert!(state.quick_launches.selected_path().is_none());
    }

    #[test]
    fn given_unknown_node_when_released_then_reducer_ignores_event() {
        let mut state = State::default();

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::NodeReleased {
                path: vec![String::from("Missing")],
            },
        );

        assert!(state.quick_launches.selected_path().is_none());
        assert!(state.quick_launches.launching().is_empty());
    }

    #[test]
    fn given_editor_create_save_request_when_reduced_then_command_is_added() {
        let mut state = State::default();
        state.quick_launches.data_mut().root = QuickLaunchFolder {
            title: String::from("Root"),
            expanded: true,
            children: Vec::new(),
        };

        let request = QuickLaunchWizardSaveRequest {
            tab_id: 7,
            target: QuickLaunchWizardSaveTarget::Create {
                parent_path: Vec::new(),
            },
            command: sample_command(),
        };

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::WizardSaveRequested(request),
        );

        assert_eq!(state.quick_launches.data().root.children.len(), 1);
        assert!(state.quick_launches.is_dirty());
    }

    #[test]
    fn given_editor_save_with_duplicate_title_when_reduced_then_data_is_unchanged()
     {
        let mut state = State::default();
        state.quick_launches.data_mut().root = QuickLaunchFolder {
            title: String::from("Root"),
            expanded: true,
            children: vec![
                QuickLaunchNode::Command(QuickLaunch {
                    title: String::from("Run"),
                    spec: super::super::model::CommandSpec::Custom {
                        custom: super::super::model::CustomCommand {
                            program: String::from("bash"),
                            args: Vec::new(),
                            env: Vec::new(),
                            working_directory: None,
                        },
                    },
                }),
                QuickLaunchNode::Command(QuickLaunch {
                    title: String::from("Copy"),
                    spec: super::super::model::CommandSpec::Custom {
                        custom: super::super::model::CustomCommand {
                            program: String::from("bash"),
                            args: Vec::new(),
                            env: Vec::new(),
                            working_directory: None,
                        },
                    },
                }),
            ],
        };

        let request = QuickLaunchWizardSaveRequest {
            tab_id: 3,
            target: QuickLaunchWizardSaveTarget::Edit {
                path: vec![String::from("Run")],
            },
            command: QuickLaunch {
                title: String::from("Copy"),
                spec: super::super::model::CommandSpec::Custom {
                    custom: super::super::model::CustomCommand {
                        program: String::from("bash"),
                        args: Vec::new(),
                        env: Vec::new(),
                        working_directory: None,
                    },
                },
            },
        };

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::WizardSaveRequested(request),
        );

        assert_eq!(state.quick_launches.data().root.children.len(), 2);
        assert!(!state.quick_launches.is_dirty());
    }

    #[test]
    fn given_editor_save_with_missing_target_when_reduced_then_state_is_unchanged()
     {
        let mut state = State::default();

        let request = QuickLaunchWizardSaveRequest {
            tab_id: 42,
            target: QuickLaunchWizardSaveTarget::Edit {
                path: vec![String::from("Missing")],
            },
            command: sample_command(),
        };

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::WizardSaveRequested(request),
        );

        assert!(state.quick_launches.data().root.children.is_empty());
        assert!(!state.quick_launches.is_dirty());
    }

    #[test]
    fn given_error_tab_opened_and_closed_when_reduced_then_payload_is_cleaned()
    {
        let mut state = State::default();

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::OpenErrorTab {
                tab_id: 42,
                title: String::from("Failed"),
                message: String::from("Boom"),
            },
        );
        assert!(state.quick_launches.error_tab(42).is_some());

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::TabClosed { tab_id: 42 },
        );
        assert!(state.quick_launches.error_tab(42).is_none());
    }

    #[test]
    fn given_tick_event_with_active_launch_when_reduced_then_blink_nonce_increments()
     {
        let mut state = State::default();
        let path = vec![String::from("Demo")];
        state.quick_launches.set_blink_nonce_for_tests(10);
        state.quick_launches.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 1,
                launch_ticks: 0,
                is_indicator_highlighted: false,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::Tick,
        );

        assert_eq!(state.quick_launches.blink_nonce(), 11);
        let info = state
            .quick_launches
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert_eq!(info.launch_ticks, 1);
        assert!(!info.is_indicator_highlighted);
    }

    #[test]
    fn given_launch_before_delay_when_tick_then_indicator_is_off() {
        let mut state = State::default();
        let path = vec![String::from("Demo")];
        state.quick_launches.set_blink_nonce_for_tests(4);
        state.quick_launches.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 1,
                launch_ticks: 3,
                is_indicator_highlighted: true,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::Tick,
        );

        let info = state
            .quick_launches
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert_eq!(info.launch_ticks, 4);
        assert!(!info.is_indicator_highlighted);
    }

    #[test]
    fn given_launch_after_delay_when_tick_then_indicator_toggles_by_period() {
        let mut state = State::default();
        let path = vec![String::from("Demo")];
        state.quick_launches.set_blink_nonce_for_tests(2);
        state.quick_launches.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 1,
                launch_ticks: 4,
                is_indicator_highlighted: false,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::Tick,
        );
        let info = state
            .quick_launches
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert!(!info.is_indicator_highlighted);

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::Tick,
        );
        let info = state
            .quick_launches
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert!(!info.is_indicator_highlighted);

        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::Tick,
        );
        let info = state
            .quick_launches
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert!(info.is_indicator_highlighted);
    }

    #[test]
    fn given_failed_setup_completion_when_reduced_then_launch_is_removed() {
        let mut state = State::default();
        let path = vec![String::from("Demo")];
        state.quick_launches.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 9,
                launch_ticks: 0,
                is_indicator_highlighted: false,
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
            deps(&Settings::default()),
            QuickLaunchEvent::SetupCompleted(outcome),
        );

        assert!(!state.quick_launches.launching().contains_key(&path));
    }

    #[test]
    fn given_canceled_launch_completion_when_reduced_then_result_is_ignored() {
        let mut state = State::default();
        let path = vec![String::from("Demo")];
        state.quick_launches.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 11,
                launch_ticks: 0,
                is_indicator_highlighted: false,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );
        state.quick_launches.cancel_launch(&path);

        let outcome = QuickLaunchSetupOutcome::Failed {
            path: path.clone(),
            launch_id: 11,
            command: Box::new(sample_command()),
            error: Arc::new(QuickLaunchError::Validation {
                message: String::from("Program is empty."),
            }),
        };
        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::SetupCompleted(outcome),
        );

        assert!(state.quick_launches.launching().get(&path).is_none());
    }

    #[test]
    fn given_stale_launch_completion_when_reduced_then_current_launch_is_kept()
    {
        let mut state = State::default();
        let path = vec![String::from("Demo")];
        state.quick_launches.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 22,
                launch_ticks: 0,
                is_indicator_highlighted: false,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let outcome = QuickLaunchSetupOutcome::Failed {
            path: path.clone(),
            launch_id: 21,
            command: Box::new(sample_command()),
            error: Arc::new(QuickLaunchError::Validation {
                message: String::from("Program is empty."),
            }),
        };
        let _task = quick_launches_reducer(
            &mut state,
            deps(&Settings::default()),
            QuickLaunchEvent::SetupCompleted(outcome),
        );

        let active = state
            .quick_launches
            .launching()
            .get(&path)
            .expect("current launch must remain active");
        assert_eq!(active.id, 22);
    }
}
