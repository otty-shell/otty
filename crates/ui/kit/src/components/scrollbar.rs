//! Thin scrollbar for panels, trees and forms.

use iced::widget::scrollable::{
    self, Direction, Rail, Scrollbar, Scroller, Status, Style,
};
use iced::{Border, Color, Theme};

use crate::palette::UiPalette;

/// Width of the bar and its scroller. The design system has no size token
/// for scrollbars; 4px keeps the bar out of the way of panel content.
const WIDTH: f32 = 4.0;

/// Vertical scrolling with a thin bar on the right edge of the content.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::{scrollable, text};
/// use otty_ui_kit::components::scrollbar;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let list: Element<'_, ()> = scrollable(text("Rows"))
///     .direction(scrollbar::vertical())
///     .style(scrollbar::style(&palette))
///     .into();
/// ```
pub fn vertical() -> Direction {
    Direction::Vertical(
        Scrollbar::new()
            .width(WIDTH)
            .margin(0)
            .scroller_width(WIDTH),
    )
}

/// Horizontal scrolling with a thin bar on the bottom edge of the content.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::{scrollable, text};
/// use otty_ui_kit::components::scrollbar;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let wide: Element<'_, ()> = scrollable(text("A wide row"))
///     .direction(scrollbar::horizontal())
///     .style(scrollbar::style(&palette))
///     .into();
/// ```
pub fn horizontal() -> Direction {
    Direction::Horizontal(
        Scrollbar::new()
            .width(WIDTH)
            .margin(0)
            .scroller_width(WIDTH),
    )
}

/// Scrolling both ways, with a thin bar on the right and on the bottom edge
/// of the content. Each bar shows only while the content overflows that way,
/// floats over the content and takes no room from it.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::{scrollable, text};
/// use otty_ui_kit::components::scrollbar;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let page: Element<'_, ()> = scrollable(text("A large page"))
///     .direction(scrollbar::both())
///     .style(scrollbar::style(&palette))
///     .into();
/// ```
pub fn both() -> Direction {
    let bar = Scrollbar::new()
        .width(WIDTH)
        .margin(0)
        .scroller_width(WIDTH);

    Direction::Both {
        vertical: bar,
        horizontal: bar,
    }
}

/// Style of a thin scrollbar: square, without a track, with a scroller in
/// [`UiPalette::scrollbar_idle`] that turns `text-faint` while the pointer is
/// on it or drags it.
pub fn style(
    palette: &UiPalette,
) -> impl Fn(&Theme, Status) -> Style + 'static {
    let idle = palette.scrollbar_idle();
    let active = palette.text_faint();

    move |theme, status| {
        let (vertical_active, horizontal_active) = match status {
            Status::Hovered {
                is_vertical_scrollbar_hovered,
                is_horizontal_scrollbar_hovered,
                ..
            } => (
                is_vertical_scrollbar_hovered,
                is_horizontal_scrollbar_hovered,
            ),
            Status::Dragged {
                is_vertical_scrollbar_dragged,
                is_horizontal_scrollbar_dragged,
                ..
            } => (
                is_vertical_scrollbar_dragged,
                is_horizontal_scrollbar_dragged,
            ),
            _ => (false, false),
        };

        Style {
            vertical_rail: rail(if vertical_active { active } else { idle }),
            horizontal_rail: rail(if horizontal_active {
                active
            } else {
                idle
            }),
            ..scrollable::default(theme, status)
        }
    }
}

fn rail(scroller: Color) -> Rail {
    Rail {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            background: scroller.into(),
            border: Border::default(),
        },
    }
}

#[cfg(test)]
mod tests {
    use iced::Background;

    use super::*;

    fn scroller_color(palette: &UiPalette, status: Status) -> Option<Color> {
        match style(palette)(&Theme::Light, status)
            .vertical_rail
            .scroller
            .background
        {
            Background::Color(color) => Some(color),
            _ => None,
        }
    }

    #[test]
    fn given_a_bar_at_rest_then_it_has_the_idle_color_of_the_theme() {
        let palette = UiPalette::otty_light();
        let status = Status::Active {
            is_horizontal_scrollbar_disabled: false,
            is_vertical_scrollbar_disabled: false,
        };

        assert_eq!(
            scroller_color(&palette, status),
            Some(palette.scrollbar_idle())
        );
    }

    #[test]
    fn given_the_pointer_on_a_bar_then_it_turns_text_faint() {
        let palette = UiPalette::otty_light();
        let status = Status::Hovered {
            is_horizontal_scrollbar_hovered: false,
            is_vertical_scrollbar_hovered: true,
            is_horizontal_scrollbar_disabled: false,
            is_vertical_scrollbar_disabled: false,
        };

        assert_eq!(
            scroller_color(&palette, status),
            Some(palette.text_faint())
        );
    }
}
