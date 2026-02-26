use iced::{Point, Size};

use crate::app::App;
use crate::state::pane_grid_size;
use crate::widgets::sidebar;
use crate::widgets::terminal::TerminalCtx;

/// Compute current terminal pane-grid size from window/sidebar state.
pub(crate) fn current_pane_grid_size(app: &App) -> Size {
    let sidebar = app.widgets.sidebar();
    pane_grid_size(
        app.screen_size(),
        sidebar.is_hidden(),
        sidebar::SIDEBAR_MENU_WIDTH,
        sidebar.effective_workspace_ratio(),
    )
}

/// Build terminal widget reduction context from app snapshot.
pub(crate) fn make_terminal_ctx(app: &App) -> TerminalCtx {
    TerminalCtx {
        active_tab_id: app.widgets.tab().active_tab_id(),
        pane_grid_size: current_pane_grid_size(app),
        screen_size: app.screen_size(),
        sidebar_cursor: app.widgets.sidebar().cursor(),
    }
}

/// Sync current terminal grid sizes into terminal widget state.
pub(crate) fn sync_terminal_grid_sizes(app: &mut App) {
    let size = current_pane_grid_size(app);
    app.widgets.terminal_mut().set_grid_size(size);
}

/// Build quick-launch context from explicit values.
pub(crate) fn quick_launch_ctx_from_values<'a>(
    terminal_settings: &'a otty_ui_term::settings::Settings,
    sidebar_cursor: Point,
    sidebar_is_resizing: bool,
) -> crate::widgets::quick_launch::QuickLaunchCtx<'a> {
    crate::widgets::quick_launch::QuickLaunchCtx {
        terminal_settings,
        sidebar_cursor,
        sidebar_is_resizing,
    }
}
