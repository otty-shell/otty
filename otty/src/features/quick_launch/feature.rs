use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::widget::operation;
use iced::{Point, Task};
use otty_ui_term::settings::Settings;

use super::event::QuickLaunchEvent;
use super::model::{
    ContextMenuAction, ContextMenuTarget, LaunchInfo, NodePath,
    PreparedQuickLaunch, QuickLaunch, QuickLaunchFile, QuickLaunchFolder,
    QuickLaunchNode, QuickLaunchSetupOutcome, QuickLaunchWizardSaveRequest,
    QuickLaunchWizardSaveTarget, quick_launch_error_message,
};
use super::services::prepare_quick_launch_setup;
use super::state::{
    ContextMenuState, DragState, DropTarget, InlineEditKind, InlineEditState,
    QuickLaunchErrorState, QuickLaunchState,
};
use super::storage::save_quick_launches;
use crate::app::Event as AppEvent;
use crate::features::Feature;
use crate::features::quick_launch_wizard::QuickLaunchWizardEvent;

/// Runtime dependencies for the quick launches feature.
pub(crate) struct QuickLaunchCtx<'a> {
    /// Terminal backend settings used when launching commands.
    pub(crate) terminal_settings: &'a Settings,
    /// Current sidebar cursor position (read-only snapshot from sidebar state).
    pub(crate) sidebar_cursor: Point,
    /// Whether the sidebar is currently being resized.
    pub(crate) sidebar_is_resizing: bool,
}

/// Feature root that owns quick launch state and reduction logic.
pub(crate) struct QuickLaunchFeature {
    state: QuickLaunchState,
}

impl QuickLaunchFeature {
    /// Create a feature instance from pre-loaded state.
    pub(crate) fn new(state: QuickLaunchState) -> Self {
        Self { state }
    }

    /// Load state from disk and create the feature instance.
    ///
    /// Falls back to a default state on I/O or parse errors.
    pub(crate) fn load() -> Self {
        use super::services::load_initial_quick_launch_state;
        Self::new(load_initial_quick_launch_state())
    }

    /// Return read-only access to the quick launch state for widget rendering.
    pub(crate) fn state(&self) -> &QuickLaunchState {
        &self.state
    }

    /// Return whether any launch is currently in progress.
    pub(crate) fn has_active_launches(&self) -> bool {
        self.state.has_active_launches()
    }

    /// Return whether there are unsaved local changes.
    pub(crate) fn is_dirty(&self) -> bool {
        self.state.is_dirty()
    }

    /// Return whether persistence is currently in progress.
    pub(crate) fn is_persist_in_flight(&self) -> bool {
        self.state.is_persist_in_flight()
    }

    /// Return active inline edit state.
    pub(crate) fn inline_edit(&self) -> Option<&InlineEditState> {
        self.state.inline_edit()
    }

    /// Return current context menu state.
    pub(crate) fn context_menu(&self) -> Option<&ContextMenuState> {
        self.state.context_menu()
    }

    /// Return in-flight launch map.
    pub(crate) fn launching(
        &self,
    ) -> &std::collections::HashMap<NodePath, LaunchInfo> {
        self.state.launching()
    }
}

impl Feature for QuickLaunchFeature {
    type Event = QuickLaunchEvent;
    type Ctx<'a>
        = QuickLaunchCtx<'a>
    where
        Self: 'a;

