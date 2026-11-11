//! `StatusBadge`: colored dot and word that tell the state of an item.

use iced::widget::text::Wrapping;
use iced::widget::{row, text};
use iced::{Alignment, Color, Element};

use super::target_dot::dot;
use crate::fonts;
use crate::palette::UiPalette;

/// Gap between the dot and the word (`.ot-status { gap: 7px }`).
const GAP: f32 = 7.0;
/// Text size of the word (`.ot-table { font-size: 12.5px }`,
/// `.ot-insp-status { font-size: 12.5px }`).
pub(super) const TEXT_SIZE: f32 = 12.5;

/// Tone of a [`StatusBadge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    /// `status-ok`: running, healthy.
    Ok,
    /// `status-warn`: paused, restarting, needs a look.
    Warn,
    /// `status-error`: failed, dead.
    Error,
    /// `text-muted` word and dot: created, stopped cleanly.
    Neutral,
}

impl StatusTone {
    /// Returns the color of the word in `palette`.
    pub const fn color(self, palette: &UiPalette) -> Color {
        match self {
            Self::Ok => palette.status_ok(),
            Self::Warn => palette.status_warn(),
            Self::Error => palette.status_error(),
            Self::Neutral => palette.text_muted(),
        }
    }

    /// Returns the color of the dot in `palette`.
    pub const fn dot_color(self, palette: &UiPalette) -> Color {
        // `text-faint` is under 3:1 on `surface-selected`, so the neutral
        // dot takes the color of the word too.
        self.color(palette)
    }
}

/// Dot followed by a short state word, such as "Running" or "Exited (1)".
///
/// The word carries the meaning; the dot only repeats it in color.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::status_badge::{StatusBadge, StatusTone};
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let badge: Element<'_, ()> =
///     StatusBadge::new("Running", StatusTone::Ok, &palette).into();
/// ```
pub struct StatusBadge {
    label: String,
    tone: StatusTone,
    palette: UiPalette,
    is_wrapping: bool,
}

impl StatusBadge {
    /// Creates a badge with `label` in `tone`.
    pub fn new(
        label: impl Into<String>,
        tone: StatusTone,
        palette: &UiPalette,
    ) -> Self {
        Self {
            label: label.into(),
            tone,
            palette: palette.clone(),
            is_wrapping: false,
        }
    }

    /// Sets whether a label wider than the space goes on the next line
    /// (for a status line in a panel) instead of being cut at the edge
    /// (for a table cell). Off by default.
    pub fn wrapping(mut self, is_wrapping: bool) -> Self {
        self.is_wrapping = is_wrapping;
        self
    }
}

impl<'a, Message: 'a> From<StatusBadge> for Element<'a, Message> {
    fn from(value: StatusBadge) -> Self {
        let color = value.tone.color(&value.palette);
        let dot_color = value.tone.dot_color(&value.palette);
        let wrapping = if value.is_wrapping {
            Wrapping::WordOrGlyph
        } else {
            Wrapping::None
        };

        row![
            dot(dot_color),
            text(value.label)
                .size(TEXT_SIZE)
                .font(fonts::UI)
                .color(color)
                .wrapping(wrapping),
        ]
        .spacing(GAP)
        .align_y(Alignment::Center)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::contrast::contrast_ratio;

    /// WCAG 2.1 AA minimum for normal text.
    const TEXT_MIN: f32 = 4.5;
    /// WCAG 2.1 AA minimum for non-text marks.
    const NON_TEXT_MIN: f32 = 3.0;
    const TONES: [StatusTone; 4] = [
        StatusTone::Ok,
        StatusTone::Warn,
        StatusTone::Error,
        StatusTone::Neutral,
    ];

    fn themes() -> [(&'static str, UiPalette); 2] {
        [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ]
    }

    #[test]
    fn given_each_theme_when_badge_words_on_table_rows_then_each_reaches_4_5() {
        for (theme, palette) in themes() {
            let surfaces = [
                ("surface-canvas", palette.surface_canvas()),
                ("surface-header", palette.surface_header()),
                ("surface-selected", palette.surface_selected()),
            ];

            for tone in TONES {
                for (surface_name, surface) in surfaces {
                    let ratio = contrast_ratio(tone.color(&palette), surface);

                    assert!(
                        ratio >= TEXT_MIN,
                        "{theme}: {tone:?} word on {surface_name} is {ratio:.2}:1"
                    );
                }
            }
        }
    }

    #[test]
    fn given_each_theme_when_badge_dots_on_canvas_then_each_reaches_3() {
        for (theme, palette) in themes() {
            let surfaces = [
                ("surface-canvas", palette.surface_canvas()),
                ("surface-header", palette.surface_header()),
            ];

            for tone in TONES {
                for (surface_name, surface) in surfaces {
                    let ratio =
                        contrast_ratio(tone.dot_color(&palette), surface);

                    assert!(
                        ratio >= NON_TEXT_MIN,
                        "{theme}: {tone:?} dot on {surface_name} is {ratio:.2}:1"
                    );
                }
            }
        }
    }

    #[test]
    fn given_each_tone_when_colors_read_then_they_are_their_tokens() {
        let palette = UiPalette::otty_dark();

        assert_eq!(StatusTone::Ok.color(&palette), palette.status_ok());
        assert_eq!(StatusTone::Warn.color(&palette), palette.status_warn());
        assert_eq!(StatusTone::Error.color(&palette), palette.status_error());
        assert_eq!(StatusTone::Neutral.color(&palette), palette.text_muted());
        assert_eq!(StatusTone::Ok.dot_color(&palette), palette.status_ok());
        assert_eq!(
            StatusTone::Neutral.dot_color(&palette),
            palette.text_muted()
        );
    }

    #[test]
    fn given_badge_when_rendered_then_its_word_shows() {
        let palette = UiPalette::otty_light();
        let badge: Element<'_, ()> =
            StatusBadge::new("Exited (1)", StatusTone::Error, &palette).into();
        let mut ui = simulator(badge);

        assert!(ui.find("Exited (1)").is_ok());
    }

    #[test]
    fn given_long_label_when_wrapping_on_then_it_wraps_inside_the_width() {
        let palette = UiPalette::otty_dark();
        let label = "Exited (137) 3 hours ago, restarting soon";
        let badge: Element<'_, ()> =
            StatusBadge::new(label, StatusTone::Error, &palette)
                .wrapping(true)
                .into();
        let mut ui = simulator(iced::widget::container(badge).width(120));

        let word = ui.find(label).expect("label should be shown").bounds();

        assert!(
            word.x + word.width <= 120.5,
            "label {word:?} should stay inside 120px"
        );
        assert!(
            word.height > TEXT_SIZE * 2.0,
            "label {word:?} should wrap onto several lines"
        );
    }

    #[test]
    fn given_long_label_when_wrapping_is_off_then_it_stays_on_one_line() {
        let palette = UiPalette::otty_dark();
        let label = "Exited (137) 3 hours ago, restarting soon";
        let badge: Element<'_, ()> =
            StatusBadge::new(label, StatusTone::Error, &palette).into();
        let mut ui = simulator(iced::widget::container(badge).width(120));

        let word = ui.find(label).expect("label should be shown").bounds();

        assert!(
            word.height < TEXT_SIZE * 2.0,
            "label {word:?} should be one line"
        );
    }
}
