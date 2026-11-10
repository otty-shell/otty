use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use otty_app_domain::quick_launch::{NodePath, QuickLaunch, WizardTabInit};
use otty_app_domain::tab::TabMachine;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_term::settings::Settings;

use crate::constants::QUICK_LAUNCHES_TICK_MS;
use crate::ctx::QuickLaunchCtx;
use crate::event::{LaunchRequest, QuickLaunchIntent, QuickLaunchOutcome};
use crate::state::{
    ContextMenuState, DragState, InlineEditState, QuickLaunchState,
    SecretPromptState, WizardEditorState,
};
use crate::types::{
    ContextMenuAction, ContextMenuTarget, DropTarget, InlineEditKind,
    LaunchInfo, QuickLaunchFolder, QuickLaunchNode, QuickLaunchSetupOutcome,
    QuickLaunchWizardSaveRequest, QuickLaunchWizardSaveTarget, WizardMode,
};
use crate::wizard_draft::build_command;

/// What one intent reports, in order.
type Outcomes = Vec<QuickLaunchOutcome>;

/// Reduce a quick launch intent into state changes and what the frame does
/// next. When the last open overlay closes, it also reports
/// [`QuickLaunchOutcome::OverlayClosed`], last; a Rename or New folder from
/// the context menu opens the inline edit in its place, so it reports
/// nothing.
pub(crate) fn reduce(
    state: &mut QuickLaunchState,
    intent: QuickLaunchIntent,
    ctx: &QuickLaunchCtx,
) -> Outcomes {
    let had_overlay = state.has_overlay();

    let mut outcomes = reduce_intent(state, intent, ctx);

    if had_overlay && !state.has_overlay() {
        outcomes.push(QuickLaunchOutcome::OverlayClosed);
    }

    outcomes
}

/// Change the state for `intent` and return its outcomes, all but the
/// overlay closing.
fn reduce_intent(
    state: &mut QuickLaunchState,
    intent: QuickLaunchIntent,
    ctx: &QuickLaunchCtx,
) -> Outcomes {
    use QuickLaunchIntent::*;

    match intent {
        TabActivated { tab_id } => {
            state.wizard_mut().set_front_tab(Some(tab_id));
            Vec::new()
        },
        TabClosed {
            tab_id,
            new_active_id,
        } => {
            state.wizard_mut().remove_tab(tab_id);
            state.wizard_mut().set_front_tab(new_active_id);
            Vec::new()
        },
        CursorMoved { position } => {
            state.set_cursor(position);
            update_drag_state(state);
            Vec::new()
        },
        NodeHovered { path } => {
            if ctx.is_panel_resizing {
                return Vec::new();
            }
            state.set_hovered_path(Some(path));
            update_drag_drop_target(state);
            Vec::new()
        },
        BackgroundPressed => {
            state.clear_context_menu();
            state.clear_inline_edit();
            state.clear_selected_path();
            Vec::new()
        },
        BackgroundReleased => {
            if state.drag().map(|drag| drag.active).unwrap_or(false) {
                state.set_drop_target(Some(DropTarget::Root));
            }
            if finish_drag(state) {
                return Vec::new();
            }
            state.clear_pressed_path();
            Vec::new()
        },
        // The context menu takes the keyboard, so typed keys do not reach
        // the shell under it and Esc reaches the app.
        BackgroundRightClicked => {
            let context_menu_state = ContextMenuState::default()
                .with_target(ContextMenuTarget::Background)
                .with_cursor(ctx.cursor)
                .with_area_size(ctx.screen_size);
            state.set_context_menu(Some(context_menu_state));
            state.clear_selected_path();
            vec![QuickLaunchOutcome::TakeKeyboard]
        },
        NodeRightClicked { path } => {
            let selected_path = path.clone();
            let Some(node) = state.data().node(&path) else {
                return Vec::new();
            };
            let target = match node {
                QuickLaunchNode::Folder(_) => ContextMenuTarget::Folder(path),
                QuickLaunchNode::Command(_) => ContextMenuTarget::Command(path),
            };

            let context_menu_state = ContextMenuState::default()
                .with_target(target)
                .with_cursor(ctx.cursor)
                .with_area_size(ctx.screen_size);

            state.set_context_menu(Some(context_menu_state));
            state.set_selected_path(Some(selected_path));
            vec![QuickLaunchOutcome::TakeKeyboard]
        },
        ContextMenuDismiss => {
            state.clear_context_menu();
            Vec::new()
        },
        CancelInlineEdit => {
            state.clear_inline_edit();
            Vec::new()
        },
        ResetInteractionState => {
            state.set_hovered_path(None);
            state.clear_pressed_path();
            state.clear_drag();
            state.clear_drop_target();
            Vec::new()
        },
        HeaderCreateFolder => {
            let parent = selected_parent_path(state);
            begin_inline_create_folder(state, parent);
            focus_inline_edit(state)
        },
        HeaderCreateCommand => {
            let parent = selected_parent_path(state);
            open_create_editor_tab(parent)
        },
        NodePressed { path } => {
            state.set_pressed_path(Some(path.clone()));
            state.set_selected_path(Some(path.clone()));
            state.set_drag(Some(DragState {
                source: path,
                origin: state.cursor(),
                active: false,
            }));
            Vec::new()
        },
        NodeReleased { path } => {
            if finish_drag(state) {
                return Vec::new();
            }
            let clicked = state
                .pressed_path()
                .map(|pressed| pressed == &path)
                .unwrap_or(false);
            state.clear_pressed_path();
            if clicked {
                return handle_node_left_click(state, path);
            }
            Vec::new()
        },
        LaunchNode { path } => handle_node_left_click(state, path),
        InlineEditChanged(value) => {
            if let Some(edit) = state.inline_edit_mut() {
                edit.value = value;
                edit.error = None;
            }
            Vec::new()
        },
        InlineEditSubmit => apply_inline_edit(state),
        ContextMenuAction(action) => handle_context_menu_action(state, action),
        SetupCompleted(outcome) => reduce_setup_completed(state, outcome),
        SecretPromptChanged(value) => {
            if let Some(prompt) = state.secret_prompt_mut() {
                prompt.set_value(value);
            }
            Vec::new()
        },
        SecretPromptSubmit => reduce_secret_prompt_submit(state),
        SecretPromptCancel => {
            state.close_secret_prompt();
            Vec::new()
        },
        PersistCompleted => {
            state.complete_persist();
            Vec::new()
        },
        PersistFailed(message) => {
            log::warn!("quick launches save failed: {message}");
            if !state.fail_persist() {
                return Vec::new();
            }

            let text = i18n::with_error(Key::TplLaunchesSaveFailed, &message);
            vec![QuickLaunchOutcome::Failed(text)]
        },
        Tick => reduce_tick(state),
        // Wizard events
        WizardInitializeCreate {
            tab_id,
            parent_path,
        } => {
            state.wizard_mut().initialize_create(tab_id, parent_path);
            state.wizard_mut().set_front_tab(Some(tab_id));
            Vec::new()
        },
        WizardInitializeEdit {
            tab_id,
            path,
            command,
        } => {
            state.wizard_mut().initialize_edit(tab_id, path, &command);
            state.wizard_mut().set_front_tab(Some(tab_id));
            Vec::new()
        },
        WizardCancel { tab_id } => {
            vec![QuickLaunchOutcome::CloseTab { tab_id }]
        },
        WizardSave { tab_id } => reduce_wizard_save(state, tab_id),
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
        WizardArgInput { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_arg_input(value))
        },
        WizardArgSubmit { tab_id } => {
            reduce_wizard_field(state, tab_id, |e| e.submit_arg())
        },
        WizardArgBackspace { tab_id } => {
            reduce_wizard_field(state, tab_id, |e| e.backspace_arg())
        },
        WizardRemoveArg { tab_id, index } => {
            reduce_wizard_field(state, tab_id, |e| e.remove_arg(index))
        },
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
        WizardExtraArgInput { tab_id, value } => {
            reduce_wizard_field(state, tab_id, |e| e.set_extra_arg_input(value))
        },
        WizardExtraArgSubmit { tab_id } => {
            reduce_wizard_field(state, tab_id, |e| e.submit_extra_arg())
        },
        WizardExtraArgBackspace { tab_id } => {
            reduce_wizard_field(state, tab_id, |e| e.backspace_extra_arg())
        },
        WizardRemoveExtraArg { tab_id, index } => {
            reduce_wizard_field(state, tab_id, |e| e.remove_extra_arg(index))
        },
        WizardSelectExitBehavior {
            tab_id,
            exit_behavior,
        } => reduce_wizard_field(state, tab_id, |e| {
            e.set_exit_behavior(exit_behavior)
        }),
        WizardSelectCommandType {
            tab_id,
            command_type,
        } => {
            if let Some(editor) = state.wizard_mut().editor_mut(tab_id)
                && editor.is_create_mode()
            {
                editor.set_command_type(command_type);
            }
            Vec::new()
        },
    }
}

