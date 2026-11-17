//! `TagSearchField`: the input of a [`TagSearch`] with its floating
//! suggestion list.
//!
//! The list follows the keys of every list under a field of the kit: no row
//! is lit until ↓, ↓ and ↑ stop at the ends, ↑ from the first row leaves the
//! list, Enter and Tab pick the lit row, Esc closes the list. With no row
//! lit, Enter applies the typed text and Tab moves the focus. Each of these,
//! typing, Backspace in an empty field, a press on a suggestion, a press
//! outside the list and the input getting the focus sends a
//! [`TagSearchAction`].
//!
//! [`TagSearch`]: super::tag_search::TagSearch

use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::{Id, row, text, text_input};
use iced::{Element, Padding, Pixels};

use super::command_palette::title_pieces;
use super::field_chrome::input_style;
use super::suggestion_input::{ROW_TEXT_SIZE, SuggestionInput};
use super::tag_search::{
    Suggestion, SuggestionKind, TagSearch, TagSearchAction,
};
use super::text_field::{PADDING_X, PADDING_Y};
use crate::palette::UiPalette;
use crate::{fonts, typography};
/// What follows a key in its suggestion and in the field.
const KEY_END: &str = ":";

/// A search field with the suggestions of a [`TagSearch`] floating under it.
/// It fills the width it gets, so it works as the field of a [`Toolbar`].
/// Without an action handler the field is disabled.
///
/// [`Toolbar`]: super::toolbar::Toolbar
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::tag_search::{
///     SearchRow, TagKey, TagSchema, TagSearch, TagSearchAction,
/// };
/// use otty_ui_kit::components::tag_search_field::TagSearchField;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Search(TagSearchAction),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let schema = TagSchema::new(
///     vec![TagKey::new("name", "Name")],
///     vec![SearchRow::new("web-1")],
/// );
/// let search = TagSearch::default();
/// let field: Element<'_, Message> =
///     TagSearchField::new(&search, search.suggestions(&schema), &palette)
///         .placeholder("Filter")
///         .on_action(Message::Search)
///         .into();
/// ```
pub struct TagSearchField<'a, Message> {
    search: &'a TagSearch,
    suggestions: Vec<Suggestion>,
    palette: UiPalette,
    placeholder: String,
    id: Option<Id>,
    on_action: Option<Rc<dyn Fn(TagSearchAction) -> Message + 'a>>,
}

impl<'a, Message> TagSearchField<'a, Message> {
    /// Creates a field that shows `search` and `suggestions`, the result of
    /// [`TagSearch::suggestions`].
    pub fn new(
        search: &'a TagSearch,
        suggestions: Vec<Suggestion>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            search,
            suggestions,
            palette: palette.clone(),
            placeholder: String::new(),
            id: None,
            on_action: None,
        }
    }

    /// Sets the text shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the widget id of the input, so the app can focus it.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the function that maps a [`TagSearchAction`] to the app message
    /// and enables the field.
    pub fn on_action(
        mut self,
        on_action: impl Fn(TagSearchAction) -> Message + 'a,
    ) -> Self {
        self.on_action = Some(Rc::new(on_action));
        self
    }
}

