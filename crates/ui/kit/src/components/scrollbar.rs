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

/// Style of a thin scrollbar: square, without a track, with a
/// `surface-hover` scroller that turns `text-faint` while the pointer is on
/// it or drags it.
pub fn style(
    palette: &UiPalette,
) -> impl Fn(&Theme, Status) -> Style + 'static {
    let idle = palette.surface_hover();
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