fn reduce_wizard_field(
    state: &mut QuickLaunchState,
    tab_id: u64,
    apply: impl FnOnce(&mut WizardEditorState),
) -> Outcomes {
    if let Some(editor) = state.wizard_mut().editor_mut(tab_id) {
        apply(editor);
        editor.clear_error();
    }
    Vec::new()
}

fn reduce_wizard_save(state: &mut QuickLaunchState, tab_id: u64) -> Outcomes {
    let Some(editor) = state.wizard_mut().editor_mut(tab_id) else {
        return Vec::new();
    };
    editor.commit_pending_args();

    let editor = &*editor;
    match build_command(editor) {
        Ok(command) => {
            let target = match editor.mode() {
                WizardMode::Create { parent_path } => {
                    QuickLaunchWizardSaveTarget::Create {
                        parent_path: parent_path.clone(),
                    }
                },
                WizardMode::Edit { path } => {
                    QuickLaunchWizardSaveTarget::Edit { path: path.clone() }
                },
            };
            let request = QuickLaunchWizardSaveRequest {
                tab_id,
                target,
                command,
            };
            // Pass the save back through the reducer for tree mutation
            apply_editor_save_request(state, request)
        },
        Err(err) => {
            if let Some(editor) = state.wizard_mut().editor_mut(tab_id) {
                editor.set_error(format!("{err}"));
            }
            Vec::new()
        },
    }
}

// ---------------------------------------------------------------------------
// Tree interaction helpers
// ---------------------------------------------------------------------------

fn reduce_setup_completed(
    state: &mut QuickLaunchState,
    outcome: QuickLaunchSetupOutcome,
) -> Outcomes {
    match outcome {
        QuickLaunchSetupOutcome::Prepared {
            path,
            launch_id,
            command,
            settings,
        } => handle_prepared_quick_launch(
            state, &path, launch_id, &command, settings,
        ),
        QuickLaunchSetupOutcome::Failed {
            path,
            launch_id,
            command,
            error,
        } => {
            if should_skip_launch_result(state, &path, launch_id) {
                return Vec::new();
            }

            let text = i18n::launch_failed(command.title(), &error);
            vec![QuickLaunchOutcome::Failed(text)]
        },
        QuickLaunchSetupOutcome::SecretRequired {
            path,
            launch_id,
            command,
            request,
        } => {
            if should_skip_launch_result(state, &path, launch_id) {
                return Vec::new();
            }

            let prompt = SecretPromptState::new(path, command, request);
            let input_id = prompt.input_id().clone();
            state.open_secret_prompt(prompt);
            vec![QuickLaunchOutcome::FocusInput(input_id)]
        },
        QuickLaunchSetupOutcome::Canceled { path, launch_id } => {
            let _ = should_skip_launch_result(state, &path, launch_id);
            Vec::new()
        },
    }
}

/// Relaunch the prompted command with the typed secret.
fn reduce_secret_prompt_submit(state: &mut QuickLaunchState) -> Outcomes {
    if state
        .secret_prompt()
        .is_none_or(|prompt| prompt.value().is_empty())
    {
        return Vec::new();
    }

    let Some(prompt) = state.take_secret_prompt() else {
        return Vec::new();
    };
    let (path, command, secret) = prompt.into_launch();

    launch_quick_launch(state, path, command, Some(secret))
}

/// A prepared launch opens its tab, on the SSH host it connects to or on
/// this computer, unless a newer run of it or a kill made it stale.
fn handle_prepared_quick_launch(
    state: &mut QuickLaunchState,
    path: &[String],
    launch_id: u64,
    command: &QuickLaunch,
    settings: Box<Settings>,
) -> Outcomes {
    if should_skip_launch_result(state, path, launch_id) {
        return Vec::new();
    }

    let machine = match command.ssh_host() {
        Some(host) => TabMachine::Ssh(host.to_string()),
        None => TabMachine::ThisComputer,
    };

    vec![QuickLaunchOutcome::LaunchReady {
        title: command.title().to_string(),
        settings,
        exit_behavior: command.exit_behavior(),
        machine,
    }]
}

fn handle_node_left_click(
    state: &mut QuickLaunchState,
    path: NodePath,
) -> Outcomes {
    let Some(node) = state.data().node(&path).cloned() else {
        return Vec::new();
    };

    if matches!(state.inline_edit(), Some(edit) if inline_edit_matches(edit, &path))
    {
        return Vec::new();
    }

    state.clear_inline_edit();
    state.clear_context_menu();
    state.set_selected_path(Some(path.clone()));

    match node {
        QuickLaunchNode::Folder(_) => {
            toggle_folder_expanded(state, &path);
            state.mark_dirty();
            Vec::new()
        },
        QuickLaunchNode::Command(command) => {
            launch_quick_launch(state, path, command, None)
        },
    }
}