impl<'a, Message: Clone + 'a> From<TagSearchField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: TagSearchField<'a, Message>) -> Self {
        let palette = value.palette;
        let on_action = value.on_action;
        let send = |action: TagSearchAction| {
            on_action.as_ref().map(|on_action| on_action(action))
        };

        let is_open = value.search.is_open() && !value.suggestions.is_empty();
        let highlighted = value
            .suggestions
            .iter()
            .position(Suggestion::is_highlighted);
        let on_pick = on_action.clone().map(|on_action| {
            Box::new(move |row| on_action(TagSearchAction::Pick(row)))
                as Box<dyn Fn(usize) -> Message + 'a>
        });
        let on_highlight = on_action.clone().map(|on_action| {
            Box::new(move |row| on_action(TagSearchAction::Highlight(row)))
                as Box<dyn Fn(Option<usize>) -> Message + 'a>
        });
        let on_input = on_action.clone().map(|on_action| {
            move |text| on_action(TagSearchAction::InputChanged(text))
        });

        let style = typography::TERM;
        let mut input = text_input(&value.placeholder, value.search.input())
            .font(fonts::MONO)
            .size(style.size())
            .line_height(Pixels(style.line_height()))
            .padding(Padding::from([PADDING_Y, PADDING_X]))
            .on_input_maybe(on_input)
            .on_submit_maybe(send(TagSearchAction::Apply))
            .style(input_style(&palette, false));
        if let Some(id) = value.id {
            input = input.id(id);
        }

        let mut field =
            SuggestionInput::new(input, value.search.input(), &palette)
                .highlighted(highlighted)
                .on_pick(on_pick)
                .on_highlight(on_highlight)
                .on_focus(send(TagSearchAction::Focused))
                .on_close(send(TagSearchAction::Close));
        if value.search.input().is_empty()
            && let Some(message) = send(TagSearchAction::RemoveLastTag)
        {
            field = field.on_key(Named::Backspace, message);
        }
        if is_open {
            for suggestion in &value.suggestions {
                field = field.row(row_content(suggestion, &palette));
            }
        }

        field.into()
    }
}

