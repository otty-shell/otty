use iced::Task;

use super::{App, AppEvent};
use crate::events;
use crate::guards::{
    MenuGuard, context_menu_guard, inline_edit_guard,
    is_new_session_menu_choice, is_terminal_input,
};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(super) fn update(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    // Keys typed while the command palette is open belong to the palette,
    // even if a terminal still had keyboard focus.
    if app.widgets.palette.is_open() && is_terminal_input(&event) {
        return Task::none();
    }

    let mut pre_dispatch_tasks = Vec::new();

    if app.widgets.quick_launch.has_inline_edit() && inline_edit_guard(&event) {
        pre_dispatch_tasks.push(events::handle(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::CancelInlineEdit,
            )),
        ));
    }

    if app.widgets.sidebar.has_new_session_menu_open()
        && is_new_session_menu_choice(&event)
    {
        pre_dispatch_tasks.push(events::handle(
            app,
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::NewSessionMenuDismiss,
            )),
        ));
    } else if any_context_menu_open(app) {
        match context_menu_guard(&event) {
            MenuGuard::Allow => {},
            MenuGuard::Ignore => return Task::none(),
            MenuGuard::Dismiss => return close_all_context_menus(app),
        }
    }

    let dispatch_task = events::handle(app, event);
    if pre_dispatch_tasks.is_empty() {
        dispatch_task
    } else {
        pre_dispatch_tasks.push(dispatch_task);
        Task::batch(pre_dispatch_tasks)
    }
}

fn any_context_menu_open(app: &App) -> bool {
    if app.widgets.sidebar.has_new_session_menu_open()
        || app.widgets.quick_launch.context_menu().is_some()
    {
        return true;
    }

    app.widgets.terminal_workspace.has_any_context_menu()
}

fn close_all_context_menus(app: &mut App) -> Task<AppEvent> {
    Task::batch(vec![
        events::handle(
            app,
            AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::NewSessionMenuDismiss,
            )),
        ),
        events::handle(
            app,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::ContextMenuDismiss,
            )),
        ),
        events::handle(
            app,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::CloseAllContextMenus,
            )),
        ),
    ])
}
