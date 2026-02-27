use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::Task;
use iced::widget::operation;

use super::command::QuickLaunchCommand;
use super::errors::quick_launch_error_message;
use super::event::QuickLaunchEffect;
use super::model::{
    ContextMenuAction, ContextMenuTarget, LaunchInfo, NodePath,
    PreparedQuickLaunch, QuickLaunch, QuickLaunchFile, QuickLaunchFolder,
    QuickLaunchNode, QuickLaunchSetupOutcome, QuickLaunchWizardSaveRequest,
    QuickLaunchWizardSaveTarget,
};
use super::services::prepare_quick_launch_setup;
use super::state::{
    ContextMenuState, DragState, DropTarget, InlineEditKind, InlineEditState,
    QuickLaunchErrorState, QuickLaunchState,
};
use super::storage::save_quick_launches;
use super::wizard_model::build_command;

/// Runtime context for the quick launch reducer.
pub(crate) struct QuickLaunchCtx<'a> {
    /// Terminal backend settings used when launching commands.
    pub(crate) terminal_settings: &'a otty_ui_term::settings::Settings,
    /// Current sidebar cursor position (read-only snapshot).
    pub(crate) sidebar_cursor: iced::Point,
    /// Whether the sidebar is currently being resized.
    pub(crate) sidebar_is_resizing: bool,
}

