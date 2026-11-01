//! `CommandBox`: a shell command in a mono font with a copy button.

use iced::widget::text::Wrapping;
use iced::widget::{container, row, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::button::{Button, ButtonSize};
use crate::palette::UiPalette;
use crate::{fonts, radius, spacing};

/// Size of the command text (`font-size: 12.5px`).
const COMMAND_SIZE: f32 = 12.5;
/// Left padding of the box (`padding: 6px 6px 6px 12px`).
const PADDING_LEFT: f32 = 12.0;
/// Other paddings of the box.
const PADDING_OTHER: f32 = 6.0;

/// A command the user can read and copy. The box shows the command on one
/// line and a small button on its right; the button sends the message set
/// with [`CommandBox::on_copy`]. The caller writes the command to the
/// clipboard.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::command_box::CommandBox;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Copy,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let command: Element<'_, Message> =
///     CommandBox::new("sudo systemctl start docker", "Copy", &palette)
///         .on_copy(Message::Copy)
///         .into();
/// ```
pub struct CommandBox<Message> {
    command: String,
    copy_label: String,
    palette: UiPalette,
    on_copy: Option<Message>,
}

impl<Message> CommandBox<Message> {
    /// Creates a box for `command` with a button labeled `copy_label`.
    pub fn new(
        command: impl Into<String>,
        copy_label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            command: command.into(),
            copy_label: copy_label.into(),
            palette: palette.clone(),
            on_copy: None,
        }
    }

    /// Sets the message sent when the copy button is pressed.
    pub fn on_copy(mut self, message: Message) -> Self {
        self.on_copy = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<CommandBox<Message>>
    for Element<'a, Message>
{
    fn from(value: CommandBox<Message>) -> Self {
        let palette = value.palette;
        let background = palette.surface_chrome();

        let command = text(value.command)
            .size(COMMAND_SIZE)
            .font(fonts::MONO)
            .color(palette.text_strong())
            .wrapping(Wrapping::None)
            .width(Length::Fill);
        let mut copy =
            Button::new(value.copy_label, &palette).size(ButtonSize::Small);
        if let Some(message) = value.on_copy {
            copy = copy.on_press(message);
        }

        container(
            row![command, copy]
                .spacing(spacing::SPACE_2)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding {
            top: PADDING_OTHER,
            right: PADDING_OTHER,
            bottom: PADDING_OTHER,
            left: PADDING_LEFT,
        })
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
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Copy,
    }

    #[test]
    fn given_command_box_then_the_command_is_shown() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(CommandBox::<Message>::new(
            "sudo systemctl start docker",
            "Copy",
            &palette,
        ));

        assert!(ui.find("sudo systemctl start docker").is_ok());
    }

    #[test]
    fn given_command_box_when_copy_pressed_then_its_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            CommandBox::new("ls", "Copy", &palette).on_copy(Message::Copy),
        );

        ui.click("Copy").expect("copy button should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Copy]);
    }
}
