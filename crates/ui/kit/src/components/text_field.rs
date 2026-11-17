//! `TextField`: labelled single-line input with a hint or an error.

use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::text_input::TextInput;
use iced::widget::{container, row, text, text_input};
use iced::{Element, Length, Padding, Pixels};

use super::button::Button;
use super::field_chrome::{self, Note, input_style};
use super::input_keys::InputKeys;
use super::suggestion_input::{ROW_TEXT_SIZE, SuggestionInput};
use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// Horizontal padding of the input (`padding: 0 11px`).
pub(super) const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high.
pub(super) const PADDING_Y: f32 = 7.0;

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
///         .optional("optional")
///         .hint("Defaults to the home directory.")
///         .action("Browse", Message::Browse)
///         .on_input(Message::Path)
///         .into();
/// ```
///
/// A field in a dialog can send its Cancel on Esc with
/// [`TextField::on_escape`].
pub struct TextField<'a, Message> {
    label: String,
    value: &'a str,
    palette: UiPalette,
    placeholder: String,
    hint: Option<String>,
    error: Option<String>,
    optional: Option<String>,
    width: Length,
    action: Option<(String, Message)>,
    is_secure: bool,
    id: Option<iced::widget::Id>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    on_escape: Option<Message>,
    suggestions: Vec<String>,
    on_pick: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    highlighted: Option<usize>,
    on_highlight: Option<Box<dyn Fn(Option<usize>) -> Message + 'a>>,
    on_suggestions_close: Option<Message>,
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
            optional: None,
            width: Length::Fill,
            action: None,
            is_secure: false,
            id: None,
            on_input: None,
            on_submit: None,
            on_escape: None,
            suggestions: Vec::new(),
            on_pick: None,
            highlighted: None,
            on_highlight: None,
            on_suggestions_close: None,
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

    /// Marks the field as one that may stay empty: `marker` shows beside the
    /// label, in the language of the app.
    pub fn optional(mut self, marker: impl Into<String>) -> Self {
        self.optional = Some(marker.into());
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

    /// Hides the typed characters (passwords and passphrases).
    pub fn secure(mut self, is_secure: bool) -> Self {
        self.is_secure = is_secure;
        self
    }

    /// Sets the widget id, so the app can focus the input.
    pub fn id(mut self, id: iced::widget::Id) -> Self {
        self.id = Some(id);
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

    /// Sets the message sent when Esc is pressed in the focused input, such
    /// as Cancel of the dialog it is in. Without it Esc only takes the focus
    /// from the input, as in any `iced` text input.
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
    ///     Password(String),
    ///     Cancel,
    /// }
    ///
    /// let palette = UiPalette::otty_dark();
    /// let field: Element<'_, Message> = TextField::new("Password", "", &palette)
    ///     .secure(true)
    ///     .on_input(Message::Password)
    ///     .on_escape(Message::Cancel)
    ///     .into();
    /// ```
    pub fn on_escape(mut self, message: Message) -> Self {
        self.on_escape = Some(message);
        self
    }

    /// Floats `suggestions` under the input, such as the folders whose
    /// path starts with the typed text; a press on one sends `on_pick`
    /// with its place in the list. The list shows while it is not empty,
    /// so the app closes it by giving none.
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
    ///     Folder(String),
    ///     Picked(usize),
    ///     Closed,
    /// }
    ///
    /// let palette = UiPalette::otty_dark();
    /// let field: Element<'_, Message> =
    ///     TextField::new("Folder", "~/pro", &palette)
    ///         .on_input(Message::Folder)
    ///         .suggestions(vec![String::from("~/projects/")], Message::Picked)
    ///         .on_suggestions_close(Message::Closed)
    ///         .into();
    /// ```
    pub fn suggestions(
        mut self,
        suggestions: Vec<String>,
        on_pick: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        self.suggestions = suggestions;
        self.on_pick = Some(Box::new(on_pick));
        self
    }

    /// Lets the keys move a highlight through the suggestions while they
    /// show. The app keeps the highlighted row and gives it back as
    /// `highlighted`; `on_highlight` sends the row to highlight next, `None`
    /// for none. ↓ highlights the first row, then the next one, and stops at
    /// the last; ↑ highlights the row above, and from the first row leaves
    /// the list. Enter and Tab pick the highlighted row as a press on it
    /// does; with none highlighted they do what they do in any field. The
    /// app clears the highlight when the text changes.
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
    ///     Folder(String),
    ///     Picked(usize),
    ///     Highlighted(Option<usize>),
    /// }
    ///
    /// let palette = UiPalette::otty_dark();
    /// let field: Element<'_, Message> =
    ///     TextField::new("Folder", "~/pro", &palette)
    ///         .on_input(Message::Folder)
    ///         .suggestions(vec![String::from("~/projects/")], Message::Picked)
    ///         .highlighted_suggestion(Some(0), Message::Highlighted)
    ///         .into();
    /// ```
    pub fn highlighted_suggestion(
        mut self,
        highlighted: Option<usize>,
        on_highlight: impl Fn(Option<usize>) -> Message + 'a,
    ) -> Self {
        self.highlighted = highlighted;
        self.on_highlight = Some(Box::new(on_highlight));
        self
    }

    /// Sets the message sent on a press away from the input and its
    /// suggestions while they show, and on Esc while they show; the press
    /// still reaches what is there.
    pub fn on_suggestions_close(mut self, message: Message) -> Self {
        self.on_suggestions_close = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<TextField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: TextField<'a, Message>) -> Self {
        let style = typography::TERM;
        let invalid = value.error.is_some();

        let mut control: TextInput<'a, Message> =
            text_input(&value.placeholder, value.value)
                .secure(value.is_secure)
                .font(fonts::MONO)
                .size(style.size())
                .line_height(Pixels(style.line_height()))
                .padding(Padding::from([PADDING_Y, PADDING_X]))
                .on_input_maybe(value.on_input)
                .on_submit_maybe(value.on_submit)
                .style(input_style(&value.palette, invalid));

        if let Some(id) = value.id {
            control = control.id(id);
        }
        let input: Element<'a, Message> = match value.on_pick {
            Some(on_pick) => suggesting_input(
                control,
                value.value,
                &value.suggestions,
                SuggestionMessages {
                    on_pick: &on_pick,
                    highlighted: value.highlighted,
                    on_highlight: value.on_highlight.as_deref(),
                    on_close: value.on_suggestions_close,
                    on_escape: value.on_escape,
                },
                &value.palette,
            ),
            None => match value.on_escape {
                Some(message) => InputKeys::new(control)
                    .on_key(Named::Escape, message)
                    .into(),
                None => control.into(),
            },
        };

        let mut line = row![input].spacing(spacing::SPACE_2);
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
            value.optional.as_deref(),
            line.into(),
            note,
            &value.palette,
        ))
        .width(value.width)
        .into()
    }
}

