//! `TokenField`: list input for program arguments.
//!
//! Type a value and press Enter to turn it into a token; press Backspace on
//! an empty input to remove the last token. The add and remove rules live in
//! [`TokenList`]; [`TokenField`] only draws it and reports key presses.

use iced::keyboard::key::Named;
use iced::widget::{container, row, text, text_input};
use iced::{Alignment, Element, Length, Padding, Pixels};

use super::field_chrome::{self, Note};
use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use super::input_keys::InputKeys;
use crate::palette::UiPalette;
use crate::{fonts, radius, spacing};

/// Token height (`height: 24px`).
const TOKEN_HEIGHT: f32 = 24.0;
/// Left padding of a token (`padding: 0 2px 0 8px`).
const TOKEN_PADDING_LEFT: f32 = spacing::SPACE_3;
/// Token text size (`font-size: 12px`).
const TOKEN_TEXT_SIZE: f32 = 12.0;
/// Input text size (`font-size: 12.5px`).
const INPUT_TEXT_SIZE: f32 = 12.5;
/// Input width (`min-width: 140px`); `iced` cannot grow it to the free space
/// of a wrapped line.
const INPUT_WIDTH: f32 = 140.0;
/// Vertical padding of a 16px input line inside the 24px token row.
const INPUT_PADDING_Y: f32 = 4.0;
/// Line height of the input.
const INPUT_LINE: f32 = 16.0;
/// Padding of the box (`padding: 4px 6px` plus 1px so one line is 34px).
const BOX_PADDING: Padding = Padding {
    top: 5.0,
    right: spacing::SPACE_2,
    bottom: 5.0,
    left: spacing::SPACE_2,
};

/// Tokens already added and the text typed for the next one.
///
/// The app keeps one list per field and calls [`TokenList::submit`] and
/// [`TokenList::backspace`] when the field reports Enter and Backspace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenList {
    tokens: Vec<String>,
    input: String,
}

impl TokenList {
    /// Returns the tokens in the order they were added.
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }

    /// Returns the text typed for the next token.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Creates a list with `tokens` and an empty input.
    pub fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens,
            input: String::new(),
        }
    }

    /// Replaces the typed text.
    pub fn set_input(&mut self, input: String) {
        self.input = input;
    }

    /// Enter: turns the typed text into a token and clears the input.
    /// Empty or blank text adds nothing. Returns whether a token was added.
    pub fn submit(&mut self) -> bool {
        let token = self.input.trim();
        if token.is_empty() {
            return false;
        }

        self.tokens.push(token.to_owned());
        self.input.clear();

        true
    }

    /// Backspace: removes the last token when the input is empty. Returns
    /// whether a token was removed.
    pub fn backspace(&mut self) -> bool {
        if !self.input.is_empty() {
            return false;
        }

        self.tokens.pop().is_some()
    }

    /// Cross on a token: removes the token at `index`. Returns whether the
    /// index existed.
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.tokens.len() {
            return false;
        }

        self.tokens.remove(index);

        true
    }
}

/// Token box: tokens with a cross each, then the input, wrapping to new
/// lines. Handlers that are not set leave the field disabled.
///
/// Backspace on an empty input is detected by a small wrapper around the
/// input, because `iced` text inputs send nothing in that case.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::token_field::{TokenField, TokenList};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Input(String),
///     Submit,
///     Backspace,
///     Remove(usize),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let list = TokenList::new(vec!["--color".to_owned()]);
/// let field: Element<'_, Message> =
///     TokenField::new("Arguments", &list, &palette)
///         .on_input(Message::Input)
///         .on_submit(Message::Submit)
///         .on_backspace_empty(Message::Backspace)
///         .on_remove(Message::Remove)
///         .into();
/// ```
pub struct TokenField<'a, Message> {
    label: String,
    list: &'a TokenList,
    palette: UiPalette,
    placeholder: String,
    hint: Option<String>,
    is_optional: bool,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    on_backspace_empty: Option<Message>,
    on_remove: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

impl<'a, Message> TokenField<'a, Message> {
    /// Creates a field that shows `list`.
    pub fn new(
        label: impl Into<String>,
        list: &'a TokenList,
        palette: &UiPalette,
    ) -> Self {
        Self {
            label: label.into(),
            list,
            palette: palette.clone(),
            placeholder: String::new(),
            hint: None,
            is_optional: false,
            on_input: None,
            on_submit: None,
            on_backspace_empty: None,
            on_remove: None,
        }
    }

    /// Sets the text shown while the input is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
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

    /// Sets the function that maps the typed text to a message.
    pub fn on_input(
        mut self,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Sets the message sent on Enter; the app calls
    /// [`TokenList::submit`].
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// Sets the message sent on Backspace in an empty focused input; the
    /// app calls [`TokenList::backspace`].
    pub fn on_backspace_empty(mut self, message: Message) -> Self {
        self.on_backspace_empty = Some(message);
        self
    }

    /// Sets the function that maps a token index to the message of its
    /// cross; the app calls [`TokenList::remove`].
    pub fn on_remove(
        mut self,
        on_remove: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        self.on_remove = Some(Box::new(on_remove));
        self
    }
}

impl<'a, Message: Clone + 'a> From<TokenField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: TokenField<'a, Message>) -> Self {
        let palette = value.palette;

        let mut line =
            row![].spacing(spacing::SPACE_2).align_y(Alignment::Center);
        for (index, token) in value.list.tokens().iter().enumerate() {
            let mut cross = IconButton::new(glyph::cross(), "Remove", &palette)
                .size(IconButtonSize::Small);
            if let Some(on_remove) = &value.on_remove {
                cross = cross.on_press(on_remove(index));
            }
            line = line.push(token_view(token, cross, &palette));
        }

