//! `letter_icon`: the first letter of a name on a rounded square, the icon
//! of something that has no icon of its own, such as a provider no plugin
//! serves.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{container, text};
use iced::{Border, Color, Element, Length};

use crate::palette::UiPalette;
use crate::{fonts, radius, size};

/// The part of an edge the letter fills.
const LETTER_SHARE: f32 = 0.62;

/// Renders the first letter of `name`, in upper case, in `color` on a
/// rounded square of `edge` pixels with the kit's line around it. It
/// stands where an SVG icon of the same edge would, such as the icon of a
/// provider no plugin serves (`E` for `example`).
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::letter_icon::letter_icon;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let icon: Element<'_, ()> =
///     letter_icon("api.example.com", 16.0, palette.text_strong(), &palette);
/// ```
pub fn letter_icon<'a, Message: 'a>(
    name: &str,
    edge: f32,
    color: Color,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let line = palette.line();

    let letter = text(first_letter(name))
        .size(edge * LETTER_SHARE)
        .font(fonts::UI_SEMIBOLD)
        .color(color)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    container(letter)
        .center_x(Length::Fixed(edge))
        .center_y(Length::Fixed(edge))
        .style(move |_| container::Style {
            border: Border {
                color: line,
                width: size::LINE,
                radius: radius::RADIUS_XS.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// The first letter or digit of `name` in upper case; `?` for a name with
/// none.
fn first_letter(name: &str) -> String {
    name.chars()
        .find(|character| character.is_alphanumeric())
        .map_or_else(
            || String::from("?"),
            |first| first.to_uppercase().collect(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_letter_is_the_first_letter_of_the_name_in_upper_case() {
        assert_eq!(first_letter("example"), "E");
        assert_eq!(first_letter("api.example.com"), "A");
        assert_eq!(first_letter("  _mistral"), "M");
        assert_eq!(first_letter("ёлка"), "Ё");
        assert_eq!(first_letter("--"), "?");
        assert_eq!(first_letter(""), "?");
    }
}
