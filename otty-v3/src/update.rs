use iced::{Task, window};

use super::{App, AppEvent};
use crate::guards::{MenuGuard, context_menu_guard, inline_edit_guard};
use crate::routers;
use crate::widgets::quick_launch::QuickLaunchCommand;
use crate::widgets::sidebar::SidebarEvent;
use crate::widgets::terminal_workspace::TerminalWorkspaceCommand;

/// Thin dispatch: route each event to its owning router or handler.
pub(super) fn update(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let mut pre_dispatch_tasks = Vec::new();

    if app.widgets.quick_launch.has_inline_edit() && inline_edit_guard(&event) {
        pre_dispatch_tasks.push(routers::quick_launch::route_command(
            app,
            QuickLaunchCommand::CancelInlineEdit,
        ));
    }

    if any_context_menu_open(app) {
        match context_menu_guard(&event) {
            MenuGuard::Allow => {},
            MenuGuard::Ignore => return Task::none(),
            MenuGuard::Dismiss => return close_all_context_menus(app),
        }
    }

    let dispatch_task = route(app, event);
    if pre_dispatch_tasks.is_empty() {
        dispatch_task
    } else {
        pre_dispatch_tasks.push(dispatch_task);
        Task::batch(pre_dispatch_tasks)
    }
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

/// Return whether any context menu overlay is currently open.
pub(super) fn any_context_menu_open(app: &App) -> bool {
    if app.widgets.sidebar.has_add_menu_open()
        || app.widgets.quick_launch.context_menu().is_some()
    {
        return true;
    }

    app.widgets.terminal_workspace.has_any_context_menu()
}

/// Close all open context menus before dispatching a new event.
fn close_all_context_menus(app: &mut App) -> Task<AppEvent> {
    Task::batch(vec![
        routers::sidebar::route_event(app, SidebarEvent::DismissAddMenu),
        routers::quick_launch::route_command(
            app,
            QuickLaunchCommand::ContextMenuDismiss,
        ),
        routers::terminal_workspace::route_command(
            app,
            TerminalWorkspaceCommand::CloseAllContextMenus,
        ),
    ])
}