/// Reduce a quick launch command into state updates and effects.
pub(crate) fn reduce(
    state: &mut QuickLaunchState,
    command: QuickLaunchCommand,
    ctx: &QuickLaunchCtx<'_>,
) -> Task<QuickLaunchEffect> {
    use QuickLaunchCommand::*;

    match command {
        OpenErrorTab {
            tab_id,
            title,
            message,
        } => {
            state.set_error_tab(
                tab_id,
                QuickLaunchErrorState::new(title, message),
            );
            Task::none()
        },
        TabClosed { tab_id } => {
            state.remove_error_tab(tab_id);
            state.wizard_mut().remove_tab(tab_id);
            Task::none()
        },
        CursorMoved { position } => {
            state.set_cursor(position);
            update_drag_state(state);
            Task::none()
        },
        NodeHovered { path } => {
            if ctx.sidebar_is_resizing {
                return Task::none();
            }
            state.set_hovered_path(Some(path));
            update_drag_drop_target(state);
            Task::none()
        },
        BackgroundPressed => {
            state.clear_context_menu();
            state.clear_inline_edit();
            state.clear_selected_path();
            Task::none()
        },
        BackgroundReleased => {
            if state.drag().map(|drag| drag.active).unwrap_or(false) {
                state.set_drop_target(Some(DropTarget::Root));
            }
            if finish_drag(state) {
                return Task::none();
            }
            state.clear_pressed_path();
            Task::none()
        },
        BackgroundRightClicked => {
            state.set_context_menu(Some(ContextMenuState {
                target: ContextMenuTarget::Background,
                cursor: ctx.sidebar_cursor,
            }));
            state.clear_selected_path();
            Task::none()
        },
        NodeRightClicked { path } => {
            let selected_path = path.clone();
            let Some(node) = state.data().node(&path) else {
                return Task::none();
            };
            let target = match node {
                QuickLaunchNode::Folder(_) => ContextMenuTarget::Folder(path),
                QuickLaunchNode::Command(_) => ContextMenuTarget::Command(path),
            };
            state.set_context_menu(Some(ContextMenuState {
                target,
                cursor: ctx.sidebar_cursor,
            }));
            state.set_selected_path(Some(selected_path));
            Task::none()
        },
        ContextMenuDismiss => {
            state.clear_context_menu();
            Task::none()
        },
        CancelInlineEdit => {
            state.clear_inline_edit();
            Task::none()
        },
        ResetInteractionState => {
            state.set_hovered_path(None);
            state.clear_pressed_path();
            state.clear_drag();
            state.clear_drop_target();
            Task::none()
        },
        HeaderCreateFolder => {
            let parent = selected_parent_path(state);
            begin_inline_create_folder(state, parent);
            focus_inline_edit(state)
        },
        HeaderCreateCommand => {
            let parent = selected_parent_path(state);
            Task::done(QuickLaunchEffect::OpenWizardCreateTab {
                parent_path: parent,
            })
        },
        DeleteSelected => {
            let Some(path) = state.selected_path_cloned() else {
                return Task::none();
            };
            let Some(node) = state.data().node(&path) else {
                return Task::none();
            };
            if matches!(node, QuickLaunchNode::Folder(_)) {
                remove_node(state, &path);
                state.clear_selected_path();
            }
            Task::none()
        },
        NodePressed { path } => {
            state.set_pressed_path(Some(path.clone()));
            state.set_selected_path(Some(path.clone()));
            state.set_drag(Some(DragState {
                source: path,
                origin: state.cursor(),
                active: false,
            }));
            Task::none()
        },
        NodeReleased { path } => {
            if finish_drag(state) {
                return Task::none();
            }
            let clicked = state
                .pressed_path()
                .map(|pressed| pressed == &path)
                .unwrap_or(false);
            state.clear_pressed_path();
            if clicked {
                return handle_node_left_click(
                    state,
                    ctx.terminal_settings,
                    path,
                );
            }
            Task::none()
        },
        InlineEditChanged(value) => {
            if let Some(edit) = state.inline_edit_mut() {
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
            handle_context_menu_action(state, ctx.terminal_settings, action)
        },
        SetupCompleted(outcome) => reduce_setup_completed(state, outcome),
        PersistCompleted => {
            state.complete_persist();
            Task::none()
        },
        PersistFailed(message) => {
            state.fail_persist();
            log::warn!("quick launches save failed: {message}");
            Task::none()
        },
        Tick => reduce_tick(state),

        // Wizard commands
        WizardInitializeCreate {
            tab_id,
            parent_path,
        } => {
            state.wizard_mut().initialize_create(tab_id, parent_path);
            Task::none()
        },
        WizardInitializeEdit {
            tab_id,
            path,
            command,
        } => {
            state.wizard_mut().initialize_edit(tab_id, path, &command);
            Task::none()
        },
        WizardCancel { tab_id } => {
            Task::done(QuickLaunchEffect::CloseTabRequested { tab_id })
        },
        WizardSave { tab_id } => reduce_wizard_save(state, tab_id),
        WizardSetError { tab_id, message } => {
            if let Some(editor) = state.wizard_mut().editor_mut(tab_id) {
                editor.set_error(message);
            }
            Task::none()
        },
        WizardUpdateTitle { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_title(value))
        },
        WizardUpdateProgram { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_program(value))
        },
        WizardUpdateHost { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_host(value))
        },
        WizardUpdateUser { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_user(value))
        },
        WizardUpdatePort { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_port(value))
        },
        WizardUpdateIdentityFile { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_identity_file(value))
        },
        WizardUpdateWorkingDirectory { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| {
                e.set_working_directory(value)
            })
        },
        WizardAddArg { tab_id } => {
            reduce_wizard_field(state, tab_id, |e| e.add_arg())
        },
        WizardRemoveArg { tab_id, index } => {
            reduce_wizard_field(state, tab_id, |e| e.remove_arg(index))
        },
        WizardUpdateArg {
            tab_id,
            index,
            value,
        } => reduce_wizard_field(state, tab_id, |e| e.update_arg(index, value)),
        WizardAddEnv { tab_id } => {
            reduce_wizard_field(state, tab_id, |e| e.add_env())
        },
        WizardRemoveEnv { tab_id, index } => {
            reduce_wizard_field(state, tab_id, |e| e.remove_env(index))
        },
        WizardUpdateEnvKey {
            tab_id,
            index,
            value,
        } => reduce_wizard_field(state, tab_id, |e| {
            e.update_env_key(index, value)
        }),
        WizardUpdateEnvValue {
            tab_id,
            index,
            value,
        } => reduce_wizard_field(state, tab_id, |e| {
            e.update_env_value(index, value)
        }),
        WizardAddExtraArg { tab_id } => {
            reduce_wizard_field(state, tab_id, |e| e.add_extra_arg())
        },
        WizardRemoveExtraArg { tab_id, index } => {
            reduce_wizard_field(state, tab_id, |e| e.remove_extra_arg(index))
        },
        WizardUpdateExtraArg {
            tab_id,
            index,
            value,
        } => reduce_wizard_field(state, tab_id, |e| {
            e.update_extra_arg(index, value)
        }),
        WizardSelectCommandType {
            tab_id,
            command_type,
        } => {
            if let Some(editor) = state.wizard_mut().editor_mut(tab_id) {
                if editor.is_create_mode() {
                    editor.set_command_type(command_type);
                }
            }
            Task::none()
        },
    }
}

// ---------------------------------------------------------------------------
// Wizard helpers
// ---------------------------------------------------------------------------

fn reduce_wizard_field(
    state: &mut QuickLaunchState,
    tab_id: u64,
    apply: impl FnOnce(&mut super::state::WizardEditorState),
) -> Task<QuickLaunchEffect> {
    if let Some(editor) = state.wizard_mut().editor_mut(tab_id) {
        apply(editor);
        editor.clear_error();
    }
    Task::none()
}

