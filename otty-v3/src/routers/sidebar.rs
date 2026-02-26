use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::sidebar::{
    SidebarCommand, SidebarCtx, SidebarEffect, SidebarEvent,
};

/// Route a sidebar UI event through the widget reducer and map effects.
pub(crate) fn route_event(
    app: &mut App,
    event: SidebarEvent,
) -> Task<AppEvent> {
    let command = map_sidebar_ui_event_to_command(event);
    app.widgets
        .sidebar
        .reduce(command, &SidebarCtx)
        .map(AppEvent::SidebarEffect)
}

/// Route a sidebar effect event to an app-level task.
pub(crate) fn route_effect(event: SidebarEffect) -> Task<AppEvent> {
    map_sidebar_effect_event_to_app_task(event)
}

fn map_sidebar_ui_event_to_command(event: SidebarEvent) -> SidebarCommand {
    use {SidebarCommand as C, SidebarEvent as E};

    match event {
        E::SelectTerminal => C::SelectTerminal,
        E::SelectExplorer => C::SelectExplorer,
        E::ToggleWorkspace => C::ToggleWorkspace,
        E::OpenSettings => C::OpenSettings,
        E::AddMenuOpen => C::AddMenuOpen,
        E::AddMenuDismiss => C::AddMenuDismiss,
        E::AddMenuCreateTab => C::AddMenuCreateTab,
        E::AddMenuCreateCommand => C::AddMenuCreateCommand,
        E::AddMenuCreateFolder => C::AddMenuCreateFolder,
        E::WorkspaceCursorMoved { position } => {
            C::WorkspaceCursorMoved { position }
        },
        E::ToggleVisibility => C::ToggleVisibility,
        E::PaneGridCursorMoved { position } => {
            C::PaneGridCursorMoved { position }
        },
        E::Resized(event) => C::Resized(event),
        E::DismissAddMenu => C::DismissAddMenu,
    }
}

fn map_sidebar_effect_event_to_app_task(
    event: SidebarEffect,
) -> Task<AppEvent> {
    use SidebarEffect as E;

    match event {
        E::SyncTerminalGridSizes => {
            Task::done(AppEvent::SyncTerminalGridSizes)
        },
        E::OpenSettingsTab => Task::done(AppEvent::Flow(
            crate::app::AppFlowEvent::OpenSettingsTab,
        )),
        E::OpenTerminalTab => Task::done(AppEvent::Flow(
            crate::app::AppFlowEvent::OpenTerminalTab,
        )),
        E::QuickLaunchHeaderCreateCommand => {
            Task::done(AppEvent::QuickLaunchUi(
                crate::widgets::quick_launch::QuickLaunchEvent::HeaderCreateCommand,
            ))
        },
        E::QuickLaunchHeaderCreateFolder => {
            Task::done(AppEvent::QuickLaunchUi(
                crate::widgets::quick_launch::QuickLaunchEvent::HeaderCreateFolder,
            ))
        },
        E::QuickLaunchResetInteractionState => {
            Task::done(AppEvent::QuickLaunchUi(
                crate::widgets::quick_launch::QuickLaunchEvent::ResetInteractionState,
            ))
        },
    }
}
