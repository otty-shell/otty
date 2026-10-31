use iced::Size;
use otty_ui_kit::{radius, size};

use crate::app::view::HEADER_SEPARATOR_HEIGHT;
use crate::widgets::tabs::view::tab_bar::TAB_BAR_HEIGHT;

/// Shared compact control size used by dense toolbars and menus.
pub(crate) const BUTTON_SIZE_COMPACT: f32 = size::CONTROL_SM;
/// Shared regular control size used by form actions.
pub(crate) const BUTTON_SIZE_REGULAR: f32 = size::CONTROL_MD;
/// Shared rounded corner radius for standard buttons.
pub(crate) const BUTTON_RADIUS_ROUNDED: f32 = radius::RADIUS_SM;

pub(crate) fn screen_size_from_window(window_size: Size) -> Size {
    let height =
        (window_size.height - size::TITLEBAR - HEADER_SEPARATOR_HEIGHT)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_window_when_screen_size_computed_then_title_bar_and_separator_are_removed()
     {
        let window = Size::new(1000.0, 700.0);

        let screen = screen_size_from_window(window);

        assert_eq!(screen.width, 1000.0);
        assert_eq!(screen.height, 700.0 - 38.0 - 1.0);
    }

    #[test]
    fn given_window_shorter_than_header_when_screen_size_computed_then_height_is_zero()
     {
        let screen = screen_size_from_window(Size::new(800.0, 20.0));

        assert_eq!(screen.height, 0.0);
    }

    #[test]
    fn given_visible_rail_when_pane_grid_computed_then_rail_width_is_48() {
        let screen = Size::new(1000.0, 700.0);

        let grid = pane_grid_size(screen, false, size::RAIL, 0.0);

        assert_eq!(size::RAIL, 48.0);
        assert_eq!(grid.width, 1000.0 - 48.0);
    }

    #[test]
    fn given_hidden_sidebar_when_pane_grid_computed_then_rail_is_not_counted() {
        let screen = Size::new(1000.0, 700.0);

        let grid = pane_grid_size(screen, true, size::RAIL, 0.0);

        assert_eq!(grid.width, 1000.0);
    }
}