/// What the keys and the rows of a field with suggestions send.
struct SuggestionMessages<'s, 'a, Message> {
    on_pick: &'s (dyn Fn(usize) -> Message + 'a),
    highlighted: Option<usize>,
    on_highlight: Option<&'s (dyn Fn(Option<usize>) -> Message + 'a)>,
    on_close: Option<Message>,
    on_escape: Option<Message>,
}

/// The input of a field with suggestions: always wrapped the same way, with
/// the list open or not, so it keeps the keyboard while the rows come and
/// go. While the list shows, ↓ and ↑ move the highlight, Enter and Tab pick
/// the highlighted row and Esc closes the list.
fn suggesting_input<'a, Message: Clone + 'a>(
    control: TextInput<'a, Message>,
    value: &'a str,
    suggestions: &[String],
    messages: SuggestionMessages<'_, 'a, Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let rows = suggestions.len();
    let is_open = rows > 0;
    let highlighted = messages.highlighted.filter(|row| *row < rows);

    let mut field = SuggestionInput::new(control, value, palette)
        .highlighted(highlighted)
        .on_close(messages.on_close.clone());
    if is_open && let Some(on_highlight) = messages.on_highlight {
        field = field.on_key(
            Named::ArrowDown,
            on_highlight(Some(row_below(highlighted, rows))),
        );
        if let Some(row) = highlighted {
            field = field.on_key(Named::ArrowUp, on_highlight(row_above(row)));
        }
    }
    if let Some(row) = highlighted {
        field = field
            .on_key(Named::Enter, (messages.on_pick)(row))
            .on_key(Named::Tab, (messages.on_pick)(row));
    }
    let escape = if is_open {
        messages.on_close.or(messages.on_escape)
    } else {
        messages.on_escape
    };
    if let Some(message) = escape {
        field = field.on_key(Named::Escape, message);
    }

    for (index, suggestion) in suggestions.iter().enumerate() {
        let label = text(suggestion.clone())
            .size(ROW_TEXT_SIZE)
            .font(fonts::MONO)
            .wrapping(Wrapping::None);
        field = field.row(label, Some((messages.on_pick)(index)));
    }

    field.into()
}

