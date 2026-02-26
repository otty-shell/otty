use iced::{Task, window};

use crate::app::{App, AppEvent};

/// Handle window resize events and propagate layout changes.
pub(crate) fn handle_resize(app: &mut App, size: iced::Size) -> Task<AppEvent> {
    app.window_size = size;
    app.state.window_size = size;
    app.state
        .set_screen_size(crate::app::view::screen_size_from_window(size));
    sync_terminal_grid_sizes(app);
    Task::none()
}

/// Handle window drag-resize from resize grips.
pub(crate) fn handle_drag_resize(dir: window::Direction) -> Task<AppEvent> {
    window::latest().and_then(move |id| window::drag_resize(id, dir))
}

/// Propagate the current pane grid size to terminal features.
pub(crate) fn sync_terminal_grid_sizes(app: &mut App) {
    let size = current_pane_grid_size(app);
    app.widgets.terminal_workspace.set_grid_size(size);
}

/// Compute the current pane grid size from app state.
pub(crate) fn current_pane_grid_size(app: &App) -> iced::Size {
    let sidebar = &app.widgets.sidebar;
    crate::state::pane_grid_size(
        app.state.screen_size,
        sidebar.is_hidden(),
        crate::widgets::sidebar::SIDEBAR_MENU_WIDTH,
        sidebar.effective_workspace_ratio(),
    )
}
