//! `Kbd`: keyboard shortcut hint.

use iced::keyboard::Modifiers;
use iced::widget::{container, text};
use iced::{Border, Element, Padding};

use crate::palette::UiPalette;
use crate::{fonts, radius, typography};

/// Horizontal padding of a key hint (`padding: 1px 6px`).
const PADDING_X: f32 = 6.0;
/// Vertical padding of a key hint.
const PADDING_Y: f32 = 1.0;

/// Builds the text of a shortcut.
///
/// Modifiers come first in the order Ctrl, Alt, Logo, Shift. `logo_name` is
/// how the Logo key is written: "Super" on Linux, "Cmd" on macOS. A single
/// character key is upper-cased; longer key names are kept as given.
///
/// # Example
///
/// ```
/// use iced::keyboard::Modifiers;
/// use otty_ui_kit::components::kbd::shortcut_text;
///
/// let modifiers = Modifiers::CTRL | Modifiers::SHIFT;
/// assert_eq!(shortcut_text(modifiers, "c", "Super"), "Ctrl Shift C");
/// ```
pub fn shortcut_text(
    modifiers: Modifiers,
    key: &str,
    logo_name: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if modifiers.control() {
        parts.push("Ctrl".to_owned());
    }
    if modifiers.alt() {
        parts.push("Alt".to_owned());
    }
    if modifiers.logo() {
        parts.push(logo_name.to_owned());
    }
    if modifiers.shift() {
        parts.push("Shift".to_owned());
    }

    if key.chars().count() == 1 {
        parts.push(key.to_uppercase());
    } else {
        parts.push(key.to_owned());
    }

    parts.join(" ")
}

/// Renders a shortcut hint such as `Ctrl Shift C`.
///
/// Use it only for real key bindings. Mono 10.5/14, `text-muted` on
/// `surface-raised`, `radius-xs`.
pub fn kbd<'a, Message: 'a>(
    label: impl Into<String>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::KBD;
    let color = palette.text_muted();
    let background = palette.surface_raised();

    container(
        text(label.into())
            .font(fonts::MONO)
            .size(style.size())
            .line_height(iced::Pixels(style.line_height()))
            .color(color),
    )
    .padding(Padding::from([PADDING_Y, PADDING_X]))
    .style(move |_| container::Style {
        background: Some(background.into()),
        border: Border {
            radius: radius::RADIUS_XS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_text_orders_modifiers_and_uppercases_key() {
        let modifiers = Modifiers::SHIFT | Modifiers::CTRL;

        assert_eq!(shortcut_text(modifiers, "c", "Super"), "Ctrl Shift C");
    }

    #[test]
    fn shortcut_text_keeps_named_keys() {
        assert_eq!(shortcut_text(Modifiers::ALT, "1", "Super"), "Alt 1");
        assert_eq!(
            shortcut_text(Modifiers::empty(), "Enter", "Super"),
            "Enter"
        );
    }

    #[test]
    fn shortcut_text_puts_logo_before_shift() {
        let modifiers = Modifiers::SHIFT | Modifiers::LOGO | Modifiers::CTRL;

        assert_eq!(
            shortcut_text(modifiers, "t", "Super"),
            "Ctrl Super Shift T"
        );
    }

    #[test]
    fn shortcut_text_uses_the_given_logo_name() {
        let modifiers = Modifiers::LOGO | Modifiers::SHIFT;

        assert_eq!(shortcut_text(modifiers, "e", "Cmd"), "Cmd Shift E");
    }
}