/// The row ↓ highlights in a list of `rows` rows: the first one while none
/// is highlighted, else the next one; the last row keeps the highlight.
fn row_below(highlighted: Option<usize>, rows: usize) -> usize {
    match highlighted {
        Some(row) => (row + 1).min(rows.saturating_sub(1)),
        None => 0,
    }
}

/// The row ↑ highlights from `row`: the one above; from the first row the
/// highlight leaves the list.
fn row_above(row: usize) -> Option<usize> {
    row.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;
    use iced::widget::Id;
    use iced_test::simulator;

    use super::*;

    /// The id of the input of the fields under test.
    const FIELD: &str = "field";

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Changed(String),
        Escape,
        Picked(usize),
        Highlighted(Option<usize>),
        Closed,
        Submitted,
    }

    /// A folder field with two suggestions floating under it.
    fn with_suggestions(palette: &UiPalette) -> Element<'_, Message> {
        TextField::new("Folder", "~/pro", palette)
            .id(Id::new(FIELD))
            .on_input(Message::Changed)
            .suggestions(
                vec![String::from("~/projects/"), String::from("~/promo/")],
                Message::Picked,
            )
            .on_suggestions_close(Message::Closed)
            .into()
    }

    #[test]
    fn a_press_on_a_suggestion_sends_its_place_in_the_list() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(with_suggestions(&palette));

        ui.find("~/projects/").expect("the first suggestion shows");
        ui.click("~/promo/").expect("the second suggestion shows");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(1)]);
    }

    #[test]
    fn a_press_away_from_the_suggestions_closes_them() {
        let palette = UiPalette::otty_dark();
        let page = iced::widget::column![
            with_suggestions(&palette),
            iced::widget::Space::new().height(400),
        ];
        let mut ui = simulator(page);

        ui.point_at(iced::Point::new(5.0, 390.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Closed]);
    }

    #[test]
    fn a_field_with_no_suggestions_shows_no_list() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> =
            TextField::new("Folder", "", &palette)
                .on_input(Message::Changed)
                .suggestions(Vec::new(), Message::Picked)
                .into();
        let mut ui = simulator(field);

        assert!(ui.find("~/projects/").is_err());
    }

    /// The rows of a field whose suggestions come and go as the text
    /// changes, as the folders of a path do while it is typed.
    fn coming_and_going(value: &str) -> Vec<String> {
        if value.len() % 2 == 1 {
            vec![String::from("~/projects/")]
        } else {
            Vec::new()
        }
    }

    /// The rows of a field that always suggests two folders.
    fn two_folders(_value: &str) -> Vec<String> {
        vec![String::from("~/projects/"), String::from("~/promo/")]
    }

    /// A field in a running app, its view built again after each event
    /// with the state iced keeps between two frames. The app keeps the
    /// text, whether the list is open and the highlighted row, as the
    /// agents dialogs do: typing opens the list and clears the highlight,
    /// a pick puts the row in the field.
    struct Flow {
        rows: fn(&str) -> Vec<String>,
        value: String,
        is_open: bool,
        highlighted: Option<usize>,
        sent: Vec<Message>,
        renderer: iced_test::renderer::Renderer,
        cache: Option<iced_test::runtime::user_interface::Cache>,
    }

    impl Flow {
        fn new(rows: fn(&str) -> Vec<String>) -> Self {
            use iced_test::core::renderer::Headless;

            let renderer = iced_test::futures::futures::executor::block_on(
                iced_test::renderer::Renderer::new(
                    iced::Font::with_name("Fira Sans"),
                    iced::Pixels(16.0),
                    None,
                ),
            )
            .expect("headless renderer");

            Self {
                rows,
                value: String::new(),
                is_open: true,
                highlighted: None,
                sent: Vec::new(),
                renderer,
                cache: None,
            }
        }

        fn shown_rows(&self) -> Vec<String> {
            if self.is_open {
                (self.rows)(&self.value)
            } else {
                Vec::new()
            }
        }

        fn send(&mut self, events: impl IntoIterator<Item = iced::Event>) {
            let events: Vec<iced::Event> = events.into_iter().collect();
            let palette = UiPalette::otty_dark();
            let rows = self.shown_rows();
            let mut messages = Vec::new();

            let field: Element<'_, Message> =
                TextField::new("Folder", &self.value, &palette)
                    .on_input(Message::Changed)
                    .on_submit(Message::Submitted)
                    .on_escape(Message::Escape)
                    .suggestions(rows.clone(), Message::Picked)
                    .highlighted_suggestion(
                        self.highlighted,
                        Message::Highlighted,
                    )
                    .on_suggestions_close(Message::Closed)
                    .into();
            let mut ui = iced_test::runtime::UserInterface::build(
                field,
                iced::Size::new(400.0, 400.0),
                self.cache.take().unwrap_or_default(),
                &mut self.renderer,
            );
            let _ = ui.update(
                &events,
                iced::mouse::Cursor::Available(iced::Point::new(100.0, 30.0)),
                &mut self.renderer,
                &mut iced_test::core::clipboard::Null,
                &mut messages,
            );
            self.cache = Some(ui.into_cache());

            for message in messages {
                match &message {
                    Message::Changed(value) => {
                        self.value.clone_from(value);
                        self.is_open = true;
                        self.highlighted = None;
                    },
                    Message::Picked(index) => {
                        if let Some(row) = rows.get(*index) {
                            self.value.clone_from(row);
                        }
                        self.highlighted = None;
                    },
                    Message::Highlighted(row) => self.highlighted = *row,
                    Message::Closed => {
                        self.is_open = false;
                        self.highlighted = None;
                    },
                    _ => {},
                }
                self.sent.push(message);
            }
        }

        fn click(&mut self) {
            self.send(iced_test::simulator::click());
        }

        fn type_text(&mut self, text: &str) {
            self.send(iced_test::simulator::typewrite(text));
        }

        fn tap(&mut self, key: Named) {
            self.send(iced_test::simulator::tap_key(key, None));
        }
    }

    #[test]
    fn the_field_keeps_the_focus_while_its_suggestions_come_and_go() {
        let mut flow = Flow::new(coming_and_going);

        flow.click();
        for letter in ["~", "/", "p", "r"] {
            flow.type_text(letter);
        }

        assert_eq!(flow.value, "~/pr");
    }

    /// Issue 39: ↓ highlights the first row, then the next one, and stops
    /// at the last; the highlight lives through the frames.
    #[test]
    fn down_highlights_the_first_row_then_the_next_and_stops_at_the_last() {
        let mut flow = Flow::new(two_folders);
        flow.click();
        flow.type_text("~/pro");

        flow.tap(Named::ArrowDown);
        let first = flow.highlighted;
        flow.tap(Named::ArrowDown);
        let second = flow.highlighted;
        flow.tap(Named::ArrowDown);

        assert_eq!(first, Some(0));
        assert_eq!(second, Some(1));
        assert_eq!(flow.highlighted, Some(1), "no wrap at the end");
        assert_eq!(flow.value, "~/pro", "the text stays as typed");
    }

    /// Issue 39: ↑ moves up, and from the first row it leaves the list.
    #[test]
    fn up_moves_up_and_from_the_first_row_leaves_the_list() {
        let mut flow = Flow::new(two_folders);
        flow.click();
        flow.type_text("~/pro");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::ArrowDown);

        flow.tap(Named::ArrowUp);
        let moved_up = flow.highlighted;
        flow.tap(Named::ArrowUp);
        let left = flow.highlighted;
        flow.tap(Named::ArrowUp);

        assert_eq!(moved_up, Some(0));
        assert_eq!(left, None);
        assert_eq!(flow.highlighted, None);
    }

    /// Issue 39: Enter on a highlighted row picks it as a press does, and
    /// the next letter goes after the picked text.
    #[test]
    fn enter_picks_the_highlighted_row_and_typing_goes_on_after_it() {
        let mut flow = Flow::new(two_folders);
        flow.click();
        flow.type_text("~/pro");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::ArrowDown);

        flow.tap(Named::Enter);
        let picked = flow.value.clone();
        flow.type_text("x");

        assert_eq!(picked, "~/promo/");
        assert_eq!(flow.value, "~/promo/x");
        assert!(!flow.sent.contains(&Message::Submitted));
    }

    #[test]
    fn tab_picks_the_highlighted_row() {
        let mut flow = Flow::new(two_folders);
        flow.click();
        flow.type_text("~/pro");
        flow.tap(Named::ArrowDown);

        flow.tap(Named::Tab);

        assert_eq!(flow.value, "~/projects/");
    }

    /// With no row highlighted, Enter does what it does in any field.
    #[test]
    fn enter_with_no_row_highlighted_submits() {
        let mut flow = Flow::new(two_folders);
        flow.click();
        flow.type_text("~/pro");

        flow.tap(Named::Enter);

        assert_eq!(flow.value, "~/pro");
        assert_eq!(flow.sent.last(), Some(&Message::Submitted));
    }

    /// Typing after a move clears the highlight: Enter then submits.
    #[test]
    fn typing_after_a_move_clears_the_highlight() {
        let mut flow = Flow::new(two_folders);
        flow.click();
        flow.type_text("~/pro");
        flow.tap(Named::ArrowDown);

        flow.type_text("j");
        flow.tap(Named::Enter);

        assert_eq!(flow.value, "~/proj");
        assert_eq!(flow.sent.last(), Some(&Message::Submitted));
    }

    /// Esc closes the list first; the next Esc is the field's own.
    #[test]
    fn esc_closes_the_list_first() {
        let mut flow = Flow::new(two_folders);
        flow.click();
        flow.type_text("~/pro");
        flow.tap(Named::ArrowDown);

        flow.tap(Named::Escape);
        let closed = flow.sent.last().cloned();
        flow.tap(Named::Escape);

        assert_eq!(closed, Some(Message::Closed));
        assert_eq!(flow.sent.last(), Some(&Message::Escape));
    }

    #[test]
    fn esc_in_the_focused_field_sends_the_escape_message() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> = TextField::new("Name", "", &palette)
            .id(Id::new(FIELD))
            .on_input(Message::Changed)
            .on_escape(Message::Escape)
            .into();
        let mut ui = simulator(field);

        ui.click(Id::new(FIELD)).expect("the field should show");
        ui.tap_key(Named::Escape);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Escape]);
    }

    /// Without an Esc message the input behaves as an `iced` input: Esc
    /// only takes its focus, so the next letter goes nowhere.
    #[test]
    fn a_field_without_an_escape_message_keeps_today_s_esc() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> = TextField::new("Name", "", &palette)
            .id(Id::new(FIELD))
            .on_input(Message::Changed)
            .into();
        let mut ui = simulator(field);

        ui.click(Id::new(FIELD)).expect("the field should show");
        ui.tap_key(Named::Escape);
        let _ = ui.typewrite("a");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(messages.is_empty());
    }

    #[test]
    fn given_an_optional_marker_when_drawn_then_it_reads_as_given() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, ()> = TextField::new("Имя", "", &palette)
            .optional("необязательно")
            .into();
        let mut ui = simulator(field);

        assert!(ui.find("необязательно").is_ok());
        assert!(ui.find("optional").is_err());
    }

    #[test]
    fn given_no_optional_marker_when_drawn_then_only_the_label_shows() {
        let palette = UiPalette::otty_dark();
        let field: Element<'_, ()> =
            TextField::new("Name", "", &palette).into();
        let mut ui = simulator(field);

        assert!(ui.find("Name").is_ok());
        assert!(ui.find("optional").is_err());
    }
}
