use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::sidebar::{
    SidebarCtx, SidebarEffect, SidebarEvent, SidebarUiEvent,
};

/// Route a sidebar event through widget reduction or app orchestration.
pub(crate) fn route(app: &mut App, event: SidebarEvent) -> Task<AppEvent> {
    match event {
        SidebarEvent::Ui(event) => route_ui_event(app, event),
        SidebarEvent::Effect(effect) => route_effect_event(effect),
    }
}

fn route_ui_event(app: &mut App, event: SidebarUiEvent) -> Task<AppEvent> {
    app.widgets
        .sidebar
        .reduce(event, &SidebarCtx)
        .map(AppEvent::Sidebar)
}

fn route_effect_event(event: SidebarEffect) -> Task<AppEvent> {
    use SidebarEffect as E;

    match event {
        E::SyncTerminalGridSizes => {
            Task::done(AppEvent::SyncTerminalGridSizes)
        },
        E::OpenSettingsTab => Task::done(AppEvent::OpenSettingsTab),
        E::OpenTerminalTab => Task::done(AppEvent::OpenTerminalTab),
        E::QuickLaunchHeaderCreateCommand => {
            Task::done(AppEvent::QuickLaunch(
                crate::widgets::quick_launch::QuickLaunchEvent::Ui(
                    crate::widgets::quick_launch::QuickLaunchUiEvent::HeaderCreateCommand,
                ),
            ))
        },
        E::QuickLaunchHeaderCreateFolder => {
            Task::done(AppEvent::QuickLaunch(
                crate::widgets::quick_launch::QuickLaunchEvent::Ui(
                    crate::widgets::quick_launch::QuickLaunchUiEvent::HeaderCreateFolder,
                ),
            ))
        },
        E::QuickLaunchResetInteractionState => {
            Task::done(AppEvent::QuickLaunch(
                crate::widgets::quick_launch::QuickLaunchEvent::Ui(
                    crate::widgets::quick_launch::QuickLaunchUiEvent::ResetInteractionState,
                ),
            ))
        },
    }
}
