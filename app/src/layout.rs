use iced::Size;
use otty_ui_kit::size;

pub(crate) fn screen_size_from_window(window_size: Size) -> Size {
    let height = (window_size.height - size::TITLEBAR - size::LINE).max(0.0);

    Size::new(window_size.width, height)
}

/// Size of the terminal pane grid: the full screen height and the width
/// left after the shown side panels.
pub(crate) fn pane_grid_size(
    screen_size: Size,
    side_panels_ratio: f32,
) -> Size {
    let width = (screen_size.width * (1.0 - side_panels_ratio)).max(0.0);

    Size::new(width, screen_size.height)
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
    fn given_no_side_panel_when_pane_grid_computed_then_main_takes_the_full_width()
     {
        let screen = Size::new(1000.0, 700.0);

        let grid = pane_grid_size(screen, 0.0);

        assert_eq!(grid.width, 1000.0);
    }

    #[test]
    fn given_side_panels_when_pane_grid_computed_then_their_share_is_removed() {
        let screen = Size::new(1000.0, 700.0);

        let grid = pane_grid_size(screen, 0.4);

        assert_eq!(grid.width, 600.0);
    }

    #[test]
    fn given_screen_when_pane_grid_computed_then_it_takes_the_full_height() {
        let screen = Size::new(1000.0, 700.0);

        let grid = pane_grid_size(screen, 0.0);

        assert_eq!(grid.height, 700.0);
    }
}
