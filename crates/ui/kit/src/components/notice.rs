//! `Notice`: inline error message with a dismiss button.

use iced::widget::{button, container, row, svg, text, tooltip};
use iced::{Alignment, Border, Element, Length, Padding};

use super::glyph;
use super::icon_button::tooltip_hint;
use crate::palette::UiPalette;
use crate::{fonts, radius, spacing};

/// Gap between icon, text and dismiss button (`.ot-banner { gap: 10px }`).
const GAP: f32 = 10.0;
/// Padding of the notice (`.ot-banner { padding: 9px 12px }`).
const PADDING: Padding = Padding {
    top: 9.0,
    right: spacing::SPACE_4,
    bottom: 9.0,
    left: spacing::SPACE_4,
};
/// Text size (`.ot-banner { font-size: 12.5px }`).
const TEXT_SIZE: f32 = 12.5;
/// Edge of the leading icon. The design says 15px; 16 keeps the icon on
/// whole pixels at every scale, as in [`IconButton`].
///
/// [`IconButton`]: super::icon_button::IconButton
const ICON_SIZE: f32 = 16.0;
/// Edge of the dismiss button (`.clear-x { width: 16px; height: 16px }`).
const DISMISS_SIZE: f32 = 16.0;
/// Edge of the cross in the dismiss button. The design says 11px; 12 is an
/// even number of pixels and keeps the cross sharp.
const CROSS_SIZE: f32 = 12.0;

/// Inline error message: an error icon, the text and a dismiss cross on a
/// `surface-danger` background. Long text wraps.
///
/// Only the error tone exists today; it is what a failed action shows.
/// Components never bundle icons, so the caller passes the error icon.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::notice::Notice;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Dismiss,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let notice: Element<'_, Message> =
///     Notice::new("Container is already stopped.", icon, &palette)
///         .on_dismiss(Message::Dismiss, "Dismiss")
///         .into();
/// ```
pub struct Notice<Message> {
    text: String,
    icon: svg::Handle,
    palette: UiPalette,
    on_dismiss: Option<(Message, String)>,
}

impl<Message> Notice<Message> {
    /// Creates a notice with `text` and the error `icon`, without a dismiss
    /// button.
    pub fn new(
        text: impl Into<String>,
        icon: svg::Handle,
        palette: &UiPalette,
    ) -> Self {
        Self {
            text: text.into(),
            icon,
            palette: palette.clone(),
            on_dismiss: None,
        }
    }

    /// Shows a dismiss cross that sends `message`. `label` is the cross's
    /// tooltip, such as "Dismiss", in the app's language.
    pub fn on_dismiss(
        mut self,
        message: Message,
        label: impl Into<String>,
    ) -> Self {
        self.on_dismiss = Some((message, label.into()));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Notice<Message>> for Element<'a, Message> {
    fn from(value: Notice<Message>) -> Self {
        let palette = value.palette;
        let background = palette.surface_danger();
        let color = palette.text_danger();
        let icon_color = palette.status_error();

        let mut content = row![
            svg(value.icon).width(ICON_SIZE).height(ICON_SIZE).style(
                move |_, _| svg::Style {
                    color: Some(icon_color)
                }
            ),
            text(value.text)
                .size(TEXT_SIZE)
                .font(fonts::UI)
                .color(color)
                .width(Length::Fill),
        ]
        .spacing(GAP)
        .align_y(Alignment::Center);
        if let Some((message, label)) = value.on_dismiss {
            content = content.push(dismiss_button(message, label, &palette));
        }

        container(content)
            .width(Length::Fill)
            .padding(PADDING)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_MD.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}

/// Draws the small dismiss cross: `text-danger`, turning `text-strong` on a
/// `surface-hover` square while hovered (`.clear-x`).
fn dismiss_button<'a, Message: Clone + 'a>(
    message: Message,
    label: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let idle = palette.text_danger();
    let hover = palette.text_strong();
    let hover_background = palette.surface_hover();

    let cross = svg(glyph::cross())
        .width(CROSS_SIZE)
        .height(CROSS_SIZE)
        .style(move |_, status| svg::Style {
            color: Some(match status {
                svg::Status::Hovered => hover,
                _ => idle,
            }),
        });

    let control = button(container(cross).center(Length::Fill))
        .width(DISMISS_SIZE)
        .height(DISMISS_SIZE)
        .padding(0)
        .on_press(message)
        .style(move |_, status| {
            let is_active = matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            );

            button::Style {
                background: is_active.then(|| hover_background.into()),
                border: Border {
                    radius: radius::RADIUS_XS.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        });

    tooltip(
        control,
        tooltip_hint(label, palette),
        tooltip::Position::Bottom,
    )
    .gap(spacing::SPACE_1)
    .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;
    use crate::contrast::contrast_ratio;

    /// WCAG 2.1 AA minimum for normal text.
    const TEXT_MIN: f32 = 4.5;
    /// WCAG 2.1 AA minimum for non-text marks.
    const NON_TEXT_MIN: f32 = 3.0;
    /// Width of the notice in the tests.
    const WIDTH: f32 = 400.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Dismiss,
    }

    fn notice_view(text: &str) -> Element<'_, Message> {
        let palette = UiPalette::otty_dark();

        container(
            Notice::new(text, glyph::plus(), &palette)
                .on_dismiss(Message::Dismiss, "Dismiss"),
        )
        .width(WIDTH)
        .into()
    }

    #[test]
    fn given_notice_when_dismiss_pressed_then_dismiss_is_sent() {
        let mut ui = simulator(notice_view("Port 8080 is already in use."));

        // The cross sits in the last 16px before the 12px right padding,
        // centered in a one-line notice: 9 + 16 / 2 from the top.
        ui.point_at(Point::new(WIDTH - 12.0 - 8.0, 9.0 + 8.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Dismiss]);
    }

    #[test]
    fn given_notice_when_text_pressed_then_nothing_is_sent() {
        let mut ui = simulator(notice_view("Port 8080 is already in use."));

        ui.click("Port 8080 is already in use.")
            .expect("text should be visible");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_each_theme_when_notice_drawn_then_text_and_icon_are_readable() {
        for (theme, palette) in [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ] {
            let text =
                contrast_ratio(palette.text_danger(), palette.surface_danger());
            let icon = contrast_ratio(
                palette.status_error(),
                palette.surface_danger(),
            );

            assert!(text >= TEXT_MIN, "{theme}: text is {text:.2}:1");
            assert!(icon >= NON_TEXT_MIN, "{theme}: icon is {icon:.2}:1");
        }
    }
}
