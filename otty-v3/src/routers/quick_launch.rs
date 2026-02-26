use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::quick_launch::{
    QuickLaunchCommand, QuickLaunchCtx, QuickLaunchEffect, QuickLaunchEvent,
};

/// Route a quick launch UI event through the widget reducer and map effects.
pub(crate) fn route_event(
    app: &mut App,
    event: QuickLaunchEvent,
) -> Task<AppEvent> {
    let command = map_quick_launch_ui_event_to_command(event);
    let ctx = build_ctx_from_parts(
        &app.terminal_settings,
        app.widgets.sidebar.cursor(),
        app.widgets.sidebar.is_resizing(),
    );
    app.widgets
        .quick_launch
        .reduce(command, &ctx)
        .map(AppEvent::QuickLaunchEffect)
}

/// Route a quick launch command directly (used by flow routers).
pub(crate) fn route_command(
    app: &mut App,
    command: QuickLaunchCommand,
) -> Task<AppEvent> {
    let ctx = build_ctx_from_parts(
        &app.terminal_settings,
        app.widgets.sidebar.cursor(),
        app.widgets.sidebar.is_resizing(),
    );
    app.widgets
        .quick_launch
        .reduce(command, &ctx)
        .map(AppEvent::QuickLaunchEffect)
}

/// Route a quick launch effect event to app-level tasks.
pub(crate) fn route_effect(effect: QuickLaunchEffect) -> Task<AppEvent> {
    map_quick_launch_effect_to_app_task(effect)
}

fn build_ctx_from_parts<'a>(
    terminal_settings: &'a otty_ui_term::settings::Settings,
    sidebar_cursor: iced::Point,
    sidebar_is_resizing: bool,
) -> QuickLaunchCtx<'a> {
    QuickLaunchCtx {
        terminal_settings,
        sidebar_cursor,
        sidebar_is_resizing,
    }
}

