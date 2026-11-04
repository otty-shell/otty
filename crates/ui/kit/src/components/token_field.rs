//! `TokenField`: list input for program arguments.
//!
//! Type a value and press Enter to turn it into a token; press Backspace on
//! an empty input to remove the last token. The add and remove rules live in
//! [`TokenList`]; [`TokenField`] only draws it and reports key presses.

use iced::advanced::text::{self as core_text, Paragraph as _};
use iced::alignment::Vertical;
use iced::keyboard::key::Named;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{container, row, text, text_input};
use iced::{Alignment, Element, Length, Padding, Pixels, Renderer, Size};

use super::field_chrome::{self, Note};
use super::icon_button::{IconButton, IconButtonSize};
use super::input_keys::InputKeys;
use super::{glyph, text_field};
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
/// Smallest input width (`min-width: 140px`); `iced` cannot grow it to the
/// free space of a wrapped line, so a longer placeholder sets the width.
const INPUT_WIDTH: f32 = 140.0;
/// Vertical padding of a 16px input line inside the 24px token row.
const INPUT_PADDING_Y: f32 = 4.0;
/// Line height of the input.
const INPUT_LINE: f32 = 16.0;
/// Room left of the input's text, so it starts where the text of a
/// [`TextField`] does, also when it starts a line after tokens.
///
/// [`TextField`]: super::text_field::TextField
const INPUT_PADDING_LEFT: f32 = text_field::PADDING_X - BOX_PADDING.left;
/// Padding of the box (`padding: 4px 6px` plus 1px so one line is 34px).
const BOX_PADDING: Padding = Padding {
    top: 5.0,
    right: spacing::SPACE_2,
    bottom: 5.0,
    left: spacing::SPACE_2,
};

/// Paragraph type of the renderer, used to measure the placeholder.
type Paragraph = <Renderer as core_text::Renderer>::Paragraph;

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
///         .remove_label("Remove")
///         .into();
/// ```
pub struct TokenField<'a, Message> {
    label: String,
    list: &'a TokenList,
    palette: UiPalette,
    placeholder: String,
    hint: Option<String>,
    optional: Option<String>,
    remove_label: String,
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
            optional: None,
            remove_label: String::new(),
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

    /// Marks the field as one that may stay empty: `marker` shows beside the
    /// label, in the language of the app.
    pub fn optional(mut self, marker: impl Into<String>) -> Self {
        self.optional = Some(marker.into());
        self
    }

    /// Sets the tooltip of the cross on each token.
    pub fn remove_label(mut self, label: impl Into<String>) -> Self {
        self.remove_label = label.into();
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
            let mut cross = IconButton::new(
                glyph::cross(),
                value.remove_label.as_str(),
                &palette,
            )
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
            .padding(Padding {
                top: INPUT_PADDING_Y,
                right: 0.0,
                bottom: INPUT_PADDING_Y,
                left: INPUT_PADDING_LEFT,
            })
            .width(input_width(&value.placeholder))
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
            value.optional.as_deref(),
            frame.into(),
            value.hint.map(Note::Hint),
            &palette,
        )
    }
}

/// Returns the width of the input: at least enough for its padding and all of
/// `placeholder` on one line in the input's font. A line without that much room
/// wraps.
fn input_width(placeholder: &str) -> f32 {
    let paragraph = Paragraph::with_text(core_text::Text {
        content: placeholder,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(INPUT_TEXT_SIZE),
        line_height: LineHeight::Absolute(Pixels(INPUT_LINE)),
        font: fonts::MONO,
        align_x: core_text::Alignment::Default,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    });

    let needed = INPUT_PADDING_LEFT + paragraph.min_bounds().width;

    INPUT_WIDTH.max(needed.ceil())
}