/// The content of one suggestion row: the matched part in `text-strong`
/// bold, as in the command palette. A key is in the accent color and ends
/// with `:`, so it reads differently from a name or a value.
fn row_content<'a, Message: 'a>(
    suggestion: &Suggestion,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let is_key = suggestion.kind() == SuggestionKind::Key;
    let strong = palette.text_strong();
    let accent = palette.accent();

    let piece = |content: &str| {
        text(content.to_owned())
            .size(ROW_TEXT_SIZE)
            .font(fonts::MONO)
            .wrapping(Wrapping::None)
    };

    let mut content = row![];
    for (part, is_match) in title_pieces(suggestion.text(), suggestion.ranges())
    {
        let part = piece(part);
        content = content.push(if is_match {
            part.font(fonts::MONO_BOLD).color(strong)
        } else if is_key {
            part.color(accent)
        } else {
            part
        });
    }
    if is_key {
        content = content.push(piece(KEY_END).color(accent));
    }

    content.into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;
    use iced::widget::{Space, button, column, container, text};
    use iced::{Event, Font, Length, Point, Size, mouse};
    use iced_test::core::clipboard;
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::glyph;
    use crate::components::tag_search::{SearchRow, TagKey, TagSchema};
    use crate::components::toolbar::{Toolbar, ToolbarItem};
    use crate::size;

    const INPUT: &str = "tag-search-test-input";

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Search(TagSearchAction),
        Under,
    }

    /// Keys whose shown name is their internal name.
    fn keys() -> Vec<TagKey> {
        vec![TagKey::new("name", "name"), TagKey::new("status", "status")]
    }

    fn schema() -> TagSchema {
        TagSchema::new(
            keys(),
            vec![
                SearchRow::new("web-1")
                    .value("name", "web-1")
                    .value("status", "Running"),
                SearchRow::new("db-1")
                    .value("name", "db-1")
                    .value("status", "Exited"),
            ],
        )
    }

    /// A search with `text` typed and its list open.
    fn typed(text: &str) -> TagSearch {
        let mut search = TagSearch::default();
        let _ = search
            .perform(TagSearchAction::InputChanged(text.to_owned()), &schema());

        search
    }

    /// A search with `text` typed and its list closed.
    fn closed(text: &str) -> TagSearch {
        let mut search = typed(text);
        let _ = search.perform(TagSearchAction::Close, &schema());

        search
    }

    /// The field over a button, as the suggestions float over a table.
    fn view<'a>(
        search: &'a TagSearch,
        schema: &TagSchema,
    ) -> Element<'a, Message> {
        let palette = UiPalette::otty_dark();

        column![
            container(
                TagSearchField::new(
                    search,
                    search.suggestions(schema),
                    &palette
                )
                .placeholder("Filter")
                .id(Id::new(INPUT))
                .on_action(Message::Search)
            )
            .width(300),
            button(text("under"))
                .width(300)
                .height(200)
                .on_press(Message::Under),
        ]
        .into()
    }

    fn click_at(ui: &mut Simulator<'_, Message>, point: Point) {
        ui.point_at(point);
        let _ = ui.simulate(simulator::click());
    }

    /// Presses the input of the field, then `key`, and returns what came
    /// after the press.
    fn tap_in_input(search: &TagSearch, key: Named) -> Vec<Message> {
        let schema = schema();
        let mut ui = simulator(view(search, &schema));

        ui.click(Id::new(INPUT)).expect("input should be visible");
        ui.tap_key(key);

        ui.into_messages()
            .filter(|message| {
                *message != Message::Search(TagSearchAction::Focused)
            })
            .collect()
    }

    /// The field in a running app: the search lives in the test, every
    /// message goes to it, and the widget state is kept between events, as
    /// `iced` keeps it between frames.
    struct Flow {
        search: TagSearch,
        schema: TagSchema,
        renderer: iced_test::renderer::Renderer,
        cache: Option<iced_test::runtime::user_interface::Cache>,
        cursor: mouse::Cursor,
    }

    impl Flow {
        fn new() -> Self {
            use iced_test::core::renderer::Headless;

            let renderer = iced_test::futures::futures::executor::block_on(
                iced_test::renderer::Renderer::new(
                    Font::with_name("Fira Sans"),
                    Pixels(16.0),
                    None,
                ),
            )
            .expect("headless renderer");

            Self {
                search: TagSearch::default(),
                schema: schema(),
                renderer,
                cache: None,
                cursor: mouse::Cursor::Unavailable,
            }
        }

        /// Sends `events` to a rebuilt view and applies the messages.
        fn send(&mut self, events: impl IntoIterator<Item = Event>) {
            let events: Vec<Event> = events.into_iter().collect();
            let mut messages = Vec::new();

            let mut ui = iced_test::runtime::UserInterface::build(
                view(&self.search, &self.schema),
                Size::new(400.0, 400.0),
                self.cache.take().unwrap_or_default(),
                &mut self.renderer,
            );
            let _ = ui.update(
                &events,
                self.cursor,
                &mut self.renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            self.cache = Some(ui.into_cache());

            for message in messages {
                if let Message::Search(action) = message {
                    let _ = self.search.perform(action, &self.schema);
                }
            }
        }

        /// Presses the right end of the input, where a click puts the cursor
        /// at the end of the text.
        fn click_input(&mut self) {
            self.cursor = mouse::Cursor::Available(Point::new(290.0, 15.0));
            self.send(simulator::click());
        }

        fn click_text(&mut self, content: &str) {
            let search = self.search.clone();
            let schema = self.schema.clone();
            let mut ui = simulator(view(&search, &schema));
            let point = ui.find(content).expect("text should show").bounds();

            self.cursor = mouse::Cursor::Available(point.center());
            self.send(simulator::click());
        }

        fn type_text(&mut self, content: &str) {
            self.send(simulator::typewrite(content));
        }

        fn tap(&mut self, key: Named) {
            self.send(simulator::tap_key(key, None));
        }

        fn input(&self) -> &str {
            self.search.input()
        }

        /// The place of the lit row, if one is.
        fn highlighted(&self) -> Option<usize> {
            self.search
                .suggestions(&self.schema)
                .iter()
                .position(Suggestion::is_highlighted)
        }

        /// The texts of the rows the list shows.
        fn rows(&self) -> Vec<String> {
            self.search
                .suggestions(&self.schema)
                .iter()
                .map(|row| row.text().to_owned())
                .collect()
        }
    }

    // Issue 41: the keys of the folder fields.

    #[test]
    fn at_first_no_row_is_lit() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("status:");

        assert_eq!(flow.rows(), vec!["Running", "Exited"]);
        assert_eq!(flow.highlighted(), None);
    }

    #[test]
    fn down_lights_the_first_row_then_the_next_and_stops_at_the_last() {
        let mut flow = Flow::new();
        flow.click_input();
        flow.type_text("status:");

        flow.tap(Named::ArrowDown);
        let first = flow.highlighted();
        flow.tap(Named::ArrowDown);
        let second = flow.highlighted();
        flow.tap(Named::ArrowDown);

        assert_eq!(first, Some(0));
        assert_eq!(second, Some(1));
        assert_eq!(flow.highlighted(), Some(1), "no wrap at the end");
        assert_eq!(flow.input(), "status:", "the text stays as typed");
    }

    #[test]
    fn up_moves_up_and_from_the_first_row_leaves_the_list() {
        let mut flow = Flow::new();
        flow.click_input();
        flow.type_text("status:");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::ArrowDown);

        flow.tap(Named::ArrowUp);
        let moved_up = flow.highlighted();
        flow.tap(Named::ArrowUp);
        let left = flow.highlighted();
        flow.tap(Named::ArrowUp);

        assert_eq!(moved_up, Some(0));
        assert_eq!(left, None);
        assert_eq!(flow.highlighted(), None, "no wrap at the start");
        assert_eq!(flow.rows().len(), 2, "the list stays open");
    }

    #[test]
    fn enter_picks_the_lit_row() {
        let mut flow = Flow::new();
        flow.click_input();
        flow.type_text("status:");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::ArrowDown);

        flow.tap(Named::Enter);

        assert_eq!(
            flow.search.tags()[0].label(flow.schema.keys()),
            "status:Exited"
        );
        assert_eq!(flow.input(), "");
    }

    #[test]
    fn tab_picks_the_lit_row() {
        let mut flow = Flow::new();
        flow.click_input();
        flow.type_text("status:");
        flow.tap(Named::ArrowDown);

        flow.tap(Named::Tab);

        assert_eq!(
            flow.search.tags()[0].label(flow.schema.keys()),
            "status:Running"
        );
    }

    #[test]
    fn enter_with_no_row_lit_applies_the_text() {
        let mut flow = Flow::new();
        flow.click_input();
        flow.type_text("status:ru");

        flow.tap(Named::Enter);

        assert_eq!(
            flow.search.tags()[0].label(flow.schema.keys()),
            "status:ru"
        );
        assert_eq!(flow.input(), "");
    }

    #[test]
    fn tab_with_no_row_lit_is_left_to_move_the_focus() {
        assert!(tap_in_input(&typed("sta"), Named::Tab).is_empty());
    }

    #[test]
    fn typing_clears_the_lit_row() {
        let mut flow = Flow::new();
        flow.click_input();
        flow.type_text("s");
        flow.tap(Named::ArrowDown);

        flow.type_text("t");

        assert_eq!(flow.input(), "st");
        assert_eq!(flow.highlighted(), None);
    }

    #[test]
    fn esc_closes_the_list_first() {
        let mut flow = Flow::new();
        flow.click_input();
        flow.type_text("status:");
        flow.tap(Named::ArrowDown);

        flow.tap(Named::Escape);

        assert!(!flow.search.is_open());
        assert!(tap_in_input(&flow.search, Named::Escape).is_empty());
    }

    #[test]
    fn typing_after_a_key_applied_by_enter_goes_after_the_colon() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("sta");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Enter);
        flow.type_text("r");

        assert_eq!(flow.input(), "status:r");
    }

    #[test]
    fn typing_after_a_key_applied_by_tab_goes_after_the_colon() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("sta");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);
        flow.type_text("r");

        assert_eq!(flow.input(), "status:r");
    }

    #[test]
    fn typing_after_a_key_picked_by_a_click_goes_after_the_colon() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("sta");
        flow.click_text("tus");
        flow.type_text("r");

        assert_eq!(flow.input(), "status:r");
    }

    /// Keys and rows as a Russian table shows them.
    fn russian_schema() -> TagSchema {
        TagSchema::new(
            vec![
                TagKey::new("name", "Имя"),
                TagKey::new("status", "Состояние"),
            ],
            vec![
                SearchRow::new("web-1")
                    .value("name", "web-1")
                    .value("status", "Работает"),
            ],
        )
    }

    #[test]
    fn typing_after_a_shown_name_applied_goes_after_its_colon() {
        let mut flow = Flow::new();
        flow.schema = russian_schema();

        flow.click_input();
        flow.type_text("Сост");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);
        flow.type_text("раб");

        assert_eq!(flow.input(), "Состояние:раб");

        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);

        assert_eq!(flow.input(), "");
        assert_eq!(
            flow.search.tags()[0].label(flow.schema.keys()),
            "Состояние:Работает"
        );
    }

    #[test]
    fn the_list_shows_the_shown_names_and_never_the_internal_names() {
        let schema = russian_schema();
        let mut search = TagSearch::default();
        let _ = search.perform(TagSearchAction::Focused, &schema);
        let mut ui = simulator(view(&search, &schema));

        assert!(ui.find("Имя").is_ok());
        assert!(ui.find("Состояние").is_ok());
        assert!(ui.find("name").is_err());
        assert!(ui.find("status").is_err());
    }

    #[test]
    fn typing_after_a_name_applied_goes_after_the_name() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("web");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);
        flow.type_text("x");

        assert_eq!(flow.input(), "web-1x");
    }

    #[test]
    fn enter_on_the_typed_text_keeps_it_and_typing_goes_on_after_it() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("web");
        flow.tap(Named::Enter);
        flow.type_text("x");

        assert_eq!(flow.input(), "webx");
        assert!(flow.search.tags().is_empty());
    }

    #[test]
    fn typing_after_a_name_picked_by_a_click_goes_after_the_name() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("web");
        flow.click_text("-1");
        flow.type_text("x");

        assert_eq!(flow.input(), "web-1x");
    }

    #[test]
    fn typing_in_the_middle_of_the_text_keeps_the_cursor_there() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("ac");
        flow.tap(Named::ArrowLeft);
        flow.type_text("b");
        flow.type_text("d");

        assert_eq!(flow.input(), "abdc");
    }

    #[test]
    fn arrows_of_the_list_do_not_move_the_cursor() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("s");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::ArrowUp);
        flow.type_text("t");

        assert_eq!(flow.input(), "st");
    }

    #[test]
    fn a_value_applied_by_a_click_leaves_the_field_empty_and_focused() {
        let mut flow = Flow::new();

        flow.click_input();
        flow.type_text("status:");
        flow.click_text("Exited");
        flow.type_text("s");

        assert_eq!(flow.input(), "s");
        assert_eq!(flow.search.tags().len(), 1);
    }

    #[test]
    fn typing_sends_the_new_text() {
        let search = TagSearch::default();
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        ui.click(Id::new(INPUT)).expect("input should be visible");
        let _ = ui.typewrite("w");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Search(TagSearchAction::Focused),
                Message::Search(TagSearchAction::InputChanged("w".to_owned())),
            ]
        );
    }

    #[test]
    fn each_key_of_an_open_list_with_no_row_lit_sends_its_action() {
        let cases = [
            (Named::ArrowDown, Some(TagSearchAction::Highlight(Some(0)))),
            (Named::ArrowUp, None),
            (Named::Enter, Some(TagSearchAction::Apply)),
            (Named::Tab, None),
            (Named::Escape, Some(TagSearchAction::Close)),
        ];

        for (key, action) in cases {
            let expected: Vec<Message> =
                action.into_iter().map(Message::Search).collect();
            assert_eq!(tap_in_input(&typed("s"), key), expected, "key {key:?}");
        }
    }

    #[test]
    fn with_a_closed_list_enter_applies_and_the_arrows_send_nothing() {
        let cases = [
            (Named::Enter, Some(TagSearchAction::Apply)),
            (Named::ArrowDown, None),
            (Named::Tab, None),
        ];

        for (key, action) in cases {
            let expected: Vec<Message> =
                action.into_iter().map(Message::Search).collect();
            assert_eq!(
                tap_in_input(&closed("status:x"), key),
                expected,
                "key {key:?}"
            );
        }
    }

    #[test]
    fn esc_with_a_closed_list_is_left_to_the_input() {
        assert!(tap_in_input(&closed("s"), Named::Escape).is_empty());
    }

    #[test]
    fn backspace_in_an_empty_field_asks_to_remove_the_last_tag() {
        assert_eq!(
            tap_in_input(&TagSearch::default(), Named::Backspace),
            vec![Message::Search(TagSearchAction::RemoveLastTag)]
        );
    }

    #[test]
    fn backspace_with_text_edits_the_text() {
        assert_eq!(
            tap_in_input(&closed("ab"), Named::Backspace),
            vec![Message::Search(TagSearchAction::InputChanged(
                "a".to_owned()
            ))]
        );
    }

    #[test]
    fn keys_do_nothing_while_the_input_has_no_focus() {
        let search = typed("s");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        ui.tap_key(Named::ArrowDown);
        ui.tap_key(Named::Enter);
        ui.tap_key(Named::Escape);

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn focusing_the_input_says_so_once() {
        let search = TagSearch::default();
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        ui.click(Id::new(INPUT)).expect("input should be visible");
        ui.click(Id::new(INPUT)).expect("input should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Search(TagSearchAction::Focused)]);
    }

    #[test]
    fn an_empty_open_field_lists_the_keys() {
        let mut search = TagSearch::default();
        let _ = search.perform(TagSearchAction::Focused, &schema());
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        assert!(ui.find("name").is_ok());
        assert!(ui.find("status").is_ok());
        assert!(ui.find("web-1").is_err(), "names wait for typed text");
    }

    #[test]
    fn a_closed_list_shows_nothing() {
        let search = closed("s");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        assert!(ui.find("tatus").is_err());
    }

    #[test]
    fn the_matched_part_of_a_suggestion_is_split_from_the_rest() {
        let search = typed("sta");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        assert!(ui.find("sta").is_ok());
        assert!(ui.find("tus").is_ok());
    }

    #[test]
    fn a_key_ends_with_a_colon_and_a_name_does_not() {
        let search = typed("e");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        let key = ui.find("nam").expect("the key should show").bounds();
        let colon = ui.find(":").expect("the colon should show").bounds();
        let name = ui.find("web-1").map(|found| found.bounds());

        assert!((colon.center_y() - key.center_y()).abs() < 1.0);
        assert!(colon.x > key.x);
        assert!(name.is_err(), "the name is split at the match");
        assert!(ui.find("b-1").is_ok());
    }

    #[test]
    fn pressing_a_suggestion_picks_its_position() {
        let search = typed("status:");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        ui.click("Exited").expect("value should be listed");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Search(TagSearchAction::Pick(1))]);
    }

    #[test]
    fn the_input_keeps_the_focus_after_a_suggestion_is_pressed() {
        let search = typed("status:");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        ui.click(Id::new(INPUT)).expect("input should be visible");
        ui.click("Exited").expect("value should be listed");
        ui.tap_key(Named::ArrowDown);

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Search(TagSearchAction::Focused),
                Message::Search(TagSearchAction::Pick(1)),
                Message::Search(TagSearchAction::Highlight(Some(0))),
            ]
        );
    }

    #[test]
    fn the_list_sits_above_the_content_under_it() {
        let search = typed("status:");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        let value =
            ui.find("Running").expect("value should be listed").bounds();
        let under = ui.find("under").expect("button should show").bounds();
        assert!(
            value.y >= under.y - size::ROW,
            "the list is over the button"
        );

        click_at(&mut ui, value.center());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Search(TagSearchAction::Pick(0))]);
    }

    #[test]
    fn a_press_outside_closes_the_list_and_reaches_what_is_there() {
        let search = typed("status:");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        let under = ui.find("under").expect("button should show").bounds();
        click_at(&mut ui, Point::new(under.center_x(), under.y + 190.0));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::Search(TagSearchAction::Close), Message::Under]
        );
    }

    #[test]
    fn a_press_in_the_input_keeps_the_list_open() {
        let search = typed("status:");
        let schema = schema();
        let mut ui = simulator(view(&search, &schema));

        ui.click(Id::new(INPUT)).expect("input should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Search(TagSearchAction::Focused)]);
    }

    /// A search over 20 rows with the value at `lit` highlighted.
    fn long_list(lit: Option<usize>) -> (TagSearch, TagSchema) {
        let rows = (0..20)
            .map(|index| {
                SearchRow::new("row")
                    .value("status", format!("value-{index:02}"))
            })
            .collect();
        let schema = TagSchema::new(keys(), rows);
        let mut search = TagSearch::default();
        let _ = search.perform(
            TagSearchAction::InputChanged("status:".to_owned()),
            &schema,
        );
        let _ = search.perform(TagSearchAction::Highlight(lit), &schema);

        (search, schema)
    }

    #[test]
    fn a_long_list_shows_about_eight_rows_then_scrolls() {
        let (search, schema) = long_list(None);
        let mut ui = simulator(view(&search, &schema));

        let eighth = ui.find("value-07").expect("row should be listed");
        let ninth = ui.find("value-08").expect("row should be listed");

        assert!(eighth.visible_bounds().is_some());
        assert!(ninth.visible_bounds().is_none(), "the ninth row scrolls");
    }

    #[test]
    fn the_highlighted_row_is_kept_in_view() {
        let (search, schema) = long_list(Some(15));
        let mut ui = simulator(view(&search, &schema));

        let highlighted = ui.find("value-15").expect("row should be listed");
        let first = ui.find("value-00").expect("row should be listed");

        assert!(highlighted.visible_bounds().is_some());
        assert!(first.visible_bounds().is_none(), "the list has scrolled");
    }

    #[test]
    fn a_press_on_a_scrolled_row_picks_that_row() {
        let (search, schema) = long_list(Some(15));
        let mut ui = simulator(view(&search, &schema));

        ui.click("value-15").expect("row should be listed");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Search(TagSearchAction::Pick(15))]);
    }

    #[test]
    fn without_a_handler_the_field_is_disabled() {
        let search = TagSearch::default();
        let palette = UiPalette::otty_dark();
        let field: Element<'_, Message> =
            TagSearchField::new(&search, Vec::new(), &palette)
                .id(Id::new(INPUT))
                .into();
        let mut ui = simulator(field);

        ui.click(Id::new(INPUT)).expect("input should be visible");
        let _ = ui.typewrite("w");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn it_works_as_the_field_of_a_toolbar() {
        let search = typed("status:");
        let schema = schema();
        let suggestions = search.suggestions(&schema);
        let palette = UiPalette::otty_dark();
        let field_palette = palette.clone();
        let shown = &search;
        let toolbar = Toolbar::new("More", &palette)
            .field(120.0, 240.0, move || {
                TagSearchField::new(shown, suggestions.clone(), &field_palette)
                    .id(Id::new(INPUT))
                    .on_action(Message::Search)
                    .into()
            })
            .item(ToolbarItem::new(glyph::plus(), "Add"));
        let page: Element<'_, Message> = column![
            container(toolbar).width(400),
            Space::new().height(Length::Fill),
        ]
        .into();
        let mut ui = simulator(page);

        ui.click(Id::new(INPUT)).expect("input should be visible");
        ui.tap_key(Named::ArrowDown);
        ui.click("Exited").expect("value should be listed");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Search(TagSearchAction::Focused),
                Message::Search(TagSearchAction::Highlight(Some(0))),
                Message::Search(TagSearchAction::Pick(1)),
            ]
        );
    }
}
