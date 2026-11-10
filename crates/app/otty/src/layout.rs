use iced::{Padding, Size};
use otty_ui_kit::size;

/// Where the main area lies in the window: under the title bar and the line
/// under it, over the status bar, across the whole width. A layer over the
/// whole window puts an element in the main area with this padding.
pub(crate) fn main_area_padding() -> Padding {
    Padding {
        top: size::TITLEBAR + size::LINE,
        bottom: size::STATUSBAR,
        ..Padding::ZERO
    }
}

/// Size of the main area between the header and the status bar: the window
/// height without the title bar, the line under it and the status bar. The
/// header menus are kept inside it.
pub(crate) fn screen_size_from_window(window_size: Size) -> Size {
    let height = (window_size.height - main_area_padding().y()).max(0.0);

    Size::new(window_size.width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_area_lies_under_the_title_bar_and_its_line_and_over_the_status_bar()
    {
        let padding = main_area_padding();

        assert_eq!(padding.top, 38.0 + 1.0);
        assert_eq!(padding.bottom, 24.0);
        assert_eq!(padding.left, 0.0);
        assert_eq!(padding.right, 0.0);
    }

    #[test]
    fn given_window_when_screen_size_computed_then_title_bar_separator_and_status_bar_are_removed()
     {
        let window = Size::new(1000.0, 700.0);

        let screen = screen_size_from_window(window);

        assert_eq!(screen.width, 1000.0);
        assert_eq!(screen.height, 700.0 - 38.0 - 1.0 - 24.0);
    }

    #[test]
    fn given_window_shorter_than_header_and_status_bar_when_screen_size_computed_then_height_is_zero()
     {
        let screen = screen_size_from_window(Size::new(800.0, 50.0));

        assert_eq!(screen.height, 0.0);
    }
}