        let input_color = palette.text_strong();
        let muted = palette.text_muted();
        let accent = palette.accent();
        let is_enabled = value.on_input.is_some();
        let input = text_input(&value.placeholder, value.list.input())
            .font(fonts::MONO)
            .size(INPUT_TEXT_SIZE)
            .line_height(Pixels(INPUT_LINE))
            .padding(Padding::from([INPUT_PADDING_Y, 0.0]))
            .width(INPUT_WIDTH)
            .on_input_maybe(value.on_input)
            .on_submit_maybe(value.on_submit)
            .style(move |_, status| text_input::Style {
                background: iced::Color::TRANSPARENT.into(),
                border: iced::Border::default(),
                icon: muted,
                placeholder: muted,
                value: if is_enabled || status != text_input::Status::Disabled {
                    input_color
                } else {
                    muted
                },
                selection: iced::Color { a: 0.35, ..accent },
            });
        let mut keys = InputKeys::new(input);
        if let Some(message) = value.on_backspace_empty
            && value.list.input().is_empty()
        {
            keys = keys.on_key(Named::Backspace, message);
        }
        line = line.push(keys);

        let background = palette.surface_raised();
        let frame = container(line.wrap())
            .padding(BOX_PADDING)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: iced::Border {
                    radius: radius::RADIUS_SM.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            });

        field_chrome::labelled(
            &value.label,
            value.is_optional,
            frame.into(),
            value.hint.map(Note::Hint),
            &palette,
        )
    }
}

/// Builds one token chip with its cross.
fn token_view<'a, Message: Clone + 'a>(
    token: &str,
    cross: IconButton<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let background = palette.surface_hover();
    let color = palette.text_strong();

    container(
        row![
            text(token.to_owned())
                .size(TOKEN_TEXT_SIZE)
                .font(fonts::MONO)
                .color(color),
            cross,
        ]
        .align_y(Alignment::Center),
    )
    .height(TOKEN_HEIGHT)
    .padding(Padding {
        left: TOKEN_PADDING_LEFT,
        ..Padding::ZERO
    })
    .align_y(Alignment::Center)
    .style(move |_| container::Style {
        background: Some(background.into()),
        border: iced::Border {
            radius: radius::RADIUS_XS.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(tokens: &[&str], input: &str) -> TokenList {
        let mut list =
            TokenList::new(tokens.iter().map(|t| (*t).to_owned()).collect());
        list.set_input(input.to_owned());
        list
    }

    #[test]
    fn enter_with_text_adds_one_token_and_clears_input() {
        let mut list = list(&["-v"], "--color");

        assert!(list.submit());

        assert_eq!(list.tokens(), ["-v", "--color"]);
        assert_eq!(list.input(), "");
    }

    #[test]
    fn enter_trims_surrounding_whitespace() {
        let mut list = list(&[], "  --flag \t");

        assert!(list.submit());

        assert_eq!(list.tokens(), ["--flag"]);
    }

    #[test]
    fn enter_with_empty_input_adds_nothing() {
        let mut list = list(&["-v"], "");

        assert!(!list.submit());

        assert_eq!(list.tokens(), ["-v"]);
    }

    #[test]
    fn enter_with_whitespace_only_adds_nothing() {
        let mut list = list(&["-v"], "   ");

        assert!(!list.submit());

        assert_eq!(list.tokens(), ["-v"]);
        assert_eq!(list.input(), "   ");
    }

    #[test]
    fn backspace_on_empty_input_removes_last_token() {
        let mut list = list(&["a", "b"], "");

        assert!(list.backspace());

        assert_eq!(list.tokens(), ["a"]);
    }

    #[test]
    fn backspace_on_empty_input_and_no_tokens_does_nothing() {
        let mut list = list(&[], "");

        assert!(!list.backspace());

        assert!(list.tokens().is_empty());
    }

    #[test]
    fn backspace_with_text_does_nothing() {
        let mut list = list(&["a", "b"], "x");

        assert!(!list.backspace());

        assert_eq!(list.tokens(), ["a", "b"]);
        assert_eq!(list.input(), "x");
    }

    #[test]
    fn remove_drops_token_at_index() {
        let mut list = list(&["a", "b", "c"], "");

        assert!(list.remove(1));
        assert!(!list.remove(5));

        assert_eq!(list.tokens(), ["a", "c"]);
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Input(String),
        Backspace,
    }

    /// Clicks the field's input (found by its placeholder value), then
    /// presses Backspace.
    fn press_backspace(list: &TokenList) -> Vec<Message> {
        use iced_test::simulator;

        let palette = UiPalette::otty_dark();
        let field = TokenField::new("Arguments", list, &palette)
            .on_input(Message::Input)
            .on_backspace_empty(Message::Backspace);
        let mut ui = simulator(field);

        ui.click(list.input()).expect("input should be visible");
        ui.tap_key(Named::Backspace);

        ui.into_messages().collect()
    }

    #[test]
    fn given_empty_focused_input_when_backspace_pressed_then_it_is_reported() {
        let list = list(&["a"], "");

        assert_eq!(press_backspace(&list), vec![Message::Backspace]);
    }

    #[test]
    fn given_typed_text_when_backspace_pressed_then_input_edits_it() {
        let list = list(&["a"], "xy");

        assert_eq!(
            press_backspace(&list),
            vec![Message::Input("x".to_owned())]
        );
    }
}