fn map_quick_launch_ui_event_to_command(
    event: QuickLaunchEvent,
) -> QuickLaunchCommand {
    use {QuickLaunchCommand as C, QuickLaunchEvent as E};

    match event {
        E::NodeHovered { path } => C::NodeHovered { path },
        E::NodePressed { path } => C::NodePressed { path },
        E::NodeReleased { path } => C::NodeReleased { path },
        E::NodeRightClicked { path } => C::NodeRightClicked { path },
        E::BackgroundPressed => C::BackgroundPressed,
        E::BackgroundReleased => C::BackgroundReleased,
        E::BackgroundRightClicked => C::BackgroundRightClicked,
        E::CursorMoved { position } => C::CursorMoved { position },
        E::ResetInteractionState => C::ResetInteractionState,
        E::ContextMenuDismiss => C::ContextMenuDismiss,
        E::ContextMenuAction(action) => C::ContextMenuAction(action),
        E::InlineEditChanged(value) => C::InlineEditChanged(value),
        E::InlineEditSubmit => C::InlineEditSubmit,
        E::CancelInlineEdit => C::CancelInlineEdit,
        E::HeaderCreateFolder => C::HeaderCreateFolder,
        E::HeaderCreateCommand => C::HeaderCreateCommand,
        E::DeleteSelected => C::DeleteSelected,
        E::OpenErrorTab {
            tab_id,
            title,
            message,
        } => C::OpenErrorTab {
            tab_id,
            title,
            message,
        },
        E::TabClosed { tab_id } => C::TabClosed { tab_id },
        E::WizardInitializeCreate {
            tab_id,
            parent_path,
        } => C::WizardInitializeCreate {
            tab_id,
            parent_path,
        },
        E::WizardInitializeEdit {
            tab_id,
            path,
            command,
        } => C::WizardInitializeEdit {
            tab_id,
            path,
            command,
        },
        E::WizardCancel { tab_id } => C::WizardCancel { tab_id },
        E::WizardSave { tab_id } => C::WizardSave { tab_id },
        E::WizardSetError { tab_id, message } => {
            C::WizardSetError { tab_id, message }
        },
        E::WizardUpdateTitle { tab_id, value } => {
            C::WizardUpdateTitle { tab_id, value }
        },
        E::WizardUpdateProgram { tab_id, value } => {
            C::WizardUpdateProgram { tab_id, value }
        },
        E::WizardUpdateHost { tab_id, value } => {
            C::WizardUpdateHost { tab_id, value }
        },
        E::WizardUpdateUser { tab_id, value } => {
            C::WizardUpdateUser { tab_id, value }
        },
        E::WizardUpdatePort { tab_id, value } => {
            C::WizardUpdatePort { tab_id, value }
        },
        E::WizardUpdateIdentityFile { tab_id, value } => {
            C::WizardUpdateIdentityFile { tab_id, value }
        },
        E::WizardUpdateWorkingDirectory { tab_id, value } => {
            C::WizardUpdateWorkingDirectory { tab_id, value }
        },
        E::WizardAddArg { tab_id } => C::WizardAddArg { tab_id },
        E::WizardRemoveArg { tab_id, index } => {
            C::WizardRemoveArg { tab_id, index }
        },
        E::WizardUpdateArg {
            tab_id,
            index,
            value,
        } => C::WizardUpdateArg {
            tab_id,
            index,
            value,
        },
        E::WizardAddEnv { tab_id } => C::WizardAddEnv { tab_id },
        E::WizardRemoveEnv { tab_id, index } => {
            C::WizardRemoveEnv { tab_id, index }
        },
        E::WizardUpdateEnvKey {
            tab_id,
            index,
            value,
        } => C::WizardUpdateEnvKey {
            tab_id,
            index,
            value,
        },
        E::WizardUpdateEnvValue {
            tab_id,
            index,
            value,
        } => C::WizardUpdateEnvValue {
            tab_id,
            index,
            value,
        },
        E::WizardAddExtraArg { tab_id } => C::WizardAddExtraArg { tab_id },
        E::WizardRemoveExtraArg { tab_id, index } => {
            C::WizardRemoveExtraArg { tab_id, index }
        },
        E::WizardUpdateExtraArg {
            tab_id,
            index,
            value,
        } => C::WizardUpdateExtraArg {
            tab_id,
            index,
            value,
        },
        E::WizardSelectCommandType {
            tab_id,
            command_type,
        } => C::WizardSelectCommandType {
            tab_id,
            command_type,
        },
        E::SetupCompleted(outcome) => C::SetupCompleted(outcome),
        E::WizardSaveRequested(request) => C::WizardSaveRequested(request),
        E::PersistCompleted => C::PersistCompleted,
        E::PersistFailed(message) => C::PersistFailed(message),
        E::Tick => C::Tick,
    }
}

fn map_quick_launch_effect_to_app_task(
    effect: QuickLaunchEffect,
) -> Task<AppEvent> {
    use crate::app::AppFlowEvent;

    match effect {
        QuickLaunchEffect::OpenWizardCreateTab { parent_path } => {
            Task::done(AppEvent::Flow(
                AppFlowEvent::OpenQuickLaunchWizardCreateTab { parent_path },
            ))
        },
        QuickLaunchEffect::OpenWizardEditTab { path, command } => {
            Task::done(AppEvent::Flow(
                AppFlowEvent::OpenQuickLaunchWizardEditTab { path, command },
            ))
        },
        QuickLaunchEffect::OpenCommandTerminalTab {
            title,
            settings,
            command,
        } => Task::done(AppEvent::Flow(
            AppFlowEvent::OpenQuickLaunchCommandTerminalTab {
                title,
                settings,
                command,
            },
        )),
        QuickLaunchEffect::OpenErrorTab { title, message } => {
            Task::done(AppEvent::Flow(AppFlowEvent::OpenQuickLaunchErrorTab {
                title,
                message,
            }))
        },
        QuickLaunchEffect::CloseTabRequested { tab_id } => {
            if tab_id == 0 {
                // Dummy effect from canceled operations, ignore
                Task::none()
            } else {
                Task::done(AppEvent::Flow(AppFlowEvent::CloseTab { tab_id }))
            }
        },
        QuickLaunchEffect::WizardSetError { tab_id, message } => {
            Task::done(AppEvent::QuickLaunchUi(
                QuickLaunchEvent::WizardSetError { tab_id, message },
            ))
        },
    }
}
