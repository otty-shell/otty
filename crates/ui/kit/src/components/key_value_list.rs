//! `KeyValueList`: two-column list of facts about one item.

use iced::widget::text::Wrapping;
use iced::widget::{column, row, text};
use iced::{Element, Length};

use crate::palette::UiPalette;
use crate::{fonts, size, spacing};

/// Text size of keys and values (`font-size: 12.5px`).
const TEXT_SIZE: f32 = 12.5;

/// Two columns: muted keys at a fixed width, mono values that wrap.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::key_value_list::KeyValueList;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let list: Element<'_, ()> = KeyValueList::new(&palette)
///     .entry("Image", "nginx:1.27")
///     .entry("Ports", "8080:80/tcp")
///     .into();
/// ```
pub struct KeyValueList {
    palette: UiPalette,
    entries: Vec<(String, String)>,
}

impl KeyValueList {
    /// Creates an empty list.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            entries: Vec::new(),
        }
    }

    /// Adds a `key` and its `value` below the previous entry.
    pub fn entry(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.entries.push((key.into(), value.into()));
        self
    }
}

impl<'a, Message: 'a> From<KeyValueList> for Element<'a, Message> {
    fn from(value: KeyValueList) -> Self {
        let palette = value.palette;
        let mut lines = column![].spacing(spacing::SPACE_3);

        for (key, entry_value) in value.entries {
            lines = lines.push(
                row![
                    text(key)
                        .size(TEXT_SIZE)
                        .font(fonts::UI)
                        .wrapping(Wrapping::Glyph)
                        .color(palette.text_muted())
                        .width(size::KEY_VALUE_KEY),
                    // Values break at any character so a long id or path
                    // never pushes the list wider than its panel.
                    text(entry_value)
                        .size(TEXT_SIZE)
                        .font(fonts::MONO)
                        .wrapping(Wrapping::Glyph)
                        .color(palette.text_body())
                        .width(Length::Fill),
                ]
                .spacing(spacing::SPACE_3),
            );
        }

        lines.into()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[test]
    fn given_entries_when_rendered_then_keys_and_values_are_shown() {
        let palette = UiPalette::otty_dark();
        let list: Element<'_, ()> = KeyValueList::new(&palette)
            .entry("Image", "nginx:1.27")
            .entry("Ports", "8080:80/tcp")
            .into();
        let mut ui = simulator(list);

        for label in ["Image", "nginx:1.27", "Ports", "8080:80/tcp"] {
            assert!(ui.find(label).is_ok(), "{label} should be shown");
        }
    }

    #[test]
    fn given_entry_when_laid_out_then_value_starts_after_key_column() {
        let palette = UiPalette::otty_light();
        let list: Element<'_, ()> =
            KeyValueList::new(&palette).entry("Image", "nginx").into();
        let mut ui = simulator(list);

        let key = ui.find("Image").expect("key should be shown").bounds();
        let value = ui.find("nginx").expect("value should be shown").bounds();

        assert!(
            (value.x - key.x - size::KEY_VALUE_KEY - spacing::SPACE_3).abs()
                < 0.5,
            "value x {} should be {} right of key x {}",
            value.x,
            size::KEY_VALUE_KEY + spacing::SPACE_3,
            key.x
        );
    }

    #[test]
    fn given_long_value_when_laid_out_then_it_wraps_inside_the_width() {
        let palette = UiPalette::otty_dark();
        let long = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef";
        let list: Element<'_, ()> = iced::widget::container(
            KeyValueList::new(&palette).entry("ID", long),
        )
        .width(200)
        .into();
        let mut ui = simulator(list);

        let value = ui.find(long).expect("value should be shown").bounds();

        assert!(
            value.x + value.width <= 200.5,
            "value {value:?} should stay inside 200px"
        );
        assert!(
            value.height > TEXT_SIZE * 2.0,
            "value {value:?} should wrap onto several lines"
        );
    }
}
