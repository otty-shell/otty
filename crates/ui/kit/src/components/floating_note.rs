//! `FloatingNote`: small pill that floats over a pane.

use iced::widget::{container, row, text};
use iced::{Alignment, Border, Element, Padding};

use super::kbd::kbd;
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Side padding of the pill (`padding: 0 12px`).
const PADDING_X: f32 = spacing::SPACE_4;
/// Gap between parts (`gap: 10px`).
const GAP: f32 = 10.0;

/// One piece of a [`FloatingNote`].
enum Part {
    Text(String),
    Muted(String),
    Key(String),
}

/// Pill on `surface-overlay` with `shadow-popover`, made of text parts,
/// muted parts and shortcut hints. Place it with a `stack` or an overlay.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::floating_note::FloatingNote;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let note: Element<'_, ()> = FloatingNote::new(&palette)
///     .text("Focus mode")
///     .muted("Panels are hidden")
///     .kbd("Ctrl Shift F")
///     .muted("to show them")
///     .into();
/// ```
pub struct FloatingNote {
    palette: UiPalette,
    parts: Vec<Part>,
}

impl FloatingNote {
    /// Creates an empty note.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            parts: Vec::new(),
        }
    }

    /// Adds a part in the body text color.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.parts.push(Part::Text(text.into()));
        self
    }

    /// Adds a part in the muted text color.
    pub fn muted(mut self, text: impl Into<String>) -> Self {
        self.parts.push(Part::Muted(text.into()));
        self
    }

    /// Fades the whole note to `opacity`, from 0.0 (invisible) to 1.0 (fully
    /// shown).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.palette = self.palette.with_opacity(opacity);
        self
    }

    /// Adds a shortcut hint.
    pub fn kbd(mut self, shortcut: impl Into<String>) -> Self {
        self.parts.push(Part::Key(shortcut.into()));
        self
    }
}

impl<'a, Message: 'a> From<FloatingNote> for Element<'a, Message> {
    fn from(value: FloatingNote) -> Self {
        let palette = value.palette;
        let style = typography::SMALL;

        let label = |content: String, color| {
            text(content)
                .size(style.size())
                .line_height(iced::Pixels(style.line_height()))
                .font(fonts::UI)
                .color(color)
        };

        let mut content = row![].spacing(GAP).align_y(Alignment::Center);
        for part in value.parts {
            content = match part {
                Part::Text(part) => {
                    content.push(label(part, palette.text_body()))
                },
                Part::Muted(part) => {
                    content.push(label(part, palette.text_muted()))
                },
                Part::Key(part) => content.push(kbd(part, &palette)),
            };
        }

        let background = palette.surface_overlay();
        let shadow = palette.shadow_popover();

        container(content)
            .height(size::FLOATING_NOTE)
            .align_y(Alignment::Center)
            .padding(Padding::from([0.0, PADDING_X]))
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                shadow,
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

    #[test]
    fn given_parts_when_rendered_then_each_part_is_shown_in_order() {
        let palette = UiPalette::otty_dark();
        let note = FloatingNote::new(&palette)
            .text("Focus mode")
            .muted("Panels are hidden")
            .kbd("Ctrl Shift F")
            .muted("to show them");
        let note: Element<'_, ()> = note.into();
        let mut ui = simulator(note);

        assert!(ui.find("Focus mode").is_ok());
        assert!(ui.find("Panels are hidden").is_ok());
        assert!(ui.find("Ctrl Shift F").is_ok());
        assert!(ui.find("to show them").is_ok());
    }
}
