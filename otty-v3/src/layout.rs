use iced::Size;

use crate::app::view::HEADER_SEPARATOR_HEIGHT;
use crate::widgets::chrome::view::action_bar::ACTION_BAR_HEIGHT;
use crate::widgets::tabs::view::tab_bar::TAB_BAR_HEIGHT;

// TODO:
pub(crate) fn screen_size_from_window(window_size: Size) -> Size {
    let height =
        (window_size.height - ACTION_BAR_HEIGHT - HEADER_SEPARATOR_HEIGHT)
            .max(0.0);

    Size::new(window_size.width, height)
}

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
