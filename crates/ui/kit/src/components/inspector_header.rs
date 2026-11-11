//! `InspectorHeader`: title, status line and actions of the inspected item.

use iced::widget::text::Wrapping;
use iced::widget::{column, row, text};
use iced::{Element, Length};

use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// Size of the mono title (`font-size: 14px`).
const TITLE_SIZE: f32 = 14.0;

/// Head of the inspector: a mono title, a status line, a wrapping row of
/// small buttons and an optional hint under the buttons.
///
/// The status line and the buttons are elements the caller builds, such as a
/// status badge and kit [`Button`]s with the small size.
///
/// [`Button`]: super::button::Button
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::button::{Button, ButtonSize};
/// use otty_ui_kit::components::inspector_header::InspectorHeader;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Stop,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let header: Element<'_, Message> = InspectorHeader::new("web-1", &palette)
///     .status(text("Running"))
///     .action(
///         Button::new("Stop", &palette)
///             .size(ButtonSize::Small)
///             .on_press(Message::Stop),
///     )
///     .into();
/// ```
pub struct InspectorHeader<'a, Message> {
    title: String,
    palette: UiPalette,
    status: Option<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
    hint: Option<String>,
}

impl<'a, Message> InspectorHeader<'a, Message> {
    /// Creates a header with the item `title`.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            palette: palette.clone(),
            status: None,
            actions: Vec::new(),
            hint: None,
        }
    }

    /// Shows an element, such as a status badge, under the title.
    pub fn status(mut self, element: impl Into<Element<'a, Message>>) -> Self {
        self.status = Some(element.into());
        self
    }

    /// Adds a button after the previous ones; the row wraps when it is full.
    pub fn action(mut self, element: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(element.into());
        self
    }

    /// Shows a muted line under the buttons, such as why one is disabled.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl<'a, Message: 'a> From<InspectorHeader<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: InspectorHeader<'a, Message>) -> Self {
        let palette = value.palette;
        let hint_style = typography::SMALL;

        let mut head = column![
            text(value.title)
                .size(TITLE_SIZE)
                .font(fonts::MONO)
                .wrapping(Wrapping::Glyph)
                .color(palette.text_strong())
        ]
        .spacing(spacing::SPACE_1)
        .width(Length::Fill);
        if let Some(status) = value.status {
            head = head.push(status);
        }
        if !value.actions.is_empty() {
            head =
                head.push(row(value.actions).spacing(spacing::SPACE_2).wrap());
        }
        if let Some(hint) = value.hint {
            head = head.push(
                text(hint)
                    .size(hint_style.size())
                    .line_height(iced::Pixels(hint_style.line_height()))
                    .font(fonts::UI)
                    .color(palette.text_muted()),
            );
        }

        head.into()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::components::button::{Button, ButtonSize};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Start,
        Stop,
    }

    fn small_button(
        label: &str,
        message: Message,
        palette: &UiPalette,
    ) -> Button<Message> {
        Button::new(label, palette)
            .size(ButtonSize::Small)
            .on_press(message)
    }

    #[test]
    fn given_full_header_when_rendered_then_all_parts_are_shown() {
        let palette = UiPalette::otty_dark();
        let header: Element<'_, Message> =
            InspectorHeader::new("web-1", &palette)
                .status(text("Running"))
                .action(small_button("Stop", Message::Stop, &palette))
                .hint("Used by a container")
                .into();
        let mut ui = simulator(header);

        for label in ["web-1", "Running", "Stop", "Used by a container"] {
            assert!(ui.find(label).is_ok(), "{label} should be shown");
        }
    }

    #[test]
    fn given_two_buttons_when_one_pressed_then_only_its_message_is_sent() {
        let palette = UiPalette::otty_light();
        let header: Element<'_, Message> =
            InspectorHeader::new("web-1", &palette)
                .action(small_button("Start", Message::Start, &palette))
                .action(small_button("Stop", Message::Stop, &palette))
                .into();
        let mut ui = simulator(header);

        ui.click("Stop").expect("button should be shown");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Stop]);
    }

    #[test]
    fn given_narrow_width_when_buttons_do_not_fit_then_row_wraps() {
        let palette = UiPalette::otty_dark();
        let header: Element<'_, Message> = iced::widget::container(
            InspectorHeader::new("web-1", &palette)
                .action(small_button("Restart", Message::Start, &palette))
                .action(small_button("Remove", Message::Stop, &palette)),
        )
        .width(120)
        .into();
        let mut ui = simulator(header);

        let first = ui.find("Restart").expect("shown").bounds();
        let second = ui.find("Remove").expect("shown").bounds();

        assert!(
            second.y > first.y,
            "second button {second:?} should wrap below {first:?}"
        );
    }

    #[test]
    fn given_header_without_status_and_hint_when_rendered_then_title_is_shown()
    {
        let palette = UiPalette::otty_dark();
        let header: Element<'_, Message> =
            InspectorHeader::new("my-volume", &palette).into();
        let mut ui = simulator(header);

        assert!(ui.find("my-volume").is_ok());
    }
}
