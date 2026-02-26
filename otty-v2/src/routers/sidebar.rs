use iced::Task;

use crate::app::{App, Event as AppEvent};
use crate::widgets;

/// Route sidebar UI event into widget reducer command path.
pub(crate) fn route_event(
    app: &mut App,
    event: widgets::sidebar::SidebarUiEvent,
) -> Task<AppEvent> {
    let command = map_sidebar_ui_event_to_command(event);
    app.widgets
        .sidebar_mut()
        .reduce(command, &widgets::sidebar::SidebarCtx)
        .map(AppEvent::SidebarEffect)
}

/// Route sidebar effect event into app-level action task.
pub(crate) fn route_effect(
    event: widgets::sidebar::SidebarEffectEvent,
) -> Task<AppEvent> {
    map_sidebar_effect_event_to_app_task(event)
}

fn map_sidebar_ui_event_to_command(
    event: widgets::sidebar::SidebarUiEvent,
) -> widgets::sidebar::SidebarCommand {
    use widgets::sidebar::{SidebarCommand as C, SidebarUiEvent as E};

    match event {
        E::Menu(event) => C::Menu(event),
        E::Workspace(event) => C::Workspace(event),
        E::ToggleVisibility => C::ToggleVisibility,
        E::PaneGridCursorMoved { position } => {
            C::PaneGridCursorMoved { position }
        },
        E::DismissAddMenu => C::DismissAddMenu,
    }
}

fn map_sidebar_effect_event_to_app_task(
    event: widgets::sidebar::SidebarEffectEvent,
) -> Task<AppEvent> {
    use widgets::sidebar::SidebarEffectEvent as E;

    match event {
        E::SyncTerminalGridSizes => Task::done(AppEvent::SyncTerminalGridSizes),
        E::OpenSettingsTab => Task::done(AppEvent::OpenSettingsTab),
        E::OpenTerminalTab => Task::done(AppEvent::OpenTerminalTab),
        E::QuickLaunch(event) => Task::done(AppEvent::QuickLaunch(event)),
        E::Explorer(event) => Task::done(AppEvent::ExplorerUi(event)),
    }
}
