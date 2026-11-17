//! `TextField`: labelled single-line input with a hint or an error.

use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::text_input::TextInput;
use iced::widget::{
    button, column, container, row, scrollable, text, text_input,
};
use iced::{Alignment, Border, Element, Length, Padding, Pixels};

use super::button::Button;
use super::field_chrome::{self, Note, input_style};
use super::floating_list::FloatingList;
use super::input_keys::InputKeys;
use super::{menu, scrollbar};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Horizontal padding of the input (`padding: 0 11px`).
pub(super) const PADDING_X: f32 = 11.0;
/// Vertical padding that makes a 13/20 text line 34px high.
pub(super) const PADDING_Y: f32 = 7.0;
/// Rows the suggestion list shows before it scrolls.
const SUGGESTION_ROWS: f32 = 8.0;
/// Text size of a suggestion, as in a menu item (`font-size: 12.5px`).
const SUGGESTION_TEXT_SIZE: f32 = 12.5;
/// Horizontal padding of a suggestion, as in a menu item (`padding: 0 10px`).
const SUGGESTION_PADDING_X: f32 = 10.0;

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

    /// Sets the message sent on a press away from the input and its
    /// suggestions while they show; the press still reaches what is there.
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
        let mut input: Element<'a, Message> = match value.on_escape {
            Some(message) => InputKeys::new(control)
                .on_key(Named::Escape, message)
                .into(),
            None => control.into(),
        };
        // A field with suggestions is always wrapped, with the list open
        // or not: a wrapper that came and went with the list would rebuild
        // the input and take its focus after each letter.
        if let Some(on_pick) = value.on_pick {
            let mut floating = FloatingList::new(input);
            if !value.suggestions.is_empty() {
                let list = suggestion_list(
                    &value.suggestions,
                    &on_pick,
                    &value.palette,
                );
                floating = floating.list(list, None, size::ROW);
            }
            input = floating.on_close(value.on_suggestions_close).into();
        }

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

/// The floating surface with one row per suggestion, in the look of a
/// menu. Past [`SUGGESTION_ROWS`] rows it scrolls.
fn suggestion_list<'a, Message: Clone + 'a>(
    suggestions: &[String],
    on_pick: &dyn Fn(usize) -> Message,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut rows = column![].width(Length::Fill);
    for (index, suggestion) in suggestions.iter().enumerate() {
        let label = text(suggestion.clone())
            .size(SUGGESTION_TEXT_SIZE)
            .font(fonts::MONO)
            .wrapping(Wrapping::None);
        let row = button(
            container(label)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .clip(true),
        )
        .width(Length::Fill)
        .height(size::ROW)
        .padding(Padding::from([0.0, SUGGESTION_PADDING_X]))
        .on_press(on_pick(index))
        .style(menu::item_style(palette, false));
        rows = rows.push(row);
    }

    let background = palette.surface_overlay();
    let shadow = palette.shadow_popover();
    let max_height = SUGGESTION_ROWS * size::ROW + 2.0 * menu::MENU_PADDING;

    container(
        scrollable(rows)
            .width(Length::Fill)
            .direction(scrollbar::vertical())
            .style(scrollbar::style(palette)),
    )
    .width(Length::Fill)
    .max_height(max_height)
    .padding(menu::MENU_PADDING)
    .style(move |_| container::Style {
        background: Some(background.into()),
        border: Border {
            radius: radius::RADIUS_MD.into(),
            ..Border::default()
        },
        shadow,
        ..container::Style::default()
    })
    .into()
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
        Closed,
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

    /// A field typed into one letter at a time, its view built again after
    /// each letter with the state iced keeps between two frames.
    struct Typing {
        value: String,
        renderer: iced_test::renderer::Renderer,
        cache: Option<iced_test::runtime::user_interface::Cache>,
    }

    impl Typing {
        fn new() -> Self {
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
                value: String::new(),
                renderer,
                cache: None,
            }
        }

        /// The suggestions come and go as the text changes, as the folders
        /// of a path do while it is typed.
        fn view<'a>(
            value: &'a str,
            palette: &'a UiPalette,
        ) -> Element<'a, Message> {
            let suggestions = if value.len() % 2 == 1 {
                vec![String::from("~/projects/")]
            } else {
                Vec::new()
            };

            TextField::new("Folder", value, palette)
                .on_input(Message::Changed)
                .suggestions(suggestions, Message::Picked)
                .on_suggestions_close(Message::Closed)
                .into()
        }

        fn send(&mut self, events: Vec<iced::Event>) {
            let palette = UiPalette::otty_dark();
            let mut messages = Vec::new();

            let mut ui = iced_test::runtime::UserInterface::build(
                Self::view(&self.value, &palette),
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
                if let Message::Changed(value) = message {
                    self.value = value;
                }
            }
        }
    }

    #[test]
    fn the_field_keeps_the_focus_while_its_suggestions_come_and_go() {
        let mut typing = Typing::new();

        typing.send(iced_test::simulator::click().collect());
        for letter in ["~", "/", "p", "r"] {
            typing.send(iced_test::simulator::typewrite(letter).collect());
        }

        assert_eq!(typing.value, "~/pr");
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
