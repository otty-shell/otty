use iced::Size;

use crate::ui::widgets::tab_bar;

#[derive(Default)]
pub(crate) struct State {
    pub(crate) window_size: Size,
    pub(crate) screen_size: Size,
}

impl State {
    pub(crate) fn new(window_size: Size, screen_size: Size) -> Self {
        Self {
            window_size,
            screen_size,
        }
    }

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
    let tab_bar_height = tab_bar::TAB_BAR_HEIGHT;
    let height = (screen_size.height - tab_bar_height).max(0.0);

    let menu_width = if sidebar_is_hidden {
        0.0
    } else {
        sidebar_menu_width
    };

    let available_width = (screen_size.width - menu_width).max(0.0);
    let width = (available_width * (1.0 - sidebar_workspace_ratio)).max(0.0);

    Size::new(width, height)
}