    fn reduce<'a>(
        &mut self,
        event: QuickLaunchEvent,
        ctx: &QuickLaunchCtx<'a>,
    ) -> Task<AppEvent> {
        use QuickLaunchEvent::*;

        match event {
            OpenErrorTab {
                tab_id,
                title,
                message,
            } => {
                self.state.set_error_tab(
                    tab_id,
                    QuickLaunchErrorState::new(title, message),
                );
                Task::none()
            },
            TabClosed { tab_id } => {
                self.state.remove_error_tab(tab_id);
                Task::none()
            },
            CursorMoved { position } => {
                self.state.set_cursor(position);
                // Note: sidebar cursor is updated separately by app.rs via
                // SidebarWorkspaceEvent::WorkspaceCursorMoved.
                update_drag_state(&mut self.state);
                Task::none()
            },
            NodeHovered { path } => {
                if ctx.sidebar_is_resizing {
                    return Task::none();
                }
                self.state.set_hovered_path(path);
                update_drag_drop_target(&mut self.state);
                Task::none()
            },
            BackgroundPressed => {
                self.state.clear_context_menu();
                self.state.clear_inline_edit();
                self.state.clear_selected_path();
                Task::none()
            },
            BackgroundReleased => {
                if self.state.drag().map(|drag| drag.active).unwrap_or(false) {
                    self.state.set_drop_target(Some(DropTarget::Root));
                }
                if finish_drag(&mut self.state) {
                    return Task::none();
                }
                self.state.clear_pressed_path();
                Task::none()
            },
            BackgroundRightClicked => {
                self.state.set_context_menu(Some(ContextMenuState {
                    target: ContextMenuTarget::Background,
                    cursor: ctx.sidebar_cursor,
                }));
                self.state.clear_selected_path();
                Task::none()
            },
            NodeRightClicked { path } => {
                let selected_path = path.clone();
                let Some(node) = self.state.data().node(&path) else {
                    return Task::none();
                };
                let target = match node {
                    QuickLaunchNode::Folder(_) => {
                        ContextMenuTarget::Folder(path)
                    },
                    QuickLaunchNode::Command(_) => {
                        ContextMenuTarget::Command(path)
                    },
                };
                self.state.set_context_menu(Some(ContextMenuState {
                    target,
                    cursor: ctx.sidebar_cursor,
                }));
                self.state.set_selected_path(Some(selected_path));
                Task::none()
            },
            ContextMenuDismiss => {
                self.state.clear_context_menu();
                Task::none()
            },
            CancelInlineEdit => {
                self.state.clear_inline_edit();
                Task::none()
            },
            ResetInteractionState => {
                self.state.set_hovered_path(None);
                self.state.clear_pressed_path();
                self.state.clear_drag();
                self.state.clear_drop_target();
                Task::none()
            },
            HeaderCreateFolder => {
                let parent = selected_parent_path(&self.state);
                begin_inline_create_folder(&mut self.state, parent);
                focus_inline_edit(&self.state)
            },
            HeaderCreateCommand => {
                let parent = selected_parent_path(&self.state);
                open_create_command_tab(parent)
            },
            DeleteSelected => {
                let Some(path) = self.state.selected_path_cloned() else {
                    return Task::none();
                };
                let Some(node) = self.state.data().node(&path) else {
                    return Task::none();
                };
                if matches!(node, QuickLaunchNode::Folder(_)) {
                    remove_node(&mut self.state, &path);
                    self.state.clear_selected_path();
                }
                Task::none()
            },
            NodePressed { path } => {
                self.state.set_pressed_path(Some(path.clone()));
                self.state.set_selected_path(Some(path.clone()));
                self.state.set_drag(Some(DragState {
                    source: path,
                    origin: self.state.cursor(),
                    active: false,
                }));
                Task::none()
            },
            NodeReleased { path } => {
                if finish_drag(&mut self.state) {
                    return Task::none();
                }
                let clicked = self
                    .state
                    .pressed_path()
                    .map(|pressed| pressed == &path)
                    .unwrap_or(false);
                self.state.clear_pressed_path();
                if clicked {
                    return handle_node_left_click(
                        &mut self.state,
                        ctx.terminal_settings,
                        path,
                    );
                }
                Task::none()
            },
            InlineEditChanged(value) => {
                if let Some(edit) = self.state.inline_edit_mut() {
                    edit.value = value;
                    edit.error = None;
                }
                Task::none()
            },
            InlineEditSubmit => apply_inline_edit(&mut self.state),
            WizardSaveRequested(request) => {
                apply_editor_save_request(&mut self.state, request)
            },
            ContextMenuAction(action) => handle_context_menu_action(
                &mut self.state,
                ctx.terminal_settings,
                action,
            ),
            SetupCompleted(outcome) => {
                reduce_setup_completed(&mut self.state, outcome)
            },
            PersistCompleted => {
                self.state.complete_persist();
                Task::none()
            },
            PersistFailed(message) => {
                self.state.fail_persist();
                log::warn!("quick launches save failed: {message}");
                Task::none()
            },
            Tick => {
                let mut tasks = Vec::new();
                if self.state.has_active_launches() {
                    self.state.advance_blink_nonce();
                    update_launch_indicators(&mut self.state);
                }
                if self.state.is_dirty() && !self.state.is_persist_in_flight() {
                    self.state.begin_persist();
                    tasks.push(request_persist_quick_launches(
                        self.state.data().clone(),
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
}

fn reduce_setup_completed(
    state: &mut QuickLaunchState,
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
    state: &mut QuickLaunchState,
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

    Task::done(AppEvent::OpenQuickLaunchCommandTerminalTab {
        title,
        settings,
        command,
    })
}

fn handle_node_left_click(
    state: &mut QuickLaunchState,
    terminal_settings: &Settings,
    path: NodePath,
) -> Task<AppEvent> {
    let Some(node) = state.data().node(&path).cloned() else {
        return Task::none();
    };

    if matches!(state.inline_edit(), Some(edit) if inline_edit_matches(edit, &path))
    {
        return Task::none();
    }

    state.clear_inline_edit();
    state.clear_context_menu();
    state.set_selected_path(Some(path.clone()));

    match node {
        QuickLaunchNode::Folder(_) => {
            toggle_folder_expanded(state, &path);
            state.mark_dirty();
            Task::none()
        },
        QuickLaunchNode::Command(command) => {
            launch_quick_launch(state, terminal_settings, path, command)
        },
    }
}

fn handle_context_menu_action(
    state: &mut QuickLaunchState,
    _terminal_settings: &Settings,
    action: ContextMenuAction,
) -> Task<AppEvent> {
    let Some(menu) = state.context_menu_cloned() else {
        return Task::none();
    };

    state.clear_context_menu();

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
            ContextMenuTarget::Folder(path) => open_create_command_tab(path),
            ContextMenuTarget::Command(path) => {
                let parent = path[..path.len().saturating_sub(1)].to_vec();
                open_create_command_tab(parent)
            },
            ContextMenuTarget::Background => {
                open_create_command_tab(Vec::new())
            },
        },
        ContextMenuAction::Kill => match menu.target {
            ContextMenuTarget::Command(path) => {
                state.cancel_launch(&path);
                Task::none()
            },
            _ => Task::none(),
        },
    }
}

fn begin_inline_create_folder(
    state: &mut QuickLaunchState,
    parent_path: NodePath,
) {
    if !parent_path.is_empty()
        && let Some(QuickLaunchNode::Folder(folder)) =
            state.data_mut().node_mut(&parent_path)
    {
        folder.expanded = true;
    }

    let edit = InlineEditState {
        kind: InlineEditKind::CreateFolder { parent_path },
        value: String::new(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.set_inline_edit(Some(edit));
    state.clear_context_menu();
}

fn begin_inline_rename(state: &mut QuickLaunchState, path: NodePath) {
    let Some(node) = state.data().node(&path) else {
        return;
    };

    let edit = InlineEditState {
        kind: InlineEditKind::Rename { path },
        value: node.title().to_string(),
        error: None,
        id: iced::widget::Id::unique(),
    };
    state.set_inline_edit(Some(edit));
}

fn inline_edit_matches(edit: &InlineEditState, path: &[String]) -> bool {
    match &edit.kind {
        InlineEditKind::Rename { path: edit_path } => edit_path == path,
        InlineEditKind::CreateFolder { .. } => false,
    }
}

fn apply_inline_edit(state: &mut QuickLaunchState) -> Task<AppEvent> {
    let Some(edit) = state.take_inline_edit() else {
        return Task::none();
    };

    match edit.kind {
        InlineEditKind::CreateFolder { parent_path } => {
            let Some(parent) = state.data_mut().folder_mut(&parent_path) else {
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
                    state.mark_dirty();
                    Task::none()
                },
                Err(err) => {
                    state.set_inline_edit(Some(InlineEditState {
                        kind: InlineEditKind::CreateFolder { parent_path },
                        value: edit.value,
                        error: Some(format!("{err}")),
                        id: edit.id,
                    }));
                    Task::none()
                },
            }
        },
        InlineEditKind::Rename { path } => {
            let Some(parent) = state.data_mut().parent_folder_mut(&path) else {
                return Task::none();
            };
            let current_title = path.last().cloned().unwrap_or_default();
            match parent.normalize_title(&edit.value, Some(&current_title)) {
                Ok(title) => {
                    let mut renamed_path = path.clone();
                    if let Some(last) = renamed_path.last_mut() {
                        *last = title.clone();
                    }

                    if let Some(node) = state.data_mut().node_mut(&path) {
                        *node.title_mut() = title;
                    }

                    update_launching_paths(state, &path, &renamed_path);
                    if state
                        .selected_path()
                        .map(|selected| selected == &path)
                        .unwrap_or(false)
                    {
                        state.set_selected_path(Some(renamed_path));
                    }

                    state.mark_dirty();
                    Task::none()
                },
                Err(err) => {
                    state.set_inline_edit(Some(InlineEditState {
                        kind: InlineEditKind::Rename { path },
                        value: edit.value,
                        error: Some(format!("{err}")),
                        id: edit.id,
                    }));
                    Task::none()
                },
            }
        },
    }
}

fn apply_editor_save_request(
    state: &mut QuickLaunchState,
    request: QuickLaunchWizardSaveRequest,
) -> Task<AppEvent> {
    match request.target {
        QuickLaunchWizardSaveTarget::Create { parent_path } => {
            let Some(parent) = state.data_mut().folder_mut(&parent_path) else {
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
                let Some(parent) = state.data_mut().parent_folder_mut(&path)
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

            let Some(node) = state.data_mut().node_mut(&path) else {
                return request_editor_error(
                    request.tab_id,
                    "Command no longer exists.",
                );
            };
            *node = QuickLaunchNode::Command(request.command);
        },
    }

    state.mark_dirty();
    Task::done(AppEvent::CloseTabRequested {
        tab_id: request.tab_id,
    })
}

fn request_editor_error(tab_id: u64, message: &str) -> Task<AppEvent> {
    Task::done(AppEvent::QuickLaunchWizard {
        tab_id,
        event: QuickLaunchWizardEvent::SetError {
            message: String::from(message),
        },
    })
}

fn toggle_folder_expanded(state: &mut QuickLaunchState, path: &[String]) {
    let Some(node) = state.data_mut().node_mut(path) else {
        return;
    };
    if let QuickLaunchNode::Folder(folder) = node {
        folder.expanded = !folder.expanded;
    }
}

fn selected_parent_path(state: &QuickLaunchState) -> NodePath {
    let Some(selected) = state.selected_path() else {
        return Vec::new();
    };

    let Some(node) = state.data().node(selected) else {
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

fn focus_inline_edit(state: &QuickLaunchState) -> Task<AppEvent> {
    let Some(edit) = state.inline_edit() else {
        return Task::none();
    };

    operation::focus(edit.id.clone())
}

fn update_drag_state(state: &mut QuickLaunchState) {
    let (active, source) = {
        let cursor = state.cursor();
        let Some(drag) = state.drag_mut() else {
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
            state.set_drop_target(Some(target));
        } else {
            state.clear_drop_target();
        }
    }
}

fn finish_drag(state: &mut QuickLaunchState) -> bool {
    let Some(drag) = state.take_drag() else {
        return false;
    };

    if !drag.active {
        state.clear_drop_target();
        return false;
    }

    let target = match state.take_drop_target() {
        Some(target) => target,
        None => return true,
    };
    state.clear_pressed_path();
    move_node(state, drag.source, target);
    true
}

fn drop_target_from_hover(state: &QuickLaunchState) -> DropTarget {
    let Some(hovered) = state.hovered_path() else {
        return DropTarget::Root;
    };

    let Some(node) = state.data().node(hovered) else {
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

fn update_drag_drop_target(state: &mut QuickLaunchState) {
    let (drag_active, drag_source) = match state.drag() {
        Some(drag) => (drag.active, drag.source.clone()),
        None => return,
    };

    if !drag_active {
        return;
    }

    let target = drop_target_from_hover(state);
    if can_drop(state, &drag_source, &target) {
        state.set_drop_target(Some(target));
    } else {
        state.clear_drop_target();
    }
}

fn move_node(
    state: &mut QuickLaunchState,
    source: NodePath,
    target: DropTarget,
) {
    let Some(node) = state.data().node(&source).cloned() else {
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
    if let Some(target_folder) = state.data().folder(&target_path) {
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
        let Some(parent_folder) = state.data_mut().parent_folder_mut(&source)
        else {
            return;
        };
        let Some(moved) = parent_folder.remove_child(&title) else {
            return;
        };
        moved
    };

    let Some(target_folder) = state.data_mut().folder_mut(&target_path) else {
        return;
    };

    target_folder.children.push(moved);
    let mut new_path = target_path.clone();
    new_path.push(title);
    update_launching_paths(state, &source, &new_path);
    state.set_selected_path(Some(new_path));
    state.mark_dirty();
}

fn update_launching_paths(
    state: &mut QuickLaunchState,
    source: &[String],
    new_path: &[String],
) {
    let mut moved: Vec<(NodePath, LaunchInfo)> = Vec::new();
    for (path, info) in state.launching() {
        if is_prefix(source, path) {
            moved.push((path.clone(), info.clone()));
        }
    }

    if moved.is_empty() {
        return;
    }

    for (path, _) in &moved {
        let _ = state.remove_launch(path);
    }

    for (old_path, info) in moved {
        let mut updated = new_path.to_vec();
        updated.extend_from_slice(&old_path[source.len()..]);
        state.launching_mut().insert(updated, info);
    }
}

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }

    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn can_drop(
    state: &QuickLaunchState,
    source: &[String],
    target: &DropTarget,
) -> bool {
    let Some(node) = state.data().node(source) else {
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

fn remove_node(state: &mut QuickLaunchState, path: &[String]) {
    let Some(parent) = state.data_mut().parent_folder_mut(path) else {
        return;
    };
    let Some(title) = path.last() else {
        return;
    };
    parent.remove_child(title);
    state.mark_dirty();
}

fn remove_launch_by_id(state: &mut QuickLaunchState, launch_id: u64) {
    let path = state.launching().iter().find_map(|(path, info)| {
        if info.id == launch_id {
            Some(path.clone())
        } else {
            None
        }
    });

    if let Some(path) = path {
        let _ = state.remove_launch(&path);
    }
}

fn duplicate_command(state: &mut QuickLaunchState, path: &[String]) {
    let Some(node) = state.data().node(path).cloned() else {
        return;
    };
    let QuickLaunchNode::Command(command) = node else {
        return;
    };

    let Some(parent) = state.data_mut().parent_folder_mut(path) else {
        return;
    };

    let mut clone = command.clone();
    clone.title = duplicate_title(parent, &command.title);
    parent.children.push(QuickLaunchNode::Command(clone));
    state.mark_dirty();
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
    state: &mut QuickLaunchState,
    terminal_settings: &Settings,
    path: NodePath,
    command: QuickLaunch,
) -> Task<AppEvent> {
    if state.is_launching(&path) {
        return Task::none();
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let launch_id = state.begin_launch(path.clone(), cancel.clone());

    Task::perform(
        prepare_quick_launch_setup(
            command,
            path,
            launch_id,
            terminal_settings.clone(),
            cancel,
        ),
        |outcome| {
            AppEvent::QuickLaunch(QuickLaunchEvent::SetupCompleted(outcome))
        },
    )
}

fn should_skip_launch_result(
    state: &mut QuickLaunchState,
    path: &[String],
    launch_id: u64,
) -> bool {
    if state.take_canceled_launch(launch_id) {
        remove_launch_by_id(state, launch_id);
        return true;
    }

    if let Some(info) = state.launch_info(path)
        && info.id != launch_id
    {
        return true;
    }

    remove_launch_by_id(state, launch_id);
    false
}

fn update_launch_indicators(state: &mut QuickLaunchState) {
    let blink_nonce = state.blink_nonce();
    for info in state.launching_mut().values_mut() {
        info.launch_ticks = info.launch_ticks.wrapping_add(1);
        info.is_indicator_highlighted =
            should_highlight_launch_indicator(info.launch_ticks, blink_nonce);
    }
}

const LAUNCH_ICON_DELAY_MS: u64 = 1_000;
const LAUNCH_ICON_BLINK_MS: u128 = 500;

fn should_highlight_launch_indicator(
    launch_ticks: u64,
    blink_nonce: u64,
) -> bool {
    use super::event::QUICK_LAUNCHES_TICK_MS;

    let launch_age_ms = launch_ticks.saturating_mul(QUICK_LAUNCHES_TICK_MS);
    if launch_age_ms < LAUNCH_ICON_DELAY_MS {
        return false;
    }

    let blink_step = (blink_nonce as u128 * QUICK_LAUNCHES_TICK_MS as u128)
        / LAUNCH_ICON_BLINK_MS;
    blink_step.is_multiple_of(2)
}

fn open_create_command_tab(parent: NodePath) -> Task<AppEvent> {
    Task::done(AppEvent::OpenQuickLaunchWizardCreateTab {
        parent_path: parent,
    })
}

fn open_edit_command_tab(
    state: &QuickLaunchState,
    path: NodePath,
) -> Task<AppEvent> {
    let Some(node) = state.data().node(&path).cloned() else {
        return Task::none();
    };
    let QuickLaunchNode::Command(command) = node else {
        return Task::none();
    };

    Task::done(AppEvent::OpenQuickLaunchWizardEditTab {
        path,
        command: Box::new(command),
    })
}

fn request_open_error_tab(title: String, message: String) -> Task<AppEvent> {
    Task::done(AppEvent::OpenQuickLaunchErrorTab { title, message })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use iced::Point;
    use otty_ui_term::settings::Settings;

    use super::super::model::{CommandSpec, CustomCommand};
    use super::*;

    fn ctx(settings: &Settings) -> QuickLaunchCtx<'_> {
        QuickLaunchCtx {
            terminal_settings: settings,
            sidebar_cursor: Point::ORIGIN,
            sidebar_is_resizing: false,
        }
    }

    fn sample_command() -> QuickLaunch {
        QuickLaunch {
            title: String::from("Demo"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
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
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        feature
            .state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Folder(QuickLaunchFolder {
                title: String::from("Folder"),
                expanded: true,
                children: Vec::new(),
            }));
        feature
            .state
            .set_selected_path(Some(vec![String::from("Folder")]));

        let settings = Settings::default();
        let _task =
            feature.reduce(QuickLaunchEvent::DeleteSelected, &ctx(&settings));

        assert!(feature.state.data().root.children.is_empty());
        assert!(feature.state.selected_path().is_none());
    }

    #[test]
    fn given_unknown_node_when_released_then_reducer_ignores_event() {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());

        let settings = Settings::default();
        let _task = feature.reduce(
            QuickLaunchEvent::NodeReleased {
                path: vec![String::from("Missing")],
            },
            &ctx(&settings),
        );

        assert!(feature.state.selected_path().is_none());
        assert!(feature.state.launching().is_empty());
    }

    #[test]
    fn given_editor_create_save_request_when_reduced_then_command_is_added() {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        feature.state.data_mut().root = QuickLaunchFolder {
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

        let settings = Settings::default();
        let _task = feature.reduce(
            QuickLaunchEvent::WizardSaveRequested(request),
            &ctx(&settings),
        );

        assert_eq!(feature.state.data().root.children.len(), 1);
        assert!(feature.state.is_dirty());
    }

    #[test]
    fn given_editor_save_with_duplicate_title_when_reduced_then_data_is_unchanged()
     {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        feature.state.data_mut().root = QuickLaunchFolder {
            title: String::from("Root"),
            expanded: true,
            children: vec![
                QuickLaunchNode::Command(QuickLaunch {
                    title: String::from("Run"),
                    spec: CommandSpec::Custom {
                        custom: CustomCommand {
                            program: String::from("bash"),
                            args: Vec::new(),
                            env: Vec::new(),
                            working_directory: None,
                        },
                    },
                }),
                QuickLaunchNode::Command(QuickLaunch {
                    title: String::from("Copy"),
                    spec: CommandSpec::Custom {
                        custom: CustomCommand {
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
                spec: CommandSpec::Custom {
                    custom: CustomCommand {
                        program: String::from("bash"),
                        args: Vec::new(),
                        env: Vec::new(),
                        working_directory: None,
                    },
                },
            },
        };

        let settings = Settings::default();
        let _task = feature.reduce(
            QuickLaunchEvent::WizardSaveRequested(request),
            &ctx(&settings),
        );

        assert_eq!(feature.state.data().root.children.len(), 2);
        assert!(!feature.state.is_dirty());
    }

    #[test]
    fn given_editor_save_with_missing_target_when_reduced_then_state_is_unchanged()
     {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());

        let request = QuickLaunchWizardSaveRequest {
            tab_id: 42,
            target: QuickLaunchWizardSaveTarget::Edit {
                path: vec![String::from("Missing")],
            },
            command: sample_command(),
        };

        let settings = Settings::default();
        let _task = feature.reduce(
            QuickLaunchEvent::WizardSaveRequested(request),
            &ctx(&settings),
        );

        assert!(feature.state.data().root.children.is_empty());
        assert!(!feature.state.is_dirty());
    }

    #[test]
    fn given_error_tab_opened_and_closed_when_reduced_then_payload_is_cleaned()
    {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        let settings = Settings::default();

        let _task = feature.reduce(
            QuickLaunchEvent::OpenErrorTab {
                tab_id: 5,
                title: String::from("Err"),
                message: String::from("something went wrong"),
            },
            &ctx(&settings),
        );

        assert!(feature.state.error_tab(5).is_some());

        let _task = feature
            .reduce(QuickLaunchEvent::TabClosed { tab_id: 5 }, &ctx(&settings));

        assert!(feature.state.error_tab(5).is_none());
    }

    #[test]
    fn given_active_launch_when_tick_then_blink_nonce_advances() {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        feature.state.set_blink_nonce_for_tests(0);
        let cancel = Arc::new(AtomicBool::new(false));
        feature
            .state
            .begin_launch(vec![String::from("cmd")], cancel);

        let settings = Settings::default();
        let _task = feature.reduce(QuickLaunchEvent::Tick, &ctx(&settings));

        assert_eq!(feature.state.blink_nonce(), 1);
    }

    #[test]
    fn given_dirty_state_when_tick_then_persist_begins() {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        feature.state.mark_dirty();

        let settings = Settings::default();
        let _task = feature.reduce(QuickLaunchEvent::Tick, &ctx(&settings));

        // begin_persist sets in_flight; dirty is only cleared on complete_persist
        assert!(feature.state.is_persist_in_flight());
        assert!(feature.state.is_dirty());
    }

    #[test]
    fn given_launch_before_delay_when_tick_then_indicator_is_off() {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        let path = vec![String::from("Demo")];
        feature.state.set_blink_nonce_for_tests(4);
        feature.state.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 1,
                launch_ticks: 3,
                is_indicator_highlighted: true,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let settings = Settings::default();
        let _task = feature.reduce(QuickLaunchEvent::Tick, &ctx(&settings));

        let info = feature
            .state
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert_eq!(info.launch_ticks, 4);
        assert!(!info.is_indicator_highlighted);
    }

    #[test]
    fn given_launch_after_delay_when_tick_then_indicator_toggles_by_period() {
        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        let path = vec![String::from("Demo")];
        feature.state.set_blink_nonce_for_tests(2);
        feature.state.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 1,
                launch_ticks: 4,
                is_indicator_highlighted: false,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        let settings = Settings::default();

        let _task = feature.reduce(QuickLaunchEvent::Tick, &ctx(&settings));
        let info = feature
            .state
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert!(!info.is_indicator_highlighted);

        let _task = feature.reduce(QuickLaunchEvent::Tick, &ctx(&settings));
        let info = feature
            .state
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert!(!info.is_indicator_highlighted);

        let _task = feature.reduce(QuickLaunchEvent::Tick, &ctx(&settings));
        let info = feature
            .state
            .launching()
            .get(&path)
            .expect("launch info must exist");
        assert!(info.is_indicator_highlighted);
    }

    #[test]
    fn given_failed_setup_completion_when_reduced_then_launch_is_removed() {
        use super::super::errors::QuickLaunchError;

        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        let path = vec![String::from("Demo")];
        feature.state.launching_mut().insert(
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
        let settings = Settings::default();
        let _task = feature
            .reduce(QuickLaunchEvent::SetupCompleted(outcome), &ctx(&settings));

        assert!(!feature.state.launching().contains_key(&path));
    }

    #[test]
    fn given_canceled_launch_completion_when_reduced_then_result_is_ignored() {
        use super::super::errors::QuickLaunchError;

        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        let path = vec![String::from("Demo")];
        feature.state.launching_mut().insert(
            path.clone(),
            LaunchInfo {
                id: 11,
                launch_ticks: 0,
                is_indicator_highlighted: false,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );
        feature.state.cancel_launch(&path);

        let outcome = QuickLaunchSetupOutcome::Failed {
            path: path.clone(),
            launch_id: 11,
            command: Box::new(sample_command()),
            error: Arc::new(QuickLaunchError::Validation {
                message: String::from("Program is empty."),
            }),
        };
        let settings = Settings::default();
        let _task = feature
            .reduce(QuickLaunchEvent::SetupCompleted(outcome), &ctx(&settings));

        assert!(feature.state.launching().get(&path).is_none());
    }

    #[test]
    fn given_stale_launch_completion_when_reduced_then_current_launch_is_kept()
    {
        use super::super::errors::QuickLaunchError;

        let mut feature = QuickLaunchFeature::new(QuickLaunchState::default());
        let path = vec![String::from("Demo")];
        feature.state.launching_mut().insert(
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
        let settings = Settings::default();
        let _task = feature
            .reduce(QuickLaunchEvent::SetupCompleted(outcome), &ctx(&settings));

        let active = feature
            .state
            .launching()
            .get(&path)
            .expect("current launch must remain active");
        assert_eq!(active.id, 22);
    }
}
