use iced::Padding;
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
}
