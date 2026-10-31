//! `KeyValueField`: environment variables or labels as name and value rows.

use std::rc::Rc;

use iced::widget::{column, container, row, text_input};
use iced::{Alignment, Element, Length, Padding, Pixels};

use super::button::{Button, ButtonKind, ButtonSize};
use super::field_chrome::{self, Note, input_style};
use super::glyph;
use super::icon_button::IconButton;
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Horizontal padding of an input (`padding: 0 11px`).
const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high.
const PADDING_Y: f32 = 7.0;
/// Share of the name column (`minmax(0, 1fr)`).
const NAME_PORTION: u16 = 2;
/// Share of the value column (`minmax(0, 1.5fr)`).
const VALUE_PORTION: u16 = 3;

/// Rows of name and value inputs, a cross on each row and an add button.
/// Handlers that are not set leave the matching control disabled.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::key_value_field::KeyValueField;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Name(usize, String),
///     Value(usize, String),
///     Remove(usize),
///     Add,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let rows = [("RUST_LOG".to_owned(), "debug".to_owned())];
/// let field: Element<'_, Message> =
///     KeyValueField::new("Environment variables", &rows, &palette)
///         .on_name(Message::Name)
///         .on_value(Message::Value)
///         .on_remove(Message::Remove)
///         .on_add(Message::Add)
///         .into();
/// ```
pub struct KeyValueField<'a, Message> {
    label: String,
    rows: &'a [(String, String)],
    palette: UiPalette,
    hint: Option<String>,
    is_optional: bool,
    add_label: String,
    on_name: Option<Rc<dyn Fn(usize, String) -> Message + 'a>>,
    on_value: Option<Rc<dyn Fn(usize, String) -> Message + 'a>>,
    on_remove: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_add: Option<Message>,
}

impl<'a, Message> KeyValueField<'a, Message> {
    /// Creates a field over `rows` of `(name, value)`; the add button reads
    /// "Add variable".
    pub fn new(
        label: impl Into<String>,
        rows: &'a [(String, String)],
        palette: &UiPalette,
    ) -> Self {
        Self {
            label: label.into(),
            rows,
            palette: palette.clone(),
            hint: None,
            is_optional: false,
            add_label: "Add variable".to_owned(),
            on_name: None,
            on_value: None,
            on_remove: None,
            on_add: None,
        }
    }

    /// Sets the line under the field.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Adds the `optional` marker to the label.
    pub fn optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }

    /// Sets the text of the add button.
    pub fn add_label(mut self, label: impl Into<String>) -> Self {
        self.add_label = label.into();
        self
    }

    /// Sets the function that maps `(row, new name)` to a message.
    pub fn on_name(
        mut self,
        on_name: impl Fn(usize, String) -> Message + 'a,
    ) -> Self {
        self.on_name = Some(Rc::new(on_name));
        self
    }

    /// Sets the function that maps `(row, new value)` to a message.
    pub fn on_value(
        mut self,
        on_value: impl Fn(usize, String) -> Message + 'a,
    ) -> Self {
        self.on_value = Some(Rc::new(on_value));
        self
    }

    /// Sets the function that maps a row to the message of its cross.
    pub fn on_remove(
        mut self,
        on_remove: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        self.on_remove = Some(Box::new(on_remove));
        self
    }

    /// Sets the message sent by the add button.
    pub fn on_add(mut self, message: Message) -> Self {
        self.on_add = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<KeyValueField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: KeyValueField<'a, Message>) -> Self {
        let palette = value.palette;

        let mut rows = column![].spacing(spacing::SPACE_2);
        for (index, (name, content)) in value.rows.iter().enumerate() {
            let name_input = mono_input("NAME", name, &palette).on_input_maybe(
                value
                    .on_name
                    .clone()
                    .map(|on_name| move |text| on_name(index, text)),
            );
            let value_input = mono_input("value", content, &palette)
                .on_input_maybe(
                    value
                        .on_value
                        .clone()
                        .map(|on_value| move |text| on_value(index, text)),
                );
            let mut cross =
                IconButton::new(glyph::cross(), "Remove variable", &palette);
            if let Some(on_remove) = &value.on_remove {
                cross = cross.on_press(on_remove(index));
            }

            rows = rows.push(
                row![
                    name_input.width(Length::FillPortion(NAME_PORTION)),
                    value_input.width(Length::FillPortion(VALUE_PORTION)),
                    container(cross).width(size::CONTROL_MD),
                ]
                .spacing(spacing::SPACE_2)
                .align_y(Alignment::Center),
            );
        }

        let mut add = Button::new(value.add_label, &palette)
            .kind(ButtonKind::Secondary)
            .size(ButtonSize::Small);
        if let Some(message) = value.on_add {
            add = add.on_press(message);
        }
        let body = column![rows, container(add).padding(Padding::ZERO)]
            .spacing(spacing::SPACE_2);

        field_chrome::labelled(
            &value.label,
            value.is_optional,
            body.into(),
            value.hint.map(Note::Hint),
            &palette,
        )
    }
}

/// Builds a boxed mono input of control-lg height.
fn mono_input<'a, Message: Clone + 'a>(
    placeholder: &str,
    content: &str,
    palette: &UiPalette,
) -> text_input::TextInput<'a, Message> {
    let style = typography::TERM;

    text_input(placeholder, content)
        .font(fonts::MONO)
        .size(style.size())
        .line_height(Pixels(style.line_height()))
        .padding(Padding::from([PADDING_Y, PADDING_X]))
        .style(input_style(palette, false))
}
