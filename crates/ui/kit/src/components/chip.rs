//! `Chip`: small label for a target, a path or an active filter.

use iced::widget::text::Wrapping;
use iced::widget::{container, row, text};
use iced::{Alignment, Border, Element, Padding};

use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use super::target_dot::{TargetKind, target_dot};
use super::wrapped_text::WrappedText;
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Horizontal padding of a chip (`padding: 0 8px`).
const PADDING_X: f32 = spacing::SPACE_3;
/// Height of a chip (`height: 24px`, the same as `control-sm`).
const HEIGHT: f32 = size::CONTROL_SM;

/// Chip with a label, an optional target dot, an optional muted note and an
/// optional clear button.
///
/// A chip is always one line [`HEIGHT`] high. In a space narrower than its
/// texts it does not wrap: its right end is cut off at the edge of its
/// background, so the texts never spill out of it. With [`Chip::wrap_within`]
/// the label wraps instead and the chip grows in height.
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
/// let filter: Element<'_, ()> = Chip::new("compose: web", &palette)
///     .on_clear("Show all", ())
///     .into();
/// ```
pub struct Chip<Message = ()> {
    label: String,
    palette: UiPalette,
    target: Option<TargetKind>,
    note: Option<String>,
    is_mono: bool,
    on_clear: Option<(String, Message)>,
    wrap_within: Option<f32>,
}

impl<Message> Chip<Message> {
    /// Creates a chip with `label` only.
    pub fn new(label: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            label: label.into(),
            palette: palette.clone(),
            target: None,
            note: None,
            is_mono: false,
            on_clear: None,
            wrap_within: None,
        }
    }

    /// Lets the label wrap instead of being cut, so the chip grows in height
    /// and is never wider than `max_width`. The label breaks after a space,
    /// `/`, `-`, `:` or `.` first. Meant for a label with an optional cross;
    /// a target dot or a note is not counted in the width left for the label.
    pub fn wrap_within(mut self, max_width: f32) -> Self {
        self.wrap_within = Some(max_width);
        self
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

    /// Adds a cross after the texts. A press on it sends `message`;
    /// `label` is its tooltip, for example "Show all containers".
    pub fn on_clear(
        mut self,
        label: impl Into<String>,
        message: Message,
    ) -> Self {
        self.on_clear = Some((label.into(), message));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Chip<Message>> for Element<'a, Message> {
    fn from(value: Chip<Message>) -> Self {
        let palette = value.palette;
        let style = typography::SMALL;
        let background = palette.surface_raised();

        if let Some(max_width) = value.wrap_within {
            return wrapped_chip(
                &value.label,
                value.is_mono,
                value.on_clear,
                max_width,
                &palette,
            );
        }

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

        if let Some((label, message)) = value.on_clear {
            content = content.push(
                IconButton::new(glyph::cross(), label, &palette)
                    .size(IconButtonSize::Small)
                    .hover_background(false)
                    .on_press(message),
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

/// Draws a chip whose label wraps: at least [`HEIGHT`] high, at most
/// `max_width` wide, with the cross in the middle of its height.
fn wrapped_chip<'a, Message: Clone + 'a>(
    label: &str,
    is_mono: bool,
    on_clear: Option<(String, Message)>,
    max_width: f32,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::SMALL;
    let background = palette.surface_raised();

    // What the label may use: the row minus the chip padding and the cross.
    let mut label_width = max_width - 2.0 * PADDING_X;
    if on_clear.is_some() {
        label_width -= spacing::SPACE_2 + HEIGHT;
    }

    let text = WrappedText::<Message>::new(
        vec![(label.to_owned(), None)],
        style.size(),
        if is_mono { fonts::MONO } else { fonts::UI },
        palette.text_body(),
        palette.text_body(),
    )
    .fit_text();
    // Centers one line in `HEIGHT`; more lines grow the chip.
    let text = container(text)
        .max_width(label_width.max(0.0))
        .padding(Padding::from([(HEIGHT - style.line_height()) / 2.0, 0.0]));

    let mut content = row![text]
        .spacing(spacing::SPACE_2)
        .align_y(Alignment::Center);
    if let Some((label, message)) = on_clear {
        content = content.push(
            IconButton::new(glyph::cross(), label, palette)
                .size(IconButtonSize::Small)
                .hover_background(false)
                .on_press(message),
        );
    }

    container(content)
        .max_width(max_width)
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

    #[test]
    fn given_wrap_when_label_is_long_then_chip_grows_within_max_width() {
        let palette = UiPalette::otty_dark();
        let chip: Element<'_, u8> =
            Chip::new("registry.example.com/team/image-name:1.2.3", &palette)
                .wrap_within(120.0)
                .on_clear("Remove", 1)
                .into();
        let mut ui = simulator(column![chip, text("below")]);

        let label = ui
            .find("registry.example.com/team/image-name:1.2.3")
            .expect("label should be visible")
            .bounds();
        let below =
            ui.find("below").expect("marker should be visible").bounds();

        assert!(below.y > HEIGHT + 8.0, "chip should grow: {}", below.y);
        assert!(
            label.x + label.width + spacing::SPACE_2 + HEIGHT + PADDING_X
                <= 120.5,
            "chip should fit 120px: {label:?}"
        );
    }

    #[test]
    fn given_wrap_when_label_is_short_then_chip_is_24px_high() {
        let palette = UiPalette::otty_dark();
        let chip: Element<'_, u8> = Chip::new("fedora", &palette)
            .wrap_within(300.0)
            .on_clear("Remove", 1)
            .into();
        let mut ui = simulator(column![chip, text("below")]);

        let below =
            ui.find("below").expect("marker should be visible").bounds();

        assert!((below.y - HEIGHT).abs() < 0.5, "height {}", below.y);
    }

    #[test]
    fn given_clear_button_when_pressed_then_chip_sends_its_message() {
        let palette = UiPalette::otty_dark();
        let chip: Element<'_, u8> = Chip::new("compose: web", &palette)
            .on_clear("Show all", 7)
            .into();
        let mut ui = simulator(chip);
        let label = ui.find("compose: web").expect("label should be visible");
        let bounds = label.bounds();

        // The cross sits one gap right of the label; its button is 24px.
        ui.point_at(iced::Point::new(
            bounds.x + bounds.width + spacing::SPACE_2 + 12.0,
            bounds.y + bounds.height / 2.0,
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<u8> = ui.into_messages().collect();
        assert_eq!(messages, vec![7]);
    }

    #[test]
    fn given_chip_without_clear_when_clicked_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let chip: Element<'_, u8> = Chip::new("fedora", &palette).into();
        let mut ui = simulator(chip);

        ui.click("fedora").expect("label should be visible");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