fn handle_context_menu_action(
    state: &mut QuickLaunchState,
    action: ContextMenuAction,
) -> Outcomes {
    let Some(menu) = state.context_menu_cloned() else {
        return Vec::new();
    };
    state.clear_context_menu();

    match action {
        ContextMenuAction::Edit => match menu.target() {
            ContextMenuTarget::Command(path) => {
                open_edit_command_tab(state, path)
            },
            _ => Vec::new(),
        },
        ContextMenuAction::Rename => match menu.target() {
            ContextMenuTarget::Command(path)
            | ContextMenuTarget::Folder(path) => {
                begin_inline_rename(state, path);
                focus_inline_edit(state)
            },
            ContextMenuTarget::Background => Vec::new(),
        },
        ContextMenuAction::Duplicate => match menu.target() {
            ContextMenuTarget::Command(path) => {
                duplicate_command(state, &path);
                Vec::new()
            },
            _ => Vec::new(),
        },
        ContextMenuAction::Remove => match menu.target() {
            ContextMenuTarget::Command(path) => {
                remove_node(state, &path);
                Vec::new()
            },
            _ => Vec::new(),
        },
        ContextMenuAction::Delete => match menu.target() {
            ContextMenuTarget::Folder(path) => {
                remove_node(state, &path);
                Vec::new()
            },
            _ => Vec::new(),
        },
        ContextMenuAction::CreateFolder => {
            let parent = match menu.target() {
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
        ContextMenuAction::CreateCommand => match menu.target() {
            ContextMenuTarget::Folder(path) => open_create_editor_tab(path),
            ContextMenuTarget::Command(path) => {
                let parent = path[..path.len().saturating_sub(1)].to_vec();
                open_create_editor_tab(parent)
            },
            ContextMenuTarget::Background => open_create_editor_tab(Vec::new()),
        },
        ContextMenuAction::Kill => match menu.target() {
            ContextMenuTarget::Command(path) => {
                state.cancel_launch(&path);
                Vec::new()
            },
            _ => Vec::new(),
        },
    }
}

fn reduce_tick(state: &mut QuickLaunchState) -> Outcomes {
    if state.has_active_launches() {
        state.advance_blink_nonce();
        update_launch_indicators(state);
    }

    if state.is_dirty() && !state.is_persist_in_flight() {
        state.begin_persist();

        return vec![QuickLaunchOutcome::SaveLaunches(state.data().clone())];
    }

    Vec::new()
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

fn apply_inline_edit(state: &mut QuickLaunchState) -> Outcomes {
    let Some(edit) = state.take_inline_edit() else {
        return Vec::new();
    };
    match edit.kind {
        InlineEditKind::CreateFolder { parent_path } => {
            let Some(parent) = state.data_mut().folder_mut(&parent_path) else {
                return Vec::new();
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
                    Vec::new()
                },
                Err(err) => {
                    state.set_inline_edit(Some(InlineEditState {
                        kind: InlineEditKind::CreateFolder { parent_path },
                        value: edit.value,
                        error: Some(format!("{err}")),
                        id: edit.id,
                    }));
                    Vec::new()
                },
            }
        },
        InlineEditKind::Rename { path } => {
            let Some(parent) = state.data_mut().parent_folder_mut(&path) else {
                return Vec::new();
            };
            let current_title = path.last().cloned().unwrap_or_default();
            match parent.normalize_title(&edit.value, Some(&current_title)) {
                Ok(title) => {
                    let mut renamed_path = path.clone();
                    if let Some(last) = renamed_path.last_mut() {
                        *last = title.clone();
                    }
                    if let Some(node) = state.data_mut().node_mut(&path) {
                        node.rename(title);
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
                    Vec::new()
                },
                Err(err) => {
                    state.set_inline_edit(Some(InlineEditState {
                        kind: InlineEditKind::Rename { path },
                        value: edit.value,
                        error: Some(format!("{err}")),
                        id: edit.id,
                    }));
                    Vec::new()
                },
            }
        },
    }
}

fn apply_editor_save_request(
    state: &mut QuickLaunchState,
    request: QuickLaunchWizardSaveRequest,
) -> Outcomes {
    let tab_id = request.tab_id;
    match request.target {
        QuickLaunchWizardSaveTarget::Create { parent_path } => {
            let Some(parent) = state.data_mut().folder_mut(&parent_path) else {
                set_wizard_error(
                    state,
                    tab_id,
                    i18n::t(Key::ErrMissingTargetFolder),
                );
                return Vec::new();
            };
            let title =
                match parent.normalize_title(request.command.title(), None) {
                    Ok(title) => title,
                    Err(err) => {
                        set_wizard_error(state, tab_id, &format!("{err}"));
                        return Vec::new();
                    },
                };
            let mut command = request.command;
            command.rename(title);
            parent.children.push(QuickLaunchNode::Command(command));
        },
        QuickLaunchWizardSaveTarget::Edit { path } => {
            {
                let Some(parent) = state.data_mut().parent_folder_mut(&path)
                else {
                    set_wizard_error(
                        state,
                        tab_id,
                        i18n::t(Key::ErrMissingParentFolder),
                    );
                    return Vec::new();
                };
                let current = path.last().map(String::as_str);
                if let Err(err) =
                    parent.normalize_title(request.command.title(), current)
                {
                    set_wizard_error(state, tab_id, &format!("{err}"));
                    return Vec::new();
                }
            }
            let Some(node) = state.data_mut().node_mut(&path) else {
                set_wizard_error(
                    state,
                    tab_id,
                    i18n::t(Key::ErrCommandNoLongerExists),
                );
                return Vec::new();
            };
            *node = QuickLaunchNode::Command(request.command);
        },
    }
    state.mark_dirty();
    vec![QuickLaunchOutcome::CloseTab { tab_id }]
}

fn set_wizard_error(state: &mut QuickLaunchState, tab_id: u64, message: &str) {
    if let Some(editor) = state.wizard_mut().editor_mut(tab_id) {
        editor.set_error(message.to_string());
    }
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

fn focus_inline_edit(state: &QuickLaunchState) -> Outcomes {
    let Some(edit) = state.inline_edit() else {
        return Vec::new();
    };
    vec![QuickLaunchOutcome::FocusInput(edit.id.clone())]
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
    clone.rename(duplicate_title(parent, command.title()));
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

fn launch_quick_launch(
    state: &mut QuickLaunchState,
    path: NodePath,
    command: QuickLaunch,
    secret: Option<String>,
) -> Outcomes {
    if state.is_launching(&path) {
        return Vec::new();
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let launch_id = state.begin_launch(path.clone(), cancel.clone());

    let request = LaunchRequest::new(command, path, launch_id, secret, cancel);
    vec![QuickLaunchOutcome::PrepareLaunch(request)]
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
    let launch_age_ms = launch_ticks.saturating_mul(QUICK_LAUNCHES_TICK_MS);
    if launch_age_ms < LAUNCH_ICON_DELAY_MS {
        return false;
    }
    let blink_step = (blink_nonce as u128 * QUICK_LAUNCHES_TICK_MS as u128)
        / LAUNCH_ICON_BLINK_MS;
    blink_step.is_multiple_of(2)
}

/// Open the launch editor of a saved launch, named after it.
fn open_edit_command_tab(state: &QuickLaunchState, path: NodePath) -> Outcomes {
    let Some(node) = state.data().node(&path).cloned() else {
        return Vec::new();
    };
    let QuickLaunchNode::Command(command) = node else {
        return Vec::new();
    };

    vec![QuickLaunchOutcome::OpenEditorTab {
        title: i18n::edit_tab_title(command.title()),
        init: WizardTabInit::Edit {
            path,
            command: Box::new(command),
        },
    }]
}

/// Open the launch editor of a new launch in the folder at `parent_path`.
fn open_create_editor_tab(parent_path: NodePath) -> Outcomes {
    vec![QuickLaunchOutcome::OpenEditorTab {
        title: i18n::t(Key::QuickLaunchCreateTabTitle).to_string(),
        init: WizardTabInit::Create { parent_path },
    }]
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use iced::{Point, Size};
    use otty_app_domain::exit_behavior::ExitBehavior;
    use otty_app_domain::quick_launch::{
        CommandSpec, CustomCommand, SshCommand,
    };

    use super::*;
    use crate::types::{SshSecretKind, SshSecretRequest};

    /// Size of the main area in the tests.
    const SCREEN: Size = Size::new(1024.0, 768.0);

    fn ctx() -> QuickLaunchCtx {
        QuickLaunchCtx {
            cursor: Point::ORIGIN,
            is_panel_resizing: false,
            screen_size: SCREEN,
        }
    }

    fn sample_command() -> QuickLaunch {
        QuickLaunch::new(
            String::from("Demo"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("bash"),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        )
    }

    #[test]
    fn given_unknown_node_when_released_then_reducer_ignores_event() {
        let mut state = QuickLaunchState::default();
        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::NodeReleased {
                path: vec![String::from("Missing")],
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert!(state.selected_path().is_none());
        assert!(state.launching().is_empty());
    }

    #[test]
    fn given_editor_create_save_request_when_applied_then_command_is_added() {
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

        let outcomes = apply_editor_save_request(&mut state, request);

        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::CloseTab { tab_id: 7 }]
        ));
        assert_eq!(state.data().root.children.len(), 1);
        assert!(state.is_dirty());
    }

    #[test]
    fn given_active_launch_when_tick_then_blink_nonce_advances() {
        let mut state = QuickLaunchState::default();
        state.set_blink_nonce_for_tests(0);
        let cancel = Arc::new(AtomicBool::new(false));
        state.begin_launch(vec![String::from("cmd")], cancel);

        let outcomes = reduce(&mut state, QuickLaunchIntent::Tick, &ctx());

        assert!(outcomes.is_empty());
        assert_eq!(state.blink_nonce(), 1);
    }

    #[test]
    fn given_dirty_state_when_tick_then_persist_begins() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();

        let _outcomes = reduce(&mut state, QuickLaunchIntent::Tick, &ctx());

        assert!(state.is_persist_in_flight());
        assert!(state.is_dirty());
    }

    #[test]
    fn given_wizard_field_update_when_reduced_then_editor_is_updated() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateTitle {
                tab_id: 1,
                value: String::from("MyCmd"),
            },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert!(outcomes.is_empty());
        assert_eq!(editor.title(), "MyCmd");
    }

    #[test]
    fn given_wizard_save_with_empty_title_when_reduced_then_error_is_set() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardSave { tab_id: 1 },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert!(outcomes.is_empty());
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
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(ContextMenuTarget::Background)
                .with_cursor(Point::ORIGIN),
        ));

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::BackgroundPressed, &ctx());

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

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::NodePressed { path: path.clone() },
            &ctx(),
        );

        assert_eq!(state.selected_path(), Some(&path));
        assert!(state.pressed_path().is_some());
        assert!(state.drag().is_some());
    }

    #[test]
    fn given_node_hovered_when_sidebar_resizing_then_hover_not_set() {
        let mut state = QuickLaunchState::default();
        let mut resizing_ctx = ctx();
        resizing_ctx.is_panel_resizing = true;

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::NodeHovered {
                path: vec![String::from("x")],
            },
            &resizing_ctx,
        );

        assert!(state.hovered_path().is_none());
    }

    #[test]
    fn given_node_hovered_when_not_resizing_then_hover_is_set() {
        let mut state = QuickLaunchState::default();

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::NodeHovered {
                path: vec![String::from("x")],
            },
            &ctx(),
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

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::NodeRightClicked { path: path.clone() },
            &ctx(),
        );

        assert!(state.context_menu().is_some());
        assert_eq!(state.selected_path(), Some(&path));
    }

    #[test]
    fn given_background_right_clicked_when_reduced_then_background_menu_opens()
    {
        let mut state = QuickLaunchState::default();
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::BackgroundRightClicked,
            &ctx(),
        );

        assert!(state.context_menu().is_some());
        assert!(state.selected_path().is_none());
    }

    /// The context menu takes the keyboard from the focused terminal, so
    /// typed keys do not reach its shell and Esc reaches the app.
    #[test]
    fn opening_the_context_menu_takes_the_keyboard_focus() {
        let mut on_launch = QuickLaunchState::default();
        on_launch
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        let mut on_background = QuickLaunchState::default();

        let launch_menu = reduce(
            &mut on_launch,
            QuickLaunchIntent::NodeRightClicked {
                path: vec![String::from("Demo")],
            },
            &ctx(),
        );
        let background_menu = reduce(
            &mut on_background,
            QuickLaunchIntent::BackgroundRightClicked,
            &ctx(),
        );

        assert!(matches!(
            launch_menu.as_slice(),
            [QuickLaunchOutcome::TakeKeyboard]
        ));
        assert!(matches!(
            background_menu.as_slice(),
            [QuickLaunchOutcome::TakeKeyboard]
        ));
    }

    /// The menu opens at the pointer and is kept inside the main area it
    /// opened in, read when it opens.
    #[test]
    fn a_context_menu_opens_at_the_pointer_inside_the_main_area() {
        let mut state = QuickLaunchState::default();
        let at = QuickLaunchCtx {
            cursor: Point::new(30.0, 40.0),
            ..ctx()
        };

        let _keyboard =
            reduce(&mut state, QuickLaunchIntent::BackgroundRightClicked, &at);

        let menu = state.context_menu().expect("the menu should open");
        assert_eq!(menu.cursor(), Point::new(30.0, 40.0));
        assert_eq!(menu.area_size(), SCREEN);
    }

    #[test]
    fn given_context_menu_dismiss_when_reduced_then_menu_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(ContextMenuTarget::Background)
                .with_cursor(Point::ORIGIN),
        ));

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::ContextMenuDismiss, &ctx());

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

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::CancelInlineEdit, &ctx());

        assert!(state.inline_edit().is_none());
    }

    #[test]
    fn given_reset_interaction_state_when_reduced_then_hover_and_drag_are_cleared()
     {
        let mut state = QuickLaunchState::default();
        state.set_hovered_path(Some(vec![String::from("x")]));
        state.set_pressed_path(Some(vec![String::from("x")]));

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::ResetInteractionState,
            &ctx(),
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

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::InlineEditChanged(String::from("NewFolder")),
            &ctx(),
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

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::InlineEditSubmit, &ctx());

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

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::InlineEditSubmit, &ctx());

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

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::InlineEditSubmit, &ctx());

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
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(ContextMenuTarget::Command(vec![String::from(
                    "Demo",
                )]))
                .with_cursor(Point::ORIGIN),
        ));

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Remove),
            &ctx(),
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
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(ContextMenuTarget::Command(vec![String::from(
                    "Demo",
                )]))
                .with_cursor(Point::ORIGIN),
        ));

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Duplicate),
            &ctx(),
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
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(ContextMenuTarget::Folder(vec![String::from(
                    "Dir",
                )]))
                .with_cursor(Point::ORIGIN),
        ));

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Delete),
            &ctx(),
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
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(ContextMenuTarget::Command(path))
                .with_cursor(Point::ORIGIN),
        ));

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Kill),
            &ctx(),
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
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::NodePressed { path: path.clone() },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::NodeReleased { path: path.clone() },
            &ctx(),
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
        let outcomes =
            reduce(&mut state, QuickLaunchIntent::HeaderCreateFolder, &ctx());

        let edit = state.inline_edit().expect("the name field should show");
        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::FocusInput(id)] if *id == edit.id
        ));
    }

    /// New launch opens the editor for a launch in the selected folder, or
    /// in the folder of the selected launch.
    #[test]
    fn new_launch_opens_the_editor_in_the_selected_folder() {
        let mut in_folder = state_with_root(vec![folder_node(
            "servers",
            vec![command_node("prod")],
        )]);
        in_folder.set_selected_path(Some(path(&["servers", "prod"])));
        let mut at_top = QuickLaunchState::default();

        let nested = reduce(
            &mut in_folder,
            QuickLaunchIntent::HeaderCreateCommand,
            &ctx(),
        );
        let top =
            reduce(&mut at_top, QuickLaunchIntent::HeaderCreateCommand, &ctx());

        let title = i18n::t(Key::QuickLaunchCreateTabTitle);
        assert!(matches!(
            nested.as_slice(),
            [QuickLaunchOutcome::OpenEditorTab {
                title: tab_title,
                init: WizardTabInit::Create { parent_path },
            }] if tab_title == title && *parent_path == path(&["servers"])
        ));
        assert!(matches!(
            top.as_slice(),
            [QuickLaunchOutcome::OpenEditorTab {
                init: WizardTabInit::Create { parent_path },
                ..
            }] if parent_path.is_empty()
        ));
    }

    /// Edit in the menu of a launch opens its editor, named after it, with
    /// the launch as it is saved.
    #[test]
    fn edit_from_the_menu_opens_the_editor_of_the_launch() {
        let mut state = state_with_command_menu();

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Edit),
            &ctx(),
        );

        let title = i18n::edit_tab_title("Demo");
        assert!(matches!(
            outcomes.as_slice(),
            [
                QuickLaunchOutcome::OpenEditorTab {
                    title: tab_title,
                    init: WizardTabInit::Edit { path: edited, command },
                },
                QuickLaunchOutcome::OverlayClosed,
            ] if *tab_title == title
                && *edited == path(&["Demo"])
                && command.title() == "Demo"
        ));
    }

    // -----------------------------------------------------------------------
    // Persist lifecycle tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_persist_completed_when_reduced_then_in_flight_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();
        state.begin_persist();

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::PersistCompleted, &ctx());

        assert!(!state.is_persist_in_flight());
    }

    #[test]
    fn given_persist_failed_when_reduced_then_in_flight_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();
        state.begin_persist();

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::PersistFailed(String::from("disk full")),
            &ctx(),
        );

        assert!(!state.is_persist_in_flight());
    }

    /// Start a save of the dirty tree, let it fail and return what that
    /// leads to.
    fn fail_a_save(state: &mut QuickLaunchState) -> Outcomes {
        state.begin_persist();

        reduce(
            state,
            QuickLaunchIntent::PersistFailed(String::from("disk full")),
            &ctx(),
        )
    }

    /// The save tries again on every tick while it fails, so only the first
    /// failure since the last good save tells the user; a good save starts
    /// over.
    #[test]
    fn only_the_first_failed_save_since_a_good_one_tells_the_user() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();

        let first = fail_a_save(&mut state);
        let retry = fail_a_save(&mut state);
        state.begin_persist();
        let _saved =
            reduce(&mut state, QuickLaunchIntent::PersistCompleted, &ctx());
        state.mark_dirty();
        let after_a_good_save = fail_a_save(&mut state);

        let expected =
            i18n::with_error(Key::TplLaunchesSaveFailed, "disk full");
        assert!(matches!(
            first.as_slice(),
            [QuickLaunchOutcome::Failed(text)]
                if *text == expected
        ));
        assert!(retry.is_empty());
        assert!(matches!(
            after_a_good_save.as_slice(),
            [QuickLaunchOutcome::Failed(text)]
                if *text == expected
        ));
        assert!(state.is_dirty());
    }

    // -----------------------------------------------------------------------
    // Cursor / drag tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_cursor_moved_when_reduced_then_cursor_is_updated() {
        let mut state = QuickLaunchState::default();
        let pos = Point::new(100.0, 200.0);

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::CursorMoved { position: pos },
            &ctx(),
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

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::Canceled {
                    path: path.clone(),
                    launch_id,
                },
            ),
            &ctx(),
        );

        assert!(state.launching().is_empty());
    }

    // -----------------------------------------------------------------------
    // Wizard lifecycle tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_wizard_initialize_create_when_reduced_then_editor_is_created() {
        let mut state = QuickLaunchState::default();
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardInitializeCreate {
                tab_id: 10,
                parent_path: vec![],
            },
            &ctx(),
        );

        assert!(state.wizard().editor(10).is_some());
        assert_eq!(state.wizard().front_tab_id(), Some(10));
    }

    #[test]
    fn given_wizard_initialize_edit_when_reduced_then_editor_is_loaded() {
        let mut state = QuickLaunchState::default();
        let cmd = sample_command();
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardInitializeEdit {
                tab_id: 10,
                path: vec![String::from("Demo")],
                command: Box::new(cmd),
            },
            &ctx(),
        );

        let editor = state.wizard().editor(10).expect("editor should exist");
        assert_eq!(editor.title(), "Demo");
        assert_eq!(state.wizard().front_tab_id(), Some(10));
    }

    #[test]
    fn given_wizard_cancel_when_reduced_then_close_tab_effect_is_emitted() {
        let mut state = QuickLaunchState::default();

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardCancel { tab_id: 5 },
            &ctx(),
        );

        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::CloseTab { tab_id: 5 }]
        ));
    }

    #[test]
    fn given_wizard_select_command_type_in_create_mode_when_reduced_then_type_changes()
     {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardSelectCommandType {
                tab_id: 1,
                command_type: super::super::types::QuickLaunchType::Ssh,
            },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert_eq!(
            editor.command_type(),
            super::super::types::QuickLaunchType::Ssh
        );
    }

    #[test]
    fn given_wizard_select_exit_behavior_when_reduced_then_choice_is_stored() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_error(String::from("stale"));
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardSelectExitBehavior {
                tab_id: 1,
                exit_behavior: ExitBehavior::CloseOnSuccess,
            },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        assert_eq!(editor.exit_behavior(), ExitBehavior::CloseOnSuccess);
        assert_eq!(editor.error(), None);
    }

    // -----------------------------------------------------------------------
    // Wizard field update tests
    // -----------------------------------------------------------------------

    #[test]
    fn given_wizard_update_program_when_reduced_then_program_field_changes() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateProgram {
                tab_id: 1,
                value: String::from("/usr/bin/zsh"),
            },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.program(), "/usr/bin/zsh");
    }

    #[test]
    fn given_typed_arg_when_submitted_then_arg_is_added_and_input_cleared() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardArgInput {
                tab_id: 1,
                value: String::from("  -v "),
            },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardArgSubmit { tab_id: 1 },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args(), &[String::from("-v")]);
        assert_eq!(custom.args_list().input(), "");
    }

    #[test]
    fn given_blank_arg_when_submitted_then_nothing_is_added() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardArgInput {
                tab_id: 1,
                value: String::from("   "),
            },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardArgSubmit { tab_id: 1 },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert!(custom.args().is_empty());
    }

    #[test]
    fn given_empty_input_when_arg_backspace_then_last_arg_is_removed() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_arg_input(String::from("-a"));
            editor.submit_arg();
            editor.set_arg_input(String::from("-b"));
            editor.submit_arg();
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardArgBackspace { tab_id: 1 },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args(), &[String::from("-a")]);
    }

    #[test]
    fn given_text_in_input_when_arg_backspace_then_args_are_kept() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_arg_input(String::from("-a"));
            editor.submit_arg();
            editor.set_arg_input(String::from("-"));
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardArgBackspace { tab_id: 1 },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args().len(), 1);
    }

    #[test]
    fn given_typed_arg_when_saved_then_pending_text_becomes_arg() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_program(String::from("ls"));
            editor.set_title(String::from("list"));
            editor.set_arg_input(String::from("-la"));
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardSave { tab_id: 1 },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args(), &[String::from("-la")]);
    }

    #[test]
    fn given_wizard_add_env_when_reduced_then_env_count_increases() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardAddEnv { tab_id: 1 },
            &ctx(),
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
    fn given_editor_edit_save_request_when_applied_then_command_is_updated() {
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
            command: QuickLaunch::new(
                String::from("Updated"),
                CommandSpec::Custom {
                    custom: CustomCommand::new(
                        String::from("zsh"),
                        Vec::new(),
                        Vec::new(),
                        None,
                    ),
                },
                ExitBehavior::Close,
            ),
        };

        let _outcomes = apply_editor_save_request(&mut state, request);

        assert_eq!(state.data().root.children[0].title(), "Updated");
        assert!(state.is_dirty());
    }

    // -----------------------------------------------------------------------
    // Tab closed cleans up the wizard state
    // -----------------------------------------------------------------------

    #[test]
    fn given_tab_closed_when_reduced_then_its_wizard_is_cleaned() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(5, vec![]);

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::TabClosed {
                tab_id: 5,
                new_active_id: Some(2),
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert!(state.wizard().editor(5).is_none());
        assert_eq!(state.wizard().front_tab_id(), Some(2));
    }

    /// The main area shows the editor of the tab in front: a tab the user
    /// activates comes to the front, and the last tab closing leaves none.
    #[test]
    fn the_tab_in_front_follows_the_tabs() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(3, vec![]);

        let activated = reduce(
            &mut state,
            QuickLaunchIntent::TabActivated { tab_id: 3 },
            &ctx(),
        );
        let in_front = state.wizard().front_tab_id();
        let _closed = reduce(
            &mut state,
            QuickLaunchIntent::TabClosed {
                tab_id: 3,
                new_active_id: None,
            },
            &ctx(),
        );

        assert!(activated.is_empty());
        assert_eq!(in_front, Some(3));
        assert_eq!(state.wizard().front_tab_id(), None);
    }

    // -----------------------------------------------------------------------
    // Background released finishes drag
    // -----------------------------------------------------------------------

    #[test]
    fn given_no_drag_when_background_released_then_pressed_is_cleared() {
        let mut state = QuickLaunchState::default();
        state.set_pressed_path(Some(vec![String::from("x")]));

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::BackgroundReleased, &ctx());

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

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardSave { tab_id: 1 },
            &ctx(),
        );

        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::CloseTab { tab_id: 1 }]
        ));
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
            editor.set_command_type(super::super::types::QuickLaunchType::Ssh);
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateHost {
                tab_id: 1,
                value: String::from("example.com"),
            },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateUser {
                tab_id: 1,
                value: String::from("admin"),
            },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdatePort {
                tab_id: 1,
                value: String::from("2222"),
            },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateIdentityFile {
                tab_id: 1,
                value: String::from("~/.ssh/id_rsa"),
            },
            &ctx(),
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

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateWorkingDirectory {
                tab_id: 1,
                value: String::from("/tmp"),
            },
            &ctx(),
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
            editor.set_arg_input(String::from("-a"));
            editor.submit_arg();
            editor.set_arg_input(String::from("-b"));
            editor.submit_arg();
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardRemoveArg {
                tab_id: 1,
                index: 0,
            },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.args().len(), 1);
    }

    #[test]
    fn given_wizard_remove_env_when_reduced_then_env_is_removed() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.add_env();
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardRemoveEnv {
                tab_id: 1,
                index: 0,
            },
            &ctx(),
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

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateEnvKey {
                tab_id: 1,
                index: 0,
                value: String::from("PATH"),
            },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardUpdateEnvValue {
                tab_id: 1,
                index: 0,
                value: String::from("/usr/bin"),
            },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.env()[0].0, "PATH");
        assert_eq!(custom.env()[0].1, "/usr/bin");
    }

    #[test]
    fn given_wizard_ssh_extra_args_when_submitted_and_removed_then_list_updates()
     {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(1, vec![]);
        if let Some(editor) = state.wizard_mut().editor_mut(1) {
            editor.set_command_type(super::super::types::QuickLaunchType::Ssh);
        }

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardExtraArgInput {
                tab_id: 1,
                value: String::from("-o StrictHostKeyChecking=no"),
            },
            &ctx(),
        );
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardExtraArgSubmit { tab_id: 1 },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.extra_args().len(), 1);
        assert_eq!(ssh.extra_args()[0], "-o StrictHostKeyChecking=no");

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::WizardRemoveExtraArg {
                tab_id: 1,
                index: 0,
            },
            &ctx(),
        );

        let editor = state.wizard().editor(1).expect("editor should exist");
        let ssh = editor.ssh().expect("ssh options should exist");
        assert!(ssh.extra_args().is_empty());
    }

    fn password_request(is_retry: bool) -> SshSecretRequest {
        SshSecretRequest::new(
            SshSecretKind::Password,
            String::from("alice@example.com:22"),
            is_retry,
        )
    }

    fn state_with_secret_prompt() -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        let path = vec![String::from("Demo")];
        let launch_id =
            state.begin_launch(path.clone(), Arc::new(AtomicBool::new(false)));

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::SecretRequired {
                    path,
                    launch_id,
                    command: Box::new(sample_command()),
                    request: password_request(false),
                },
            ),
            &ctx(),
        );

        state
    }

    #[test]
    fn given_secret_required_outcome_when_reduced_then_prompt_opens_and_launch_ends()
     {
        let state = state_with_secret_prompt();

        let prompt = state.secret_prompt().expect("prompt should be open");
        assert_eq!(prompt.request(), &password_request(false));
        assert!(prompt.value().is_empty());
        assert!(state.launching().is_empty());
    }

    #[test]
    fn given_canceled_launch_when_secret_required_then_prompt_stays_closed() {
        let mut state = QuickLaunchState::default();
        let path = vec![String::from("Demo")];
        let launch_id =
            state.begin_launch(path.clone(), Arc::new(AtomicBool::new(false)));
        state.cancel_launch(&path);

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::SecretRequired {
                    path,
                    launch_id,
                    command: Box::new(sample_command()),
                    request: password_request(false),
                },
            ),
            &ctx(),
        );

        assert!(state.secret_prompt().is_none());
        assert!(state.launching().is_empty());
    }

    #[test]
    fn given_open_prompt_when_secret_typed_then_value_is_updated() {
        let mut state = state_with_secret_prompt();

        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SecretPromptChanged(String::from("hunter2")),
            &ctx(),
        );

        let prompt = state.secret_prompt().expect("prompt should be open");
        assert_eq!(prompt.value(), "hunter2");
    }

    #[test]
    fn given_typed_secret_when_submitted_then_prompt_closes_and_launch_restarts()
     {
        let mut state = state_with_secret_prompt();
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SecretPromptChanged(String::from("hunter2")),
            &ctx(),
        );

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::SecretPromptSubmit, &ctx());

        assert!(state.secret_prompt().is_none());
        assert!(state.is_launching(&vec![String::from("Demo")]));
    }

    #[test]
    fn given_empty_secret_when_submitted_then_prompt_stays_open() {
        let mut state = state_with_secret_prompt();

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::SecretPromptSubmit, &ctx());

        assert!(state.secret_prompt().is_some());
        assert!(state.launching().is_empty());
    }

    #[test]
    fn given_open_prompt_when_canceled_then_prompt_closes_without_launch() {
        let mut state = state_with_secret_prompt();

        let _outcomes =
            reduce(&mut state, QuickLaunchIntent::SecretPromptCancel, &ctx());

        assert!(state.secret_prompt().is_none());
        assert!(state.launching().is_empty());
    }

    // -----------------------------------------------------------------------
    // Overlay closing tests
    // -----------------------------------------------------------------------

    /// Reduce `intent` and count how many times it reported the last open
    /// overlay closed.
    fn overlays_closed(
        state: &mut QuickLaunchState,
        intent: QuickLaunchIntent,
    ) -> usize {
        reduce(state, intent, &ctx())
            .iter()
            .filter(|outcome| {
                matches!(outcome, QuickLaunchOutcome::OverlayClosed)
            })
            .count()
    }

    /// The launch `Demo` in the tree with its context menu open.
    fn state_with_command_menu() -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        state
            .data_mut()
            .root
            .children
            .push(QuickLaunchNode::Command(sample_command()));
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(ContextMenuTarget::Command(vec![String::from(
                    "Demo",
                )]))
                .with_cursor(Point::ORIGIN),
        ));

        state
    }

    /// A new folder being named `name` in the tree.
    fn state_with_inline_edit(name: &str) -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        state.set_inline_edit(Some(InlineEditState {
            kind: InlineEditKind::CreateFolder {
                parent_path: Vec::new(),
            },
            value: String::from(name),
            error: None,
            id: iced::widget::Id::unique(),
        }));

        state
    }

    #[test]
    fn closing_the_context_menu_reports_the_overlay_closed() {
        let mut state = state_with_command_menu();

        let closed =
            overlays_closed(&mut state, QuickLaunchIntent::ContextMenuDismiss);

        assert!(state.context_menu().is_none());
        assert_eq!(closed, 1);
    }

    #[test]
    fn closing_a_closed_context_menu_reports_nothing() {
        let mut state = QuickLaunchState::default();

        let closed =
            overlays_closed(&mut state, QuickLaunchIntent::ContextMenuDismiss);

        assert_eq!(closed, 0);
    }

    /// Rename closes the menu and opens the inline edit in its place, so an
    /// overlay stays open.
    #[test]
    fn a_rename_from_the_menu_reports_nothing() {
        let mut state = state_with_command_menu();

        let closed = overlays_closed(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Rename),
        );

        assert!(state.context_menu().is_none());
        assert!(state.inline_edit().is_some());
        assert_eq!(closed, 0);
    }

    #[test]
    fn remove_from_the_menu_reports_the_overlay_closed() {
        let mut state = state_with_command_menu();

        let closed = overlays_closed(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Remove),
        );

        assert!(state.data().root.children.is_empty());
        assert_eq!(closed, 1);
    }

    #[test]
    fn cancelling_or_sending_the_secret_prompt_reports_it() {
        let mut cancelled = state_with_secret_prompt();
        let mut sent = state_with_secret_prompt();
        let typed = overlays_closed(
            &mut sent,
            QuickLaunchIntent::SecretPromptChanged(String::from("hunter2")),
        );

        let on_cancel = overlays_closed(
            &mut cancelled,
            QuickLaunchIntent::SecretPromptCancel,
        );
        let on_send =
            overlays_closed(&mut sent, QuickLaunchIntent::SecretPromptSubmit);

        assert_eq!(typed, 0);
        assert_eq!(on_cancel, 1);
        assert_eq!(on_send, 1);
        assert!(sent.secret_prompt().is_none());
    }

    /// Owner, Q4: after an inline edit's Enter the active terminal gets the
    /// keys, as after a cancel.
    #[test]
    fn cancelling_or_sending_an_inline_edit_reports_it() {
        let mut cancelled = state_with_inline_edit("MyFolder");
        let mut sent = state_with_inline_edit("MyFolder");

        let on_cancel = overlays_closed(
            &mut cancelled,
            QuickLaunchIntent::CancelInlineEdit,
        );
        let on_send =
            overlays_closed(&mut sent, QuickLaunchIntent::InlineEditSubmit);

        assert_eq!(on_cancel, 1);
        assert_eq!(on_send, 1);
        assert_eq!(sent.data().root.children.len(), 1);
    }

    #[test]
    fn a_refused_inline_edit_reports_nothing() {
        let mut state = state_with_inline_edit("   ");

        let closed =
            overlays_closed(&mut state, QuickLaunchIntent::InlineEditSubmit);

        assert!(state.inline_edit().is_some());
        assert_eq!(closed, 0);
    }

    #[test]
    fn given_secret_prompt_when_debug_printed_then_secret_is_hidden() {
        let mut state = state_with_secret_prompt();
        let _outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SecretPromptChanged(String::from("hunter2")),
            &ctx(),
        );

        let debug = format!("{:?}", state.secret_prompt());

        assert!(!debug.contains("hunter2"));
    }

    // -----------------------------------------------------------------------
    // Drag and drop tests
    // -----------------------------------------------------------------------

    fn path(parts: &[&str]) -> NodePath {
        parts.iter().map(|part| part.to_string()).collect()
    }

    fn folder_node(
        title: &str,
        children: Vec<QuickLaunchNode>,
    ) -> QuickLaunchNode {
        QuickLaunchNode::Folder(QuickLaunchFolder {
            title: String::from(title),
            expanded: false,
            children,
        })
    }

    /// The sample command under another title.
    fn titled_command(title: &str) -> QuickLaunch {
        let mut command = sample_command();
        command.rename(String::from(title));

        command
    }

    fn command_node(title: &str) -> QuickLaunchNode {
        QuickLaunchNode::Command(titled_command(title))
    }

    fn state_with_root(children: Vec<QuickLaunchNode>) -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        state.data_mut().root.children = children;
        state
    }

    fn run(state: &mut QuickLaunchState, intent: QuickLaunchIntent) {
        let _outcomes = reduce(state, intent, &ctx());
    }

    /// Press `source`, move the pointer `distance` pixels and hover `over`.
    fn drag_over(
        state: &mut QuickLaunchState,
        source: &NodePath,
        distance: f32,
        over: &NodePath,
    ) {
        run(
            state,
            QuickLaunchIntent::NodePressed {
                path: source.clone(),
            },
        );
        run(
            state,
            QuickLaunchIntent::CursorMoved {
                position: Point::new(0.0, distance),
            },
        );
        run(state, QuickLaunchIntent::NodeHovered { path: over.clone() });
    }

    fn child_titles(
        state: &QuickLaunchState,
        folder: &[String],
    ) -> Vec<String> {
        state
            .data()
            .folder(folder)
            .expect("folder should exist")
            .children()
            .iter()
            .map(|node| node.title().to_string())
            .collect()
    }

    #[test]
    fn given_command_dragged_onto_folder_when_released_then_command_moves_into_it()
     {
        let mut state = state_with_root(vec![
            folder_node("Servers", Vec::new()),
            command_node("Demo"),
        ]);
        let source = path(&["Demo"]);
        let target = path(&["Servers"]);

        drag_over(&mut state, &source, 10.0, &target);

        assert!(matches!(
            state.drop_target(),
            Some(DropTarget::Folder(folder)) if folder == &target
        ));

        run(
            &mut state,
            QuickLaunchIntent::NodeReleased {
                path: target.clone(),
            },
        );

        assert_eq!(child_titles(&state, &[]), vec![String::from("Servers")]);
        assert_eq!(child_titles(&state, &target), vec![String::from("Demo")]);
        assert_eq!(state.selected_path(), Some(&path(&["Servers", "Demo"])));
        assert!(state.is_dirty());
        assert!(state.drag().is_none());
    }

    #[test]
    fn given_pointer_moved_less_than_threshold_when_released_then_it_is_a_click()
     {
        let mut state = state_with_root(vec![
            folder_node("Servers", Vec::new()),
            folder_node("Local", Vec::new()),
        ]);
        let source = path(&["Servers"]);

        drag_over(&mut state, &source, 2.0, &path(&["Local"]));
        run(
            &mut state,
            QuickLaunchIntent::NodeReleased {
                path: source.clone(),
            },
        );

        assert!(state.drop_target().is_none());
        assert_eq!(
            child_titles(&state, &[]),
            vec![String::from("Servers"), String::from("Local")]
        );
        let folder = state.data().folder(&source).expect("folder should exist");
        assert!(folder.is_expanded());
    }

    #[test]
    fn given_folder_dragged_into_its_own_child_when_hovered_then_drop_is_refused()
     {
        let mut state = state_with_root(vec![folder_node(
            "Servers",
            vec![folder_node("Prod", Vec::new())],
        )]);
        let source = path(&["Servers"]);
        let child = path(&["Servers", "Prod"]);

        drag_over(&mut state, &source, 10.0, &child);
        run(&mut state, QuickLaunchIntent::NodeReleased { path: child });

        assert!(state.drop_target().is_none());
        assert_eq!(child_titles(&state, &source), vec![String::from("Prod")]);
    }

    #[test]
    fn given_hovered_command_in_folder_when_dragging_then_its_folder_is_the_target()
     {
        let mut state = state_with_root(vec![
            folder_node("Servers", vec![command_node("Prod")]),
            command_node("Demo"),
        ]);

        drag_over(
            &mut state,
            &path(&["Demo"]),
            10.0,
            &path(&["Servers", "Prod"]),
        );

        assert!(matches!(
            state.drop_target(),
            Some(DropTarget::Folder(folder)) if folder == &path(&["Servers"])
        ));
    }

    #[test]
    fn given_command_dragged_to_background_when_released_then_it_moves_to_root()
    {
        let mut state = state_with_root(vec![folder_node(
            "Servers",
            vec![command_node("Prod")],
        )]);
        let source = path(&["Servers", "Prod"]);

        drag_over(&mut state, &source, 10.0, &source);
        run(&mut state, QuickLaunchIntent::BackgroundReleased);

        assert_eq!(
            child_titles(&state, &[]),
            vec![String::from("Servers"), String::from("Prod")]
        );
        assert!(child_titles(&state, &path(&["Servers"])).is_empty());
    }

    #[test]
    fn given_target_with_same_title_when_dropped_then_move_is_refused() {
        let mut state = state_with_root(vec![
            folder_node("Servers", vec![command_node("Demo")]),
            command_node("Demo"),
        ]);
        let target = path(&["Servers"]);

        drag_over(&mut state, &path(&["Demo"]), 10.0, &target);
        run(
            &mut state,
            QuickLaunchIntent::NodeReleased {
                path: target.clone(),
            },
        );

        assert_eq!(
            child_titles(&state, &[]),
            vec![String::from("Servers"), String::from("Demo")]
        );
        assert_eq!(child_titles(&state, &target), vec![String::from("Demo")]);
    }

    #[test]
    fn given_running_command_when_moved_then_its_launch_follows_the_new_path() {
        let mut state = state_with_root(vec![
            folder_node("Servers", Vec::new()),
            command_node("Demo"),
        ]);
        let source = path(&["Demo"]);
        let target = path(&["Servers"]);
        let _launch_id = state
            .begin_launch(source.clone(), Arc::new(AtomicBool::new(false)));

        drag_over(&mut state, &source, 10.0, &target);
        run(&mut state, QuickLaunchIntent::NodeReleased { path: target });

        assert!(!state.is_launching(&source));
        assert!(state.is_launching(&path(&["Servers", "Demo"])));
    }

    // -----------------------------------------------------------------------
    // Context menu action tests
    // -----------------------------------------------------------------------

    fn run_menu_action(
        state: &mut QuickLaunchState,
        target: ContextMenuTarget,
        action: ContextMenuAction,
    ) {
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(target)
                .with_cursor(Point::ORIGIN),
        ));
        run(state, QuickLaunchIntent::ContextMenuAction(action));
    }

    #[test]
    fn given_command_when_duplicated_twice_then_copies_get_unique_titles() {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let target = ContextMenuTarget::Command(path(&["Demo"]));

        run_menu_action(
            &mut state,
            target.clone(),
            ContextMenuAction::Duplicate,
        );
        run_menu_action(&mut state, target, ContextMenuAction::Duplicate);

        assert_eq!(
            child_titles(&state, &[]),
            vec![
                String::from("Demo"),
                String::from("Demo copy"),
                String::from("Demo copy (0)"),
            ]
        );
        assert!(state.context_menu().is_none());
        assert!(state.is_dirty());
    }

    #[test]
    fn given_command_and_folder_menus_when_removed_then_matching_nodes_go_away()
    {
        let mut state = state_with_root(vec![
            folder_node("Servers", Vec::new()),
            command_node("Demo"),
        ]);

        run_menu_action(
            &mut state,
            ContextMenuTarget::Folder(path(&["Servers"])),
            ContextMenuAction::Remove,
        );
        run_menu_action(
            &mut state,
            ContextMenuTarget::Command(path(&["Demo"])),
            ContextMenuAction::Delete,
        );

        assert_eq!(
            child_titles(&state, &[]),
            vec![String::from("Servers"), String::from("Demo")]
        );

        run_menu_action(
            &mut state,
            ContextMenuTarget::Command(path(&["Demo"])),
            ContextMenuAction::Remove,
        );
        run_menu_action(
            &mut state,
            ContextMenuTarget::Folder(path(&["Servers"])),
            ContextMenuAction::Delete,
        );

        assert!(state.data().root.children.is_empty());
    }

    #[test]
    fn given_command_menu_when_rename_chosen_then_inline_edit_holds_its_title()
    {
        let mut state = state_with_root(vec![command_node("Demo")]);

        run_menu_action(
            &mut state,
            ContextMenuTarget::Command(path(&["Demo"])),
            ContextMenuAction::Rename,
        );

        let edit = state.inline_edit().expect("rename should start");
        assert_eq!(edit.value, "Demo");
        assert!(matches!(
            &edit.kind,
            InlineEditKind::Rename { path: edit_path } if edit_path == &path(&["Demo"])
        ));
    }

    #[test]
    fn given_command_in_closed_folder_when_create_folder_chosen_then_folder_opens()
     {
        let mut state = state_with_root(vec![folder_node(
            "Servers",
            vec![command_node("Prod")],
        )]);

        run_menu_action(
            &mut state,
            ContextMenuTarget::Command(path(&["Servers", "Prod"])),
            ContextMenuAction::CreateFolder,
        );

        let edit = state.inline_edit().expect("create folder should start");
        assert!(matches!(
            &edit.kind,
            InlineEditKind::CreateFolder { parent_path } if parent_path == &path(&["Servers"])
        ));
        let folder = state
            .data()
            .folder(&path(&["Servers"]))
            .expect("folder should exist");
        assert!(folder.is_expanded());
    }

    #[test]
    fn given_running_command_when_kill_chosen_then_its_launch_is_canceled() {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let cancel = Arc::new(AtomicBool::new(false));
        let _launch_id = state.begin_launch(path(&["Demo"]), cancel.clone());

        run_menu_action(
            &mut state,
            ContextMenuTarget::Command(path(&["Demo"])),
            ContextMenuAction::Kill,
        );

        assert!(cancel.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn given_no_open_menu_when_action_arrives_then_tree_is_unchanged() {
        let mut state = state_with_root(vec![command_node("Demo")]);

        run(
            &mut state,
            QuickLaunchIntent::ContextMenuAction(ContextMenuAction::Duplicate),
        );

        assert_eq!(child_titles(&state, &[]), vec![String::from("Demo")]);
    }

    // -----------------------------------------------------------------------
    // Launch flow tests
    // -----------------------------------------------------------------------

    fn ssh_command_node(title: &str) -> QuickLaunchNode {
        QuickLaunchNode::Command(QuickLaunch::new(
            String::from(title),
            CommandSpec::Ssh {
                ssh: SshCommand::new(
                    String::from("example.com"),
                    22,
                    Some(String::from("alice")),
                    Some(String::from("~/.ssh/id_ed25519")),
                    Vec::new(),
                ),
            },
            ExitBehavior::Close,
        ))
    }

    /// Click a node the way the tree does: press, then release on it.
    ///
    /// The release starts the launch setup task; it is dropped here so no
    /// process or SSH connection is started.
    fn click(state: &mut QuickLaunchState, node: &NodePath) {
        run(state, QuickLaunchIntent::NodePressed { path: node.clone() });
        run(
            state,
            QuickLaunchIntent::NodeReleased { path: node.clone() },
        );
    }

    fn launch_id(state: &QuickLaunchState, node: &NodePath) -> u64 {
        state
            .launch_info(node)
            .expect("clicked command should be launching")
            .id
    }

    #[test]
    fn a_dirty_tick_asks_the_route_to_save_the_launches() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();

        let outcomes = reduce(&mut state, QuickLaunchIntent::Tick, &ctx());

        assert!(state.is_persist_in_flight());
        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::SaveLaunches(_)]
        ));
    }

    #[test]
    fn a_tick_during_a_save_asks_for_nothing() {
        let mut state = QuickLaunchState::default();
        state.mark_dirty();
        state.begin_persist();

        let outcomes = reduce(&mut state, QuickLaunchIntent::Tick, &ctx());

        assert!(outcomes.is_empty());
    }

    #[test]
    fn launching_a_command_asks_the_route_to_prepare_it() {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::LaunchNode { path: node.clone() },
            &ctx(),
        );

        let current_launch = launch_id(&state, &node);
        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::PrepareLaunch(request)]
                if request.path() == node.as_slice()
                    && request.launch_id() == current_launch
                    && request.secret().is_none()
                    && request.command().title() == "Demo"
        ));
    }

    #[test]
    fn a_submitted_secret_goes_to_the_route_with_the_new_launch() {
        let mut state = state_with_secret_prompt();
        run(
            &mut state,
            QuickLaunchIntent::SecretPromptChanged(String::from("hunter2")),
        );

        let outcomes =
            reduce(&mut state, QuickLaunchIntent::SecretPromptSubmit, &ctx());

        assert!(matches!(
            outcomes.as_slice(),
            [
                QuickLaunchOutcome::PrepareLaunch(request),
                QuickLaunchOutcome::OverlayClosed,
            ] if request.secret() == Some("hunter2")
        ));
    }

    #[test]
    fn a_click_on_a_launching_command_asks_for_nothing() {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);
        click(&mut state, &node);

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::LaunchNode { path: node },
            &ctx(),
        );

        assert!(outcomes.is_empty());
    }

    #[test]
    fn given_launch_node_intent_when_reduced_then_command_starts_like_a_click()
    {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);

        run(
            &mut state,
            QuickLaunchIntent::LaunchNode { path: node.clone() },
        );

        assert!(state.is_launching(&node));
        assert_eq!(state.selected_path(), Some(&node));
    }

    #[test]
    fn given_command_clicked_when_setup_is_prepared_then_terminal_tab_is_requested()
     {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);

        click(&mut state, &node);

        assert!(state.is_launching(&node));
        assert_eq!(state.selected_path(), Some(&node));

        let current_launch = launch_id(&state, &node);

        let outcomes = prepare(&mut state, &node, current_launch, "Demo");

        assert!(state.launching().is_empty());
        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::LaunchReady {
                title,
                exit_behavior: ExitBehavior::Close,
                machine: TabMachine::ThisComputer,
                ..
            }] if title == "Demo"
        ));
    }

    /// Answer the check of the launch `title` at `node` for the run
    /// `launch_id`: it is ready.
    fn prepare(
        state: &mut QuickLaunchState,
        node: &NodePath,
        launch_id: u64,
        title: &str,
    ) -> Outcomes {
        let command = state
            .data()
            .node(node)
            .and_then(|found| match found {
                QuickLaunchNode::Command(command) => Some(command.clone()),
                QuickLaunchNode::Folder(_) => None,
            })
            .unwrap_or_else(|| titled_command(title));

        reduce(
            state,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::Prepared {
                    path: node.clone(),
                    launch_id,
                    command: Box::new(command),
                    settings: Box::new(Settings::default()),
                },
            ),
            &ctx(),
        )
    }

    /// Which machine a launch runs on is decided here: an SSH launch runs
    /// on the host it connects to, with what its tab does when it ends.
    #[test]
    fn a_ready_ssh_launch_opens_its_tab_on_the_ssh_host() {
        let mut state = state_with_root(vec![ssh_command_node("Prod")]);
        let node = path(&["Prod"]);
        click(&mut state, &node);

        let current_launch = launch_id(&state, &node);

        let outcomes = prepare(&mut state, &node, current_launch, "Prod");

        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::LaunchReady { title, machine, .. }]
                if title == "Prod"
                    && *machine == TabMachine::Ssh(String::from("example.com"))
        ));
    }

    /// A launch killed while it was checked opens no tab, even when the
    /// check finished first.
    #[test]
    fn a_killed_launch_opens_no_tab() {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);
        click(&mut state, &node);
        let killed = launch_id(&state, &node);
        state.cancel_launch(&node);

        let outcomes = prepare(&mut state, &node, killed, "Demo");

        assert!(outcomes.is_empty());
        assert!(state.launching().is_empty());
    }

    #[test]
    fn given_launching_command_when_clicked_again_then_no_second_launch_starts()
    {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);
        click(&mut state, &node);
        let first_launch = launch_id(&state, &node);

        click(&mut state, &node);

        assert_eq!(launch_id(&state, &node), first_launch);
        assert_eq!(state.launching().len(), 1);
    }

    /// A launch that fails before its tab opens tells the user why in a
    /// notification, which names the launch. No tab opens, so the mode stays
    /// as it was.
    #[test]
    fn given_command_clicked_when_setup_fails_then_the_user_is_told_why() {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);
        click(&mut state, &node);
        let current_launch = launch_id(&state, &node);
        let expected = i18n::launch_failed("Demo", "no such program");

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::Failed {
                    path: node.clone(),
                    launch_id: current_launch,
                    command: Box::new(sample_command()),
                    error: String::from("no such program"),
                },
            ),
            &ctx(),
        );

        assert!(state.launching().is_empty());
        assert!(matches!(
            outcomes.as_slice(),
            [QuickLaunchOutcome::Failed(text)] if *text == expected
        ));
    }

    /// The result of a launch the user ran again meanwhile is stale: it
    /// tells nothing, as the newer run reports for itself.
    #[test]
    fn given_a_stale_launch_when_its_setup_fails_then_nothing_is_told() {
        let mut state = state_with_root(vec![command_node("Demo")]);
        let node = path(&["Demo"]);
        click(&mut state, &node);
        let stale_launch = launch_id(&state, &node) + 1;

        let outcomes = reduce(
            &mut state,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::Failed {
                    path: node.clone(),
                    launch_id: stale_launch,
                    command: Box::new(sample_command()),
                    error: String::from("no such program"),
                },
            ),
            &ctx(),
        );

        assert!(outcomes.is_empty());
    }

    #[test]
    fn given_ssh_command_clicked_when_passphrase_is_needed_then_prompt_opens_and_cancel_closes_it()
     {
        let mut state = state_with_root(vec![ssh_command_node("Prod")]);
        let node = path(&["Prod"]);
        let request = SshSecretRequest::new(
            SshSecretKind::KeyPassphrase,
            String::from("~/.ssh/id_ed25519"),
            false,
        );

        click(&mut state, &node);

        assert!(state.is_launching(&node));

        let current_launch = launch_id(&state, &node);
        let opened = reduce(
            &mut state,
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::SecretRequired {
                    path: node.clone(),
                    launch_id: current_launch,
                    command: Box::new(titled_command("Prod")),
                    request: request.clone(),
                },
            ),
            &ctx(),
        );

        let prompt = state.secret_prompt().expect("prompt should be open");
        assert!(matches!(
            opened.as_slice(),
            [QuickLaunchOutcome::FocusInput(id)] if id == prompt.input_id()
        ));
        assert_eq!(prompt.request(), &request);
        assert!(state.launching().is_empty());

        let cancelled =
            reduce(&mut state, QuickLaunchIntent::SecretPromptCancel, &ctx());

        assert!(matches!(
            cancelled.as_slice(),
            [QuickLaunchOutcome::OverlayClosed]
        ));
        assert!(state.secret_prompt().is_none());
        assert!(state.launching().is_empty());
        assert!(!state.is_launching(&node));
    }
}
