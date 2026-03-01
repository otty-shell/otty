use iced::Task;

use super::{App, AppEvent};
use crate::guards::{MenuGuard, context_menu_guard, inline_edit_guard};
use crate::routers;
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchUiEvent};
use crate::widgets::sidebar::SidebarEvent;
use crate::widgets::terminal_workspace::TerminalWorkspaceCommand;

/// Thin dispatch: route each event to its owning router or handler.
pub(super) fn update(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let mut pre_dispatch_tasks = Vec::new();

    if app.widgets.quick_launch.has_inline_edit() && inline_edit_guard(&event) {
        pre_dispatch_tasks.push(routers::route(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Ui(
                QuickLaunchUiEvent::CancelInlineEdit,
            )),
        ));
    }

    if any_context_menu_open(app) {
        match context_menu_guard(&event) {
            MenuGuard::Allow => {},
            MenuGuard::Ignore => return Task::none(),
            MenuGuard::Dismiss => return close_all_context_menus(app),
        }
    }

    let dispatch_task = routers::route(app, event);
    if pre_dispatch_tasks.is_empty() {
        dispatch_task
    } else {
        pre_dispatch_tasks.push(dispatch_task);
        Task::batch(pre_dispatch_tasks)
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
        routers::route(app, AppEvent::SidebarUi(SidebarEvent::DismissAddMenu)),
        routers::route(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Ui(
                QuickLaunchUiEvent::ContextMenuDismiss,
            )),
        ),
        routers::route(
            app,
            AppEvent::TerminalWorkspaceCommand(
                TerminalWorkspaceCommand::CloseAllContextMenus,
            ),
        ),
    ])
}
