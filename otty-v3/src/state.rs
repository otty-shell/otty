use iced::Size;

/// Tab bar height for pane grid calculations.
const TAB_BAR_HEIGHT: f32 = 25.0;

/// Window and screen geometry state.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) window_size: Size,
    pub(crate) screen_size: Size,
}

impl State {
    /// Create state with the given initial sizes.
    pub(crate) fn new(window_size: Size, screen_size: Size) -> Self {
        Self {
            window_size,
            screen_size,
        }
    }

    /// Update the screen size after a window resize.
    pub(crate) fn set_screen_size(&mut self, size: Size) {
        self.screen_size = size;
    }
}

/// Compute available pane grid size from current screen and sidebar layout.
pub(crate) fn pane_grid_size(
    screen_size: Size,
    sidebar_is_hidden: bool,
    sidebar_menu_width: f32,
    sidebar_workspace_ratio: f32,
) -> Size {
    let height = (screen_size.height - TAB_BAR_HEIGHT).max(0.0);

    let menu_width = if sidebar_is_hidden {
        0.0
    } else {
        sidebar_menu_width
    };

    let available_width = (screen_size.width - menu_width).max(0.0);
    let width = (available_width * (1.0 - sidebar_workspace_ratio)).max(0.0);

    Size::new(width, height)
}
