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

use iced::Element;
use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::{Id, row, text};

use super::command_palette::title_pieces;
use super::field_chrome::mono_input;
use super::menu;
use super::suggestion_input::SuggestionInput;
use super::tag_search::{
    Suggestion, SuggestionKind, TagSearch, TagSearchAction,
};
use crate::fonts;
use crate::palette::UiPalette;
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

        let mut input = mono_input(
            &value.placeholder,
            value.search.input(),
            false,
            &palette,
        )
        .on_input_maybe(on_input)
        .on_submit_maybe(send(TagSearchAction::Apply));
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
            .size(menu::ITEM_TEXT_SIZE)
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
    use iced::{Length, Point};
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::glyph;
    use crate::components::tag_search::{SearchRow, TagKey, TagSchema};
    use crate::components::toolbar::{Toolbar, ToolbarItem};
    use crate::size;
    use crate::testing::live_app::LiveApp;

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

    /// What a running app keeps of the field: the search, which every
    /// message goes to, and its schema.
    struct Searching {
        search: TagSearch,
        schema: TagSchema,
    }

    impl Searching {
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

        /// The label of the first tag, with the shown name of its key.
        fn first_tag(&self) -> String {
            self.search.tags()[0].label(self.schema.keys())
        }
    }

    /// The field in a running app, over the rows of `schema`.
    fn searching(schema: TagSchema) -> LiveApp<Searching, Message> {
        let state = Searching {
            search: TagSearch::default(),
            schema,
        };

        LiveApp::new(state, searching_view, searching_update)
    }

    fn searching_view(state: &Searching) -> Element<'_, Message> {
        view(&state.search, &state.schema)
    }

    fn searching_update(state: &mut Searching, message: Message) {
        if let Message::Search(action) = message {
            let _ = state.search.perform(action, &state.schema);
        }
    }

    /// Presses the right end of the input, where a click puts the cursor
    /// at the end of the text.
    fn click_input(app: &mut LiveApp<Searching, Message>) {
        app.point_at(Point::new(290.0, 15.0));
        app.click();
    }

    /// Presses the middle of `content` where the app shows it.
    fn click_text(app: &mut LiveApp<Searching, Message>, content: &str) {
        let shown = app.find(content).expect("text should show").bounds();

        app.point_at(shown.center());
        app.click();
    }

    // Issue 41: the keys of the folder fields.

    #[test]
    fn at_first_no_row_is_lit() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("status:");

        assert_eq!(flow.state().rows(), vec!["Running", "Exited"]);
        assert_eq!(flow.state().highlighted(), None);
    }

    #[test]
    fn enter_picks_the_lit_row() {
        let mut flow = searching(schema());
        click_input(&mut flow);
        flow.type_text("status:");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::ArrowDown);

        flow.tap(Named::Enter);

        assert_eq!(flow.state().first_tag(), "status:Exited");
        assert_eq!(flow.state().input(), "");
    }

    #[test]
    fn enter_with_no_row_lit_applies_the_text() {
        let mut flow = searching(schema());
        click_input(&mut flow);
        flow.type_text("status:ru");

        flow.tap(Named::Enter);

        assert_eq!(flow.state().first_tag(), "status:ru");
        assert_eq!(flow.state().input(), "");
    }

    #[test]
    fn tab_with_no_row_lit_is_left_to_move_the_focus() {
        assert!(tap_in_input(&typed("sta"), Named::Tab).is_empty());
    }

    #[test]
    fn typing_clears_the_lit_row() {
        let mut flow = searching(schema());
        click_input(&mut flow);
        flow.type_text("s");
        flow.tap(Named::ArrowDown);

        flow.type_text("t");

        assert_eq!(flow.state().input(), "st");
        assert_eq!(flow.state().highlighted(), None);
    }

    #[test]
    fn typing_after_a_key_applied_by_enter_goes_after_the_colon() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("sta");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Enter);
        flow.type_text("r");

        assert_eq!(flow.state().input(), "status:r");
    }

    #[test]
    fn typing_after_a_key_applied_by_tab_goes_after_the_colon() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("sta");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);
        flow.type_text("r");

        assert_eq!(flow.state().input(), "status:r");
    }

    #[test]
    fn typing_after_a_key_picked_by_a_click_goes_after_the_colon() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("sta");
        click_text(&mut flow, "tus");
        flow.type_text("r");

        assert_eq!(flow.state().input(), "status:r");
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
        let mut flow = searching(russian_schema());

        click_input(&mut flow);
        flow.type_text("Сост");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);
        flow.type_text("раб");

        assert_eq!(flow.state().input(), "Состояние:раб");

        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);

        assert_eq!(flow.state().input(), "");
        assert_eq!(flow.state().first_tag(), "Состояние:Работает");
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
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("web");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::Tab);
        flow.type_text("x");

        assert_eq!(flow.state().input(), "web-1x");
    }

    #[test]
    fn enter_on_the_typed_text_keeps_it_and_typing_goes_on_after_it() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("web");
        flow.tap(Named::Enter);
        flow.type_text("x");

        assert_eq!(flow.state().input(), "webx");
        assert!(flow.state().search.tags().is_empty());
    }

    #[test]
    fn typing_after_a_name_picked_by_a_click_goes_after_the_name() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("web");
        click_text(&mut flow, "-1");
        flow.type_text("x");

        assert_eq!(flow.state().input(), "web-1x");
    }

    #[test]
    fn typing_in_the_middle_of_the_text_keeps_the_cursor_there() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("ac");
        flow.tap(Named::ArrowLeft);
        flow.type_text("b");
        flow.type_text("d");

        assert_eq!(flow.state().input(), "abdc");
    }

    #[test]
    fn arrows_of_the_list_do_not_move_the_cursor() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("s");
        flow.tap(Named::ArrowDown);
        flow.tap(Named::ArrowUp);
        flow.type_text("t");

        assert_eq!(flow.state().input(), "st");
    }

    #[test]
    fn a_value_applied_by_a_click_leaves_the_field_empty_and_focused() {
        let mut flow = searching(schema());

        click_input(&mut flow);
        flow.type_text("status:");
        click_text(&mut flow, "Exited");
        flow.type_text("s");

        assert_eq!(flow.state().input(), "s");
        assert_eq!(flow.state().search.tags().len(), 1);
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