fn reduce_wizard_save(
    state: &mut QuickLaunchState,
    tab_id: u64,
) -> Task<QuickLaunchEffect> {
    let Some(editor) = state.wizard().editor(tab_id) else {
        return Task::none();
    };
    match build_command(editor) {
        Ok(command) => {
            let target = match editor.mode() {
                super::state::WizardMode::Create { parent_path } => {
                    QuickLaunchWizardSaveTarget::Create {
                        parent_path: parent_path.clone(),
                    }
                },
                super::state::WizardMode::Edit { path } => {
                    QuickLaunchWizardSaveTarget::Edit { path: path.clone() }
                },
            };
            let request = QuickLaunchWizardSaveRequest {
                tab_id,
                target,
                command,
            };
            // Route save back through the reducer for tree mutation
            apply_editor_save_request(state, request)
        },
        Err(err) => {
            if let Some(editor) = state.wizard_mut().editor_mut(tab_id) {
                editor.set_error(format!("{err}"));
            }
            Task::none()
        },
    }
}

// ---------------------------------------------------------------------------
// Tree interaction helpers
// ---------------------------------------------------------------------------

fn reduce_setup_completed(
    state: &mut QuickLaunchState,
    outcome: QuickLaunchSetupOutcome,
) -> Task<QuickLaunchEffect> {
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
            let command_title = command.title().to_string();
            Task::done(QuickLaunchEffect::OpenErrorTab {
                title: format!("Failed to launch \"{command_title}\""),
                message: quick_launch_error_message(&command, error.as_ref()),
            })
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
) -> Task<QuickLaunchEffect> {
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

    Task::done(QuickLaunchEffect::OpenCommandTerminalTab {
        title,
        settings,
        command,
    })
}