/// Builds one token chip with its cross.
fn token_view<'a, Message: Clone + 'a>(
    token: &str,
    cross: IconButton<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let background = palette.surface_chip();
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
    use crate::size;

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

    /// Text typed into the input, to find it by.
    const TYPED: &str = "typed";
    /// Longer than the smallest input in every font.
    const LONG_PLACEHOLDER: &str = "Type a word and then press the Enter key";

    /// Width of `placeholder` in the font of the input.
    fn text_width(placeholder: &str) -> f32 {
        let shown: Element<'_, Message> = text(placeholder.to_owned())
            .font(fonts::MONO)
            .size(INPUT_TEXT_SIZE)
            .shaping(Shaping::Advanced)
            .into();

        iced_test::simulator(shown)
            .find(placeholder)
            .expect("the text should be visible")
            .bounds()
            .width
    }

    /// Bounds of the input in a field `width` wide, and of the first token.
    fn drawn(
        tokens: &[&str],
        placeholder: &str,
        width: f32,
    ) -> (iced::Rectangle, Option<iced::Rectangle>) {
        let palette = UiPalette::otty_dark();
        let list = list(tokens, TYPED);
        let field = TokenField::new("Command", &list, &palette)
            .placeholder(placeholder)
            .on_input(Message::Input);
        let mut ui = iced_test::simulator(container(field).width(width));

        let input = ui.find(TYPED).expect("the input").bounds();
        let token = tokens
            .first()
            .map(|token| ui.find(*token).expect("the token").bounds());

        (input, token)
    }

    #[test]
    fn given_a_long_placeholder_when_drawn_then_the_input_holds_all_of_it() {
        let (input, _) = drawn(&[], LONG_PLACEHOLDER, 600.0);

        assert!(
            input.width - INPUT_PADDING_LEFT >= text_width(LONG_PLACEHOLDER),
            "the input is {} wide, the text {}",
            input.width,
            text_width(LONG_PLACEHOLDER)
        );
    }

    /// Left edge of the input's text in a field that starts at the left of the
    /// window.
    fn text_start(input: iced::Rectangle) -> f32 {
        input.x + INPUT_PADDING_LEFT
    }

    #[test]
    fn given_no_tokens_when_drawn_then_the_text_starts_where_a_text_fields_does()
     {
        let (input, _) = drawn(&[], "-A", 600.0);

        assert_eq!(text_start(input), text_field::PADDING_X);
    }

    #[test]
    fn given_a_token_above_when_drawn_then_the_text_starts_where_a_text_fields_does()
     {
        let width = text_width(LONG_PLACEHOLDER) + 2.0 * text_field::PADDING_X;

        let (input, token) = drawn(&["--color"], LONG_PLACEHOLDER, width);

        assert!(input.y > token.expect("the token").y, "{input:?}");
        assert_eq!(text_start(input), text_field::PADDING_X);
    }

    #[test]
    fn given_a_token_beside_when_drawn_then_the_text_follows_it_on_its_line() {
        let (input, token) = drawn(&["--color"], "-A", 600.0);
        let token = token.expect("the token");

        assert!(input.y < token.y + token.height, "{input:?} and {token:?}");
        assert!(text_start(input) > token.x + token.width + spacing::SPACE_2);
    }

    #[test]
    fn given_any_tokens_when_drawn_then_one_line_keeps_its_height() {
        let palette = UiPalette::otty_dark();
        let height = |tokens: &[&str]| {
            let list = list(tokens, TYPED);
            let field =
                TokenField::new("", &list, &palette).on_input(Message::Input);
            let mut ui = iced_test::simulator(container(field).width(600));
            let input = ui.find(TYPED).expect("the input").bounds();

            (input.height, input.y)
        };

        assert_eq!(height(&[]), (INPUT_LINE + 2.0 * INPUT_PADDING_Y, 5.0));
        assert_eq!(height(&["-v"]).0, INPUT_LINE + 2.0 * INPUT_PADDING_Y);
    }

    /// Says whether a token is drawn as saved in `reference`, with the tooltip
    /// `remove_label` and the pointer on its cross; the first call saves it.
    /// iced draws tooltips as overlays that widget queries do not reach, so
    /// they are checked on the drawing.
    fn hovered_token_matches(
        remove_label: &str,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let list = list(&["--color"], "");
        let field = TokenField::new("", &list, &palette)
            .remove_label(remove_label)
            .on_remove(|_| Message::Backspace);
        let mut ui = iced_test::simulator(container(field).width(300));
        let token = ui.find("--color").expect("the token").bounds();
        let position = iced::Point::new(
            token.x + token.width + size::CONTROL_SM / 2.0,
            token.y + token.height / 2.0,
        );
        ui.point_at(position);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position },
        )]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("the field should draw")
            .matches_hash(reference)
            .expect("the reference should be readable")
    }

    #[test]
    fn the_cross_of_a_token_shows_the_label_the_app_gives() {
        let folder = crate::test_support::TempDir::new("token-cross");
        let reference = folder.path().join("cross");

        assert!(hovered_token_matches("Remove", &reference));
        assert!(!hovered_token_matches("Entfernen", &reference));
    }

    /// Says whether a token is drawn as saved in `reference` in `palette`.
    fn drawn_token_matches(
        palette: &UiPalette,
        reference: &std::path::Path,
    ) -> bool {
        let list = list(&["--color"], "");
        let field = TokenField::<Message>::new("", &list, palette);

        iced_test::simulator(container(field).width(300))
            .snapshot(&iced::Theme::Dark)
            .expect("the field should draw")
            .matches_hash(reference)
            .expect("the reference should be readable")
    }

    #[test]
    fn a_token_is_drawn_in_the_chip_color_of_the_palette() {
        let folder = crate::test_support::TempDir::new("token-chip");
        let reference = folder.path().join("chip");
        let palette = UiPalette::otty_light();

        assert!(drawn_token_matches(&palette, &reference));
        assert!(!drawn_token_matches(
            &palette
                .clone()
                .with_surface_chip_for_test(iced::Color::BLACK),
            &reference
        ));
    }

    #[test]
    fn given_a_short_placeholder_when_drawn_then_the_input_keeps_its_smallest_width()
     {
        let (input, _) = drawn(&[], "-A", 600.0);

        assert_eq!(input.width, INPUT_WIDTH);
    }

    #[test]
    fn given_a_token_and_no_room_beside_it_when_drawn_then_the_whole_input_moves_down()
     {
        let width = text_width(LONG_PLACEHOLDER) + 2.0 * spacing::SPACE_2;

        let (input, token) = drawn(&["--color"], LONG_PLACEHOLDER, width);

        let token = token.expect("the token");
        assert!(input.y >= token.y + token.height, "{input:?} and {token:?}");
        assert!(input.width >= text_width(LONG_PLACEHOLDER), "{input:?}");
    }

    #[test]
    fn given_an_optional_marker_when_drawn_then_it_reads_as_given() {
        let palette = UiPalette::otty_dark();
        let list = list(&[], "");
        let field = TokenField::new("Команда", &list, &palette)
            .optional("необязательно")
            .on_input(Message::Input);
        let mut ui = iced_test::simulator(field);

        assert!(ui.find("необязательно").is_ok());
        assert!(ui.find("optional").is_err());
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
