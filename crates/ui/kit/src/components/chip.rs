//! `Chip`: small label for a target or a path.

use iced::widget::text::Wrapping;
use iced::widget::{container, row, text};
use iced::{Alignment, Border, Element, Padding};

use super::target_dot::{TargetKind, target_dot};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Horizontal padding of a chip (`padding: 0 8px`).
const PADDING_X: f32 = spacing::SPACE_3;
/// Height of a chip (`height: 24px`, the same as `control-sm`).
const HEIGHT: f32 = size::CONTROL_SM;

/// Chip with a label, an optional target dot and an optional muted note.
///
/// A chip is always one line [`HEIGHT`] high. In a space narrower than its
/// texts it does not wrap: its right end is cut off at the edge of its
/// background, so the texts never spill out of it.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::chip::Chip;
/// use otty_ui_kit::components::target_dot::TargetKind;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let target: Element<'_, ()> = Chip::new("fedora", &palette)
///     .target(TargetKind::Local)
///     .note("this computer")
///     .into();
/// let path: Element<'_, ()> =
///     Chip::new("~/projects/otty", &palette).mono(true).into();
/// ```
pub struct Chip {
    label: String,
    palette: UiPalette,
    target: Option<TargetKind>,
    note: Option<String>,
    is_mono: bool,
}

impl Chip {
    /// Creates a chip with `label` only.
    pub fn new(label: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            label: label.into(),
            palette: palette.clone(),
            target: None,
            note: None,
            is_mono: false,
        }
    }

    /// Shows a target dot before the label.
    pub fn target(mut self, kind: TargetKind) -> Self {
        self.target = Some(kind);
        self
    }

    /// Shows a muted note after the label, such as "this computer".
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Draws the label in the mono font. Use it for paths.
    pub fn mono(mut self, is_mono: bool) -> Self {
        self.is_mono = is_mono;
        self
    }
}

impl<'a, Message: 'a> From<Chip> for Element<'a, Message> {
    fn from(value: Chip) -> Self {
        let palette = value.palette;
        let style = typography::SMALL;
        let background = palette.surface_raised();

        let mut content =
            row![].spacing(spacing::SPACE_2).align_y(Alignment::Center);
        if let Some(kind) = value.target {
            content = content.push(target_dot(kind, &palette));
        }
        content = content.push(
            text(value.label)
                .size(style.size())
                .line_height(iced::Pixels(style.line_height()))
                .font(if value.is_mono {
                    fonts::MONO
                } else {
                    fonts::UI
                })
                .wrapping(Wrapping::None)
                .color(palette.text_body()),
        );
        if let Some(note) = value.note {
            content = content.push(
                text(note)
                    .size(style.size())
                    .line_height(iced::Pixels(style.line_height()))
                    .font(fonts::UI)
                    .wrapping(Wrapping::None)
                    .color(palette.text_muted()),
            );
        }

        container(content)
            .height(HEIGHT)
            .align_y(Alignment::Center)
            .padding(Padding::from([0.0, PADDING_X]))
            .clip(true)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::column;
    use iced_test::simulator;

    use super::*;

    #[test]
    fn given_short_label_when_laid_out_then_chip_is_24px_high() {
        let palette = UiPalette::otty_dark();
        let chip: Element<'_, ()> = column![
            Chip::new("fedora", &palette).target(TargetKind::Local),
            text("below"),
        ]
        .into();
        let mut ui = simulator(chip);

        let below = ui.find("below").expect("marker should be visible");

        assert!(
            (below.bounds().y - HEIGHT).abs() < 0.5,
            "chip height {} should be {HEIGHT}",
            below.bounds().y
        );
    }

    #[test]
    fn given_narrow_space_when_label_is_long_then_chip_stays_one_line() {
        let palette = UiPalette::otty_dark();
        let chip: Element<'_, ()> = container(column![
            Chip::new("fedora work station", &palette)
                .target(TargetKind::Local)
                .note("this computer"),
            text("below"),
        ])
        .width(70)
        .into();
        let mut ui = simulator(chip);

        let label = ui
            .find("fedora work station")
            .expect("label should be visible")
            .bounds();
        let below =
            ui.find("below").expect("marker should be visible").bounds();

        assert!(
            (label.height - typography::SMALL.line_height()).abs() < 0.5,
            "label should stay on one line: {label:?}"
        );
        assert!(
            (below.y - HEIGHT).abs() < 0.5,
            "chip height {} should stay {HEIGHT}",
            below.y
        );
    }
}