fn handle_node_left_click(
    state: &mut QuickLaunchState,
    terminal_settings: &otty_ui_term::settings::Settings,
    path: NodePath,
) -> Task<QuickLaunchEffect> {
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
    _terminal_settings: &otty_ui_term::settings::Settings,
    action: ContextMenuAction,
) -> Task<QuickLaunchEffect> {
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
            ContextMenuTarget::Folder(path) => {
                Task::done(QuickLaunchEffect::OpenWizardCreateTab {
                    parent_path: path,
                })
            },
            ContextMenuTarget::Command(path) => {
                let parent = path[..path.len().saturating_sub(1)].to_vec();
                Task::done(QuickLaunchEffect::OpenWizardCreateTab {
                    parent_path: parent,
                })
            },
            ContextMenuTarget::Background => {
                Task::done(QuickLaunchEffect::OpenWizardCreateTab {
                    parent_path: Vec::new(),
                })
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

fn reduce_tick(state: &mut QuickLaunchState) -> Task<QuickLaunchEffect> {
    let mut tasks: Vec<Task<QuickLaunchEffect>> = Vec::new();

    if state.has_active_launches() {
        state.advance_blink_nonce();
        update_launch_indicators(state);
    }

    if state.is_dirty() && !state.is_persist_in_flight() {
        state.begin_persist();
        tasks.push(request_persist_quick_launches(state.data().clone()));
    }

    if tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(tasks)
    }
}

// ---------------------------------------------------------------------------
// Tree mutation helpers
// ---------------------------------------------------------------------------

fn begin_inline_create_folder(
    state: &mut QuickLaunchState,
    parent_path: NodePath,
) {
    if !parent_path.is_empty() {
        if let Some(QuickLaunchNode::Folder(folder)) =
            state.data_mut().node_mut(&parent_path)
        {
            folder.expanded = true;
        }
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

fn apply_inline_edit(state: &mut QuickLaunchState) -> Task<QuickLaunchEffect> {
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
) -> Task<QuickLaunchEffect> {
    match request.target {
        QuickLaunchWizardSaveTarget::Create { parent_path } => {
            let Some(parent) = state.data_mut().folder_mut(&parent_path) else {
                return Task::done(QuickLaunchEffect::WizardSetError {
                    tab_id: request.tab_id,
                    message: String::from("Missing target folder."),
                });
            };
            let title =
                match parent.normalize_title(&request.command.title, None) {
                    Ok(title) => title,
                    Err(err) => {
                        return Task::done(QuickLaunchEffect::WizardSetError {
                            tab_id: request.tab_id,
                            message: format!("{err}"),
                        });
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
                    return Task::done(QuickLaunchEffect::WizardSetError {
                        tab_id: request.tab_id,
                        message: String::from("Missing parent folder."),
                    });
                };
                let current = path.last().map(String::as_str);
                if let Err(err) =
                    parent.normalize_title(&request.command.title, current)
                {
                    return Task::done(QuickLaunchEffect::WizardSetError {
                        tab_id: request.tab_id,
                        message: format!("{err}"),
                    });
                }
            }
            let Some(node) = state.data_mut().node_mut(&path) else {
                return Task::done(QuickLaunchEffect::WizardSetError {
                    tab_id: request.tab_id,
                    message: String::from("Command no longer exists."),
                });
            };
            *node = QuickLaunchNode::Command(request.command);
        },
    }
    state.mark_dirty();
    Task::done(QuickLaunchEffect::CloseTabRequested {
        tab_id: request.tab_id,
    })
}

fn toggle_folder_expanded(state: &mut QuickLaunchState, path: &[String]) {
    if let Some(QuickLaunchNode::Folder(folder)) =
        state.data_mut().node_mut(path)
    {
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

fn focus_inline_edit(state: &QuickLaunchState) -> Task<QuickLaunchEffect> {
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
    let mut new_path = target_path;
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
    let moved: Vec<(NodePath, LaunchInfo)> = state
        .launching()
        .iter()
        .filter(|(path, _)| is_prefix(source, path))
        .map(|(path, info)| (path.clone(), info.clone()))
        .collect();
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

fn request_persist_quick_launches(
    data: QuickLaunchFile,
) -> Task<QuickLaunchEffect> {
    Task::perform(
        async move {
            match save_quick_launches(&data) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("{err}")),
            }
        },
        |result| match result {
            Ok(()) => QuickLaunchEffect::PersistCompleted,
            Err(message) => QuickLaunchEffect::PersistFailed(message),
        },
    )
}

fn launch_quick_launch(
    state: &mut QuickLaunchState,
    terminal_settings: &otty_ui_term::settings::Settings,
    path: NodePath,
    command: QuickLaunch,
) -> Task<QuickLaunchEffect> {
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
        QuickLaunchEffect::SetupCompleted,
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
    if let Some(info) = state.launch_info(path) {
        if info.id != launch_id {
            return true;
        }
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

fn open_edit_command_tab(
    state: &QuickLaunchState,
    path: NodePath,
) -> Task<QuickLaunchEffect> {
    let Some(node) = state.data().node(&path).cloned() else {
        return Task::none();
    };
    let QuickLaunchNode::Command(command) = node else {
        return Task::none();
    };
    Task::done(QuickLaunchEffect::OpenWizardEditTab {
        path,
        command: Box::new(command),
    })
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
        let mut state = QuickLaunchState::default();
        state.data_mut().root.children.push(QuickLaunchNode::Folder(
            QuickLaunchFolder {
                title: String::from("Folder"),
                expanded: true,
                children: Vec::new(),
            },
        ));
        state.set_selected_path(Some(vec![String::from("Folder")]));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::DeleteSelected,
            &ctx(&settings),
        );

        assert!(state.data().root.children.is_empty());
        assert!(state.selected_path().is_none());
    }

    #[test]
    fn given_unknown_node_when_released_then_reducer_ignores_event() {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::NodeReleased {
                path: vec![String::from("Missing")],
            },
            &ctx(&settings),
        );
        assert!(state.selected_path().is_none());
        assert!(state.launching().is_empty());
    }

    #[test]
    fn given_editor_create_save_request_when_reduced_then_command_is_added() {
        let mut state = QuickLaunchState::default();
        state.data_mut().root = QuickLaunchFolder {
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
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardSaveRequested(request),
            &ctx(&settings),
        );

        assert_eq!(state.data().root.children.len(), 1);
        assert!(state.is_dirty());
    }

    #[test]
    fn given_error_tab_opened_and_closed_when_reduced_then_payload_is_cleaned()
    {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();

        let _task = reduce(
            &mut state,
            QuickLaunchCommand::OpenErrorTab {
                tab_id: 5,
                title: String::from("Err"),
                message: String::from("something went wrong"),
            },
            &ctx(&settings),
        );
        assert!(state.error_tab(5).is_some());

        let _task = reduce(
            &mut state,
            QuickLaunchCommand::TabClosed { tab_id: 5 },
            &ctx(&settings),
        );
        assert!(state.error_tab(5).is_none());
    }

    #[test]
    fn given_active_launch_when_tick_then_blink_nonce_advances() {
        let mut state = QuickLaunchState::default();
        state.set_blink_nonce_for_tests(0);
        let cancel = Arc::new(AtomicBool::new(false));
        state.begin_launch(vec![String::from("cmd")], cancel);

        let settings = Settings::default();
        let _task =
            reduce(&mut state, QuickLaunchCommand::Tick, &ctx(&settings));

        assert_eq!(state.blink_nonce(), 1);
    }

    #[test]
    fn given_dirty_state_when_tick_then_persist_begins() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();

        let settings = Settings::default();
        let _task =
            reduce(&mut state, QuickLaunchCommand::Tick, &ctx(&settings));

        assert!(state.is_persist_in_flight());
        assert!(state.is_dirty());
    }

    #[test]
    fn given_wizard_field_update_when_reduced_then_editor_is_updated() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateTitle {
                tab_id: 1,
                value: String::from("MyCmd"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert_eq!(editor.title(), "MyCmd");
    }

    #[test]
    fn given_wizard_save_with_empty_title_when_reduced_then_error_is_set() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardSave { tab_id: 1 },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert!(editor.error().is_some());
    }

    // -----------------------------------------------------------------------
    // Node interaction tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_background_pressed_when_reduced_then_selection_and_menu_are_cleared()
     {
        let mut state = QuickLaunchState::default();
        state.set_selected_path(Some(vec![String::from("cmd")]));
        state.set_context_menu(Some(ContextMenuState {
            target: ContextMenuTarget::Background,
            cursor: Point::ORIGIN,
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::BackgroundPressed,
            &ctx(&settings),
        );

        assert!(state.selected_path().is_none());
        assert!(state.context_menu().is_none());
    }

    #[test]
    fn given_node_pressed_when_reduced_then_selection_and_drag_are_set() {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        let path = vec![String::from("Demo")];

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::NodePressed { path: path.clone() },
            &ctx(&settings),
        );

        assert_eq!(state.selected_path(), Some(&path));
        assert!(state.pressed_path().is_some());
        assert!(state.drag().is_some());
    }

    #[test]
    fn given_node_hovered_when_sidebar_resizing_then_hover_not_set() {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();
        let mut resizing_ctx = ctx(&settings);
        resizing_ctx.sidebar_is_resizing = true;

        let _task = reduce(
            &mut state,
            QuickLaunchCommand::NodeHovered {
                path: vec![String::from("x")],
            },
            &resizing_ctx,
        );

        assert!(state.hovered_path().is_none());
    }

    #[test]
    fn given_node_hovered_when_not_resizing_then_hover_is_set() {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();

        let _task = reduce(
            &mut state,
            QuickLaunchCommand::NodeHovered {
                path: vec![String::from("x")],
            },
            &ctx(&settings),
        );

        assert_eq!(state.hovered_path(), Some(&vec![String::from("x")]));
    }

    #[test]
    fn given_node_right_clicked_on_command_when_reduced_then_context_menu_opens()
     {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        let path = vec![String::from("Demo")];

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::NodeRightClicked { path: path.clone() },
            &ctx(&settings),
        );

        assert!(state.context_menu().is_some());
        assert_eq!(state.selected_path(), Some(&path));
    }

    #[test]
    fn given_background_right_clicked_when_reduced_then_background_menu_opens()
    {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::BackgroundRightClicked,
            &ctx(&settings),
        );

        assert!(state.context_menu().is_some());
        assert!(state.selected_path().is_none());
    }

    #[test]
    fn given_context_menu_dismiss_when_reduced_then_menu_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.set_context_menu(Some(ContextMenuState {
            target: ContextMenuTarget::Background,
            cursor: Point::ORIGIN,
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::ContextMenuDismiss,
            &ctx(&settings),
        );

        assert!(state.context_menu().is_none());
    }

    #[test]
    fn given_cancel_inline_edit_when_reduced_then_edit_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.set_inline_edit(Some(InlineEditState {
            kind: InlineEditKind::CreateFolder {
                parent_path: Vec::new(),
            },
            value: String::from("test"),
            error: None,
            id: iced::widget::Id::unique(),
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::CancelInlineEdit,
            &ctx(&settings),
        );

        assert!(state.inline_edit().is_none());
    }

    #[test]
    fn given_reset_interaction_state_when_reduced_then_hover_and_drag_are_cleared()
     {
        let mut state = QuickLaunchState::default();
        state.set_hovered_path(Some(vec![String::from("x")]));
        state.set_pressed_path(Some(vec![String::from("x")]));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::ResetInteractionState,
            &ctx(&settings),
        );

        assert!(state.hovered_path().is_none());
        assert!(state.pressed_path().is_none());
    }

    // -----------------------------------------------------------------------
    // Inline edit tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_inline_edit_active_when_value_changed_then_value_updates() {
        let mut state = QuickLaunchState::default();
        state.set_inline_edit(Some(InlineEditState {
            kind: InlineEditKind::CreateFolder {
                parent_path: Vec::new(),
            },
            value: String::new(),
            error: Some(String::from("old error")),
            id: iced::widget::Id::unique(),
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::InlineEditChanged(String::from("NewFolder")),
            &ctx(&settings),
        );

        let edit = state.inline_edit().expect("edit should exist");
        assert_eq!(edit.value, "NewFolder");
        assert!(edit.error.is_none());
    }

    #[test]
    fn given_inline_create_folder_when_submitted_then_folder_is_created() {
        let mut state = QuickLaunchState::default();
        state.set_inline_edit(Some(InlineEditState {
            kind: InlineEditKind::CreateFolder {
                parent_path: Vec::new(),
            },
            value: String::from("MyFolder"),
            error: None,
            id: iced::widget::Id::unique(),
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::InlineEditSubmit,
            &ctx(&settings),
        );

        assert_eq!(state.data().root.children.len(), 1);
        assert_eq!(state.data().root.children[0].title(), "MyFolder");
        assert!(state.is_dirty());
        assert!(state.inline_edit().is_none());
    }

    #[test]
    fn given_inline_create_folder_with_empty_name_when_submitted_then_error_shown()
     {
        let mut state = QuickLaunchState::default();
        state.set_inline_edit(Some(InlineEditState {
            kind: InlineEditKind::CreateFolder {
                parent_path: Vec::new(),
            },
            value: String::from("   "),
            error: None,
            id: iced::widget::Id::unique(),
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::InlineEditSubmit,
            &ctx(&settings),
        );

        let edit = state.inline_edit().expect("edit should remain");
        assert!(edit.error.is_some());
    }

    #[test]
    fn given_inline_rename_when_submitted_then_node_is_renamed() {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        let path = vec![String::from("Demo")];
        state.set_inline_edit(Some(InlineEditState {
            kind: InlineEditKind::Rename { path },
            value: String::from("Renamed"),
            error: None,
            id: iced::widget::Id::unique(),
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::InlineEditSubmit,
            &ctx(&settings),
        );

        assert_eq!(state.data().root.children[0].title(), "Renamed");
        assert!(state.is_dirty());
    }

    // -----------------------------------------------------------------------
    // Context menu action tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_context_menu_remove_on_command_when_reduced_then_command_is_removed()
     {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        state.set_context_menu(Some(ContextMenuState {
            target: ContextMenuTarget::Command(vec![String::from("Demo")]),
            cursor: Point::ORIGIN,
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::ContextMenuAction(ContextMenuAction::Remove),
            &ctx(&settings),
        );

        assert!(state.data().root.children.is_empty());
    }

    #[test]
    fn given_context_menu_duplicate_on_command_when_reduced_then_duplicate_is_created()
     {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        state.set_context_menu(Some(ContextMenuState {
            target: ContextMenuTarget::Command(vec![String::from("Demo")]),
            cursor: Point::ORIGIN,
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::ContextMenuAction(ContextMenuAction::Duplicate),
            &ctx(&settings),
        );

        assert_eq!(state.data().root.children.len(), 2);
    }

    #[test]
    fn given_context_menu_delete_on_folder_when_reduced_then_folder_is_removed()
    {
        let mut state = QuickLaunchState::default();
        state.data_mut().root.children.push(QuickLaunchNode::Folder(
            QuickLaunchFolder {
                title: String::from("Dir"),
                expanded: true,
                children: Vec::new(),
            },
        ));
        state.set_context_menu(Some(ContextMenuState {
            target: ContextMenuTarget::Folder(vec![String::from("Dir")]),
            cursor: Point::ORIGIN,
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::ContextMenuAction(ContextMenuAction::Delete),
            &ctx(&settings),
        );

        assert!(state.data().root.children.is_empty());
    }

    #[test]
    fn given_context_menu_kill_on_running_command_when_reduced_then_launch_is_canceled()
     {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        let path = vec![String::from("Demo")];
        let cancel = Arc::new(AtomicBool::new(false));
        state.begin_launch(path.clone(), cancel.clone());
        state.set_context_menu(Some(ContextMenuState {
            target: ContextMenuTarget::Command(path),
            cursor: Point::ORIGIN,
        }));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::ContextMenuAction(ContextMenuAction::Kill),
            &ctx(&settings),
        );

        assert!(cancel.load(std::sync::atomic::Ordering::SeqCst));
    }

    // -----------------------------------------------------------------------
    // Folder toggle / node click tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_folder_when_clicked_then_expanded_state_toggles() {
        let mut state = QuickLaunchState::default();
        state.data_mut().root.children.push(QuickLaunchNode::Folder(
            QuickLaunchFolder {
                title: String::from("Dir"),
                expanded: true,
                children: Vec::new(),
            },
        ));
        let path = vec![String::from("Dir")];

        // Press then release to simulate click
        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::NodePressed { path: path.clone() },
            &ctx(&settings),
        );
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::NodeReleased { path: path.clone() },
            &ctx(&settings),
        );

        let folder = state.data().folder(&path).expect("folder should exist");
        assert!(!folder.is_expanded());
    }

    // -----------------------------------------------------------------------
    // Header create tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_header_create_folder_when_reduced_then_inline_edit_starts() {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::HeaderCreateFolder,
            &ctx(&settings),
        );

        assert!(state.inline_edit().is_some());
    }

    // -----------------------------------------------------------------------
    // Persist lifecycle tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_persist_completed_when_reduced_then_in_flight_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();
        state.begin_persist();

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::PersistCompleted,
            &ctx(&settings),
        );

        assert!(!state.is_persist_in_flight());
    }

    #[test]
    fn given_persist_failed_when_reduced_then_in_flight_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();
        state.begin_persist();

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::PersistFailed(String::from("disk full")),
            &ctx(&settings),
        );

        assert!(!state.is_persist_in_flight());
    }

    // -----------------------------------------------------------------------
    // Cursor / drag tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_cursor_moved_when_reduced_then_cursor_is_updated() {
        let mut state = QuickLaunchState::default();
        let pos = Point::new(100.0, 200.0);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::CursorMoved { position: pos },
            &ctx(&settings),
        );

        assert_eq!(state.cursor(), pos);
    }

    // -----------------------------------------------------------------------
    // Setup completed tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_canceled_setup_when_reduced_then_launch_is_removed() {
        let mut state = QuickLaunchState::default();
        let path = vec![String::from("cmd")];
        let cancel = Arc::new(AtomicBool::new(false));
        state.begin_launch(path.clone(), cancel);
        let launch_id = state
            .launching()
            .get(&path)
            .expect("launch should exist")
            .id;

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::SetupCompleted(
                QuickLaunchSetupOutcome::Canceled {
                    path: path.clone(),
                    launch_id,
                },
            ),
            &ctx(&settings),
        );

        assert!(state.launching().is_empty());
    }

    // -----------------------------------------------------------------------
    // Wizard lifecycle tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_wizard_initialize_create_when_reduced_then_editor_is_created() {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardInitializeCreate {
                tab_id: 10,
                parent_path: vec![],
            },
            &ctx(&settings),
        );

        assert!(state.wizard().editor(10).is_some());
    }

    #[test]
    fn given_wizard_initialize_edit_when_reduced_then_editor_is_loaded() {
        let mut state = QuickLaunchState::default();
        let cmd = sample_command();
        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardInitializeEdit {
                tab_id: 10,
                path: vec![String::from("Demo")],
                command: Box::new(cmd),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(10).expect("editor should exist");
        assert_eq!(editor.title(), "Demo");
    }

    #[test]
    fn given_wizard_cancel_when_reduced_then_close_tab_effect_is_emitted() {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();
        // WizardCancel emits CloseTabRequested effect
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardCancel { tab_id: 5 },
            &ctx(&settings),
        );
        // Can't inspect Task directly, but at least it doesn't panic
    }

    #[test]
    fn given_wizard_set_error_when_reduced_then_editor_error_is_set() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardSetError {
                tab_id: 1,
                message: String::from("oops"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert_eq!(editor.error(), Some("oops"));
    }

    #[test]
    fn given_wizard_select_command_type_in_create_mode_when_reduced_then_type_changes()
     {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardSelectCommandType {
                tab_id: 1,
                command_type: super::super::model::QuickLaunchType::Ssh,
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert_eq!(
            editor.command_type(),
            super::super::model::QuickLaunchType::Ssh
        );
    }

    // -----------------------------------------------------------------------
    // Wizard field update tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_wizard_update_program_when_reduced_then_program_field_changes() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateProgram {
                tab_id: 1,
                value: String::from("/usr/bin/zsh"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.program(), "/usr/bin/zsh");
    }

    #[test]
    fn given_wizard_add_arg_when_reduced_then_arg_count_increases() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardAddArg { tab_id: 1 },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args().len(), 1);
    }

    #[test]
    fn given_wizard_add_env_when_reduced_then_env_count_increases() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardAddEnv { tab_id: 1 },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.env().len(), 1);
    }

    // -----------------------------------------------------------------------
    // Launch indicator tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_launch_under_delay_threshold_when_checking_indicator_then_not_highlighted()
     {
        assert!(!should_highlight_launch_indicator(0, 0));
    }

    #[test]
    fn given_launch_past_delay_and_even_blink_when_checking_then_highlighted() {
        // 10 ticks * 200ms = 2000ms > 1000ms delay
        assert!(should_highlight_launch_indicator(10, 0));
    }

    // -----------------------------------------------------------------------
    // Editor save (edit mode) tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_editor_edit_save_request_when_reduced_then_command_is_updated() {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));

        let request = QuickLaunchWizardSaveRequest {
            tab_id: 7,
            target: QuickLaunchWizardSaveTarget::Edit {
                path: vec![String::from("Demo")],
            },
            command: QuickLaunch {
                title: String::from("Updated"),
                spec: CommandSpec::Custom {
                    custom: CustomCommand {
                        program: String::from("zsh"),
                        args: Vec::new(),
                        env: Vec::new(),
                        working_directory: None,
                    },
                },
            },
        };

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardSaveRequested(request),
            &ctx(&settings),
        );

        assert_eq!(state.data().root.children[0].title(), "Updated");
        assert!(state.is_dirty());
    }

    // -----------------------------------------------------------------------
    // Delete selected (command node) does nothing
    // -----------------------------------------------------------------------

    #[test]
    fn given_selected_command_when_delete_selected_then_command_is_not_removed()
    {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        state.set_selected_path(Some(vec![String::from("Demo")]));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::DeleteSelected,
            &ctx(&settings),
        );

        // DeleteSelected only removes folders, not commands
        assert_eq!(state.data().root.children.len(), 1);
    }

    #[test]
    fn given_no_selection_when_delete_selected_then_nothing_happens() {
        let mut state = QuickLaunchState::default();
        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::DeleteSelected,
            &ctx(&settings),
        );
        assert!(state.data().root.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Tab closed cleans up wizard and error state
    // -----------------------------------------------------------------------

    #[test]
    fn given_tab_closed_when_reduced_then_both_error_and_wizard_are_cleaned() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(5, vec![]);
        state.set_error_tab(
            5,
            QuickLaunchErrorState::new(String::from("E"), String::from("msg")),
        );

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::TabClosed { tab_id: 5 },
            &ctx(&settings),
        );

        assert!(state.error_tab(5).is_none());
        assert!(state.wizard().editor(5).is_none());
    }

    // -----------------------------------------------------------------------
    // Background released finishes drag
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_drag_when_background_released_then_pressed_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.set_pressed_path(Some(vec![String::from("x")]));

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::BackgroundReleased,
            &ctx(&settings),
        );

        assert!(state.pressed_path().is_none());
    }

    // -----------------------------------------------------------------------
    // Wizard save in edit mode with valid data
    // -----------------------------------------------------------------------

    #[test]
    fn given_wizard_with_filled_fields_when_save_then_command_created() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_title(String::from("NewCmd"));
            editor.set_program(String::from("bash"));
        }

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardSave { tab_id: 1 },
            &ctx(&settings),
        );

        assert_eq!(state.data().root.children.len(), 1);
        assert_eq!(state.data().root.children[0].title(), "NewCmd");
    }

    // -----------------------------------------------------------------------
    // Wizard SSH field updates
    // -----------------------------------------------------------------------

    #[test]
    fn given_wizard_ssh_fields_when_updated_then_values_change() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_command_type(super::super::model::QuickLaunchType::Ssh);
        }

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateHost {
                tab_id: 1,
                value: String::from("example.com"),
            },
            &ctx(&settings),
        );
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateUser {
                tab_id: 1,
                value: String::from("admin"),
            },
            &ctx(&settings),
        );
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdatePort {
                tab_id: 1,
                value: String::from("2222"),
            },
            &ctx(&settings),
        );
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateIdentityFile {
                tab_id: 1,
                value: String::from("~/.ssh/id_rsa"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.host(), "example.com");
        assert_eq!(ssh.user(), "admin");
        assert_eq!(ssh.port(), "2222");
        assert_eq!(ssh.identity_file(), "~/.ssh/id_rsa");
    }

    #[test]
    fn given_wizard_working_directory_when_updated_then_value_changes() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateWorkingDirectory {
                tab_id: 1,
                value: String::from("/tmp"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.working_directory(), "/tmp");
    }

    #[test]
    fn given_wizard_remove_arg_when_reduced_then_arg_is_removed() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.add_arg();
            editor.add_arg();
        }

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardRemoveArg {
                tab_id: 1,
                index: 0,
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args().len(), 1);
    }

    #[test]
    fn given_wizard_update_arg_when_reduced_then_arg_value_changes() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.add_arg();
        }

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateArg {
                tab_id: 1,
                index: 0,
                value: String::from("-v"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args()[0], "-v");
    }

    #[test]
    fn given_wizard_remove_env_when_reduced_then_env_is_removed() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.add_env();
        }

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardRemoveEnv {
                tab_id: 1,
                index: 0,
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert!(custom.env().is_empty());
    }

    #[test]
    fn given_wizard_update_env_when_reduced_then_env_key_and_value_change() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.add_env();
        }

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateEnvKey {
                tab_id: 1,
                index: 0,
                value: String::from("PATH"),
            },
            &ctx(&settings),
        );
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateEnvValue {
                tab_id: 1,
                index: 0,
                value: String::from("/usr/bin"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.env()[0].0, "PATH");
        assert_eq!(custom.env()[0].1, "/usr/bin");
    }

    #[test]
    fn given_wizard_ssh_extra_args_when_added_and_removed_then_list_updates() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_command_type(super::super::model::QuickLaunchType::Ssh);
        }

        let settings = Settings::default();
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardAddExtraArg { tab_id: 1 },
            &ctx(&settings),
        );
        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardUpdateExtraArg {
                tab_id: 1,
                index: 0,
                value: String::from("-o StrictHostKeyChecking=no"),
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.extra_args().len(), 1);
        assert_eq!(ssh.extra_args()[0], "-o StrictHostKeyChecking=no");

        let _task = reduce(
            &mut state,
            QuickLaunchCommand::WizardRemoveExtraArg {
                tab_id: 1,
                index: 0,
            },
            &ctx(&settings),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let ssh = editor.ssh().expect("ssh options should exist");
        assert!(ssh.extra_args().is_empty());
    }
}
