use iced::{Task, window};

use super::{quick_launch, runtime, sidebar};
use crate::app::{App, Event as AppEvent};
use crate::guards::{MenuGuard, context_menu_guard};
use crate::ui::widgets::action_bar;
use crate::widgets::sidebar::SidebarUiEvent;
use crate::widgets::terminal::TerminalEvent;

/// Resolve context-menu guard and return early task when dispatch must stop.
pub(crate) fn resolve_context_menu_guard(
    app: &mut App,
    event: &AppEvent,
) -> Option<Task<AppEvent>> {
    if any_context_menu_open(app) {
        match context_menu_guard(event) {
            MenuGuard::Allow => None,
            MenuGuard::Ignore => Some(Task::none()),
            MenuGuard::Dismiss => Some(close_all_context_menus(app)),
        }
    } else {
        None
    }
}

/// Route action-bar UI event into app window actions.
pub(crate) fn route_action_bar(
    app: &mut App,
    event: action_bar::ActionBarEvent,
) -> Task<AppEvent> {
    use action_bar::ActionBarEvent::*;

    match event {
        ToggleFullScreen => {
            let mode = app.toggle_fullscreen_mode();
            window::latest().and_then(move |id| window::set_mode(id, mode))
        },
        MinimizeWindow => {
            window::latest().and_then(|id| window::minimize(id, true))
        },
        CloseWindow => iced::window::latest().and_then(iced::window::close),
        ToggleSidebarVisibility => {
            sidebar::route_event(app, SidebarUiEvent::ToggleVisibility)
        },
        StartWindowDrag => window::latest().and_then(window::drag),
    }
}

/// Route window runtime event.
pub(crate) fn route_window_event(
    app: &mut App,
    event: iced::window::Event,
) -> Task<AppEvent> {
    match event {
        window::Event::Resized(size) => {
            app.set_window_size(size);
            runtime::sync_terminal_grid_sizes(app);
            Task::none()
        },
        _ => Task::none(),
    }
}

/// Route explicit resize-window command.
pub(crate) fn route_resize_window(
    dir: iced::window::Direction,
) -> Task<AppEvent> {
    window::latest().and_then(move |id| window::drag_resize(id, dir))
}

pub(crate) fn any_context_menu_open(app: &App) -> bool {
    if app.widgets.sidebar().has_add_menu_open()
        || app.widgets.quick_launch().context_menu().is_some()
    {
        return true;
    }

    app.widgets.terminal().has_any_context_menu()
}

fn close_all_context_menus(app: &mut App) -> Task<AppEvent> {
    let sidebar_task =
        sidebar::route_event(app, SidebarUiEvent::DismissAddMenu);
    let quick_launch_task = quick_launch::route_event(
        app,
        crate::widgets::quick_launch::QuickLaunchEvent::ContextMenuDismiss,
    );
    let ctx = runtime::make_terminal_ctx(app);
    let terminal_task = app
        .widgets
        .terminal_mut()
        .reduce(TerminalEvent::CloseAllContextMenus, &ctx);
    Task::batch(vec![sidebar_task, quick_launch_task, terminal_task])
}
