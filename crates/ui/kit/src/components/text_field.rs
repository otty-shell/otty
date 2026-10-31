//! `TextField`: labelled single-line input with a hint or an error.

use iced::widget::text_input::TextInput;
use iced::widget::{container, row, text_input};
use iced::{Element, Length, Padding, Pixels};

use super::button::Button;
use super::field_chrome::{self, Note, input_style};
use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// Horizontal padding of the input (`padding: 0 11px`).
const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high.
const PADDING_Y: f32 = 7.0;

/// Single-line input with a label above and a note below. Without an input
/// handler the field is disabled.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::text_field::TextField;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Path(String),
///     Browse,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let field: Element<'_, Message> =
///     TextField::new("Working directory", "~/projects", &palette)
///         .optional(true)
///         .hint("Defaults to the home directory.")
///         .action("Browse", Message::Browse)
///         .on_input(Message::Path)
///         .into();
/// ```
pub struct TextField<'a, Message> {
    label: String,
    value: &'a str,
    palette: UiPalette,
    placeholder: String,
    hint: Option<String>,
    error: Option<String>,
    is_optional: bool,
    width: Length,
    action: Option<(String, Message)>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
}

impl<'a, Message> TextField<'a, Message> {
    /// Creates an empty-note, required, full-width field.
    pub fn new(
        label: impl Into<String>,
        value: &'a str,
        palette: &UiPalette,
    ) -> Self {
        Self {
            label: label.into(),
            value,
            palette: palette.clone(),
            placeholder: String::new(),
            hint: None,
            error: None,
            is_optional: false,
            width: Length::Fill,
            action: None,
            on_input: None,
            on_submit: None,
        }
    }

    /// Sets the text shown while the value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the line under the field that explains the value.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Sets the error. It replaces the hint and colors the border
    /// `status-error`.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Adds the `optional` marker to the label.
    pub fn optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }

    /// Sets the field width; use a fixed width for the short field of a pair
    /// such as Host and Port.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Adds a secondary button on the right of the input (for example
    /// Browse).
    pub fn action(
        mut self,
        label: impl Into<String>,
        message: Message,
    ) -> Self {
        self.action = Some((label.into(), message));
        self
    }

    /// Sets the function that maps the new value to a message and enables
    /// the field.
    pub fn on_input(
        mut self,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Sets the message sent when Enter is pressed.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<TextField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: TextField<'a, Message>) -> Self {
        let style = typography::TERM;
        let invalid = value.error.is_some();

        let control: TextInput<'a, Message> =
            text_input(&value.placeholder, value.value)
                .font(fonts::MONO)
                .size(style.size())
                .line_height(Pixels(style.line_height()))
                .padding(Padding::from([PADDING_Y, PADDING_X]))
                .on_input_maybe(value.on_input)
                .on_submit_maybe(value.on_submit)
                .style(input_style(&value.palette, invalid));

        let mut line = row![control].spacing(spacing::SPACE_2);
        if let Some((label, message)) = value.action {
            line =
                line.push(Button::new(label, &value.palette).on_press(message));
        }

        let note = match (value.error, value.hint) {
            (Some(error), _) => Some(Note::Error(error)),
            (None, Some(hint)) => Some(Note::Hint(hint)),
            (None, None) => None,
        };

        container(field_chrome::labelled(
            &value.label,
            value.is_optional,
            line.into(),
            note,
            &value.palette,
        ))
        .width(value.width)
        .into()
    }
}
