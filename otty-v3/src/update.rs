use iced::{Task, window};

use super::{App, AppEvent};
use crate::routers;

/// Thin dispatch: route each event to its owning router or handler.
pub(super) fn update(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    route(app, event)
}

fn route(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => {
            routers::flow::route(app, crate::app::AppFlowEvent::OpenTerminalTab)
        },
        // Sidebar widget
        AppEvent::SidebarUi(event) => routers::sidebar::route_event(app, event),
        AppEvent::SidebarEffect(event) => routers::sidebar::route_effect(event),
        // Chrome widget
        AppEvent::ChromeUi(event) => routers::chrome::route_event(app, event),
        AppEvent::ChromeEffect(effect) => routers::chrome::route_effect(effect),
        // Tabs widget
        AppEvent::TabsUi(event) => routers::tabs::route_event(app, event),
        AppEvent::TabsEffect(effect) => {
            routers::tabs::route_effect(app, effect)
        },
        // Quick Launch widget
        AppEvent::QuickLaunchUi(event) => {
            routers::quick_launch::route_event(app, event)
        },
        AppEvent::QuickLaunchEffect(effect) => {
            routers::quick_launch::route_effect(effect)
        },
        // Terminal Workspace widget
        AppEvent::TerminalWorkspaceUi(event) => {
            routers::terminal_workspace::route_event(app, event)
        },
        AppEvent::TerminalWorkspaceEffect(effect) => {
            routers::terminal_workspace::route_effect(app, effect)
        },
        // Explorer widget
        AppEvent::ExplorerUi(event) => {
            routers::explorer::route_event(app, event)
        },
        AppEvent::ExplorerEffect(effect) => {
            routers::explorer::route_effect(effect)
        },
        // Settings widget
        AppEvent::SettingsUi(event) => {
            routers::settings::route_event(app, event)
        },
        AppEvent::SettingsEffect(effect) => {
            routers::settings::route_effect(app, effect)
        },
        // Cross-widget flows
        AppEvent::Flow(flow) => routers::flow::route(app, flow),
        // Direct operations
        AppEvent::SyncTerminalGridSizes => {
            routers::window::sync_terminal_grid_sizes(app);
            Task::none()
        },
        AppEvent::Keyboard(_event) => Task::none(),
        AppEvent::Window(window::Event::Resized(size)) => {
            routers::window::handle_resize(app, size)
        },
        AppEvent::Window(_) => Task::none(),
        AppEvent::ResizeWindow(dir) => routers::window::handle_drag_resize(dir),
    }
}
