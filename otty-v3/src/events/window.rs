use iced::{Task, window};

use crate::app::App;
use crate::helpers::{screen_size_from_window, pane_grid_size};
use super::AppEvent;

pub(super) fn handle_resize(app: &mut App, size: iced::Size) -> Task<AppEvent> {
    app.window_size = size;
    app.state.window_size = size;
    app.state
        .set_screen_size(screen_size_from_window(size));

    sync_terminal_grid_sizes(app);
    Task::none()
}

pub(super) fn handle_drag_resize(dir: window::Direction) -> Task<AppEvent> {
    window::latest().and_then(move |id| window::drag_resize(id, dir))
}

pub(super) fn sync_terminal_grid_sizes(app: &mut App) {
    let size = current_pane_grid_size(app);
    app.widgets.terminal_workspace.set_grid_size(size);
}

pub(super) fn current_pane_grid_size(app: &App) -> iced::Size {
    let sidebar = &app.widgets.sidebar;

    pane_grid_size(
        app.state.screen_size,
        sidebar.is_hidden(),
        crate::widgets::sidebar::SIDEBAR_MENU_WIDTH,
        sidebar.effective_workspace_ratio(),
    )
}
