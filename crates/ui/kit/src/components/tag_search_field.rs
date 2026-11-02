//! `TagSearchField`: the input of a [`TagSearch`] with its floating
//! suggestion list.
//!
//! Keys are handled by the field itself while its input has focus: typing, ↑,
//! ↓, Enter, Tab, Esc (while the list is open) and Backspace (in an empty
//! field) each send a [`TagSearchAction`], and so do a press on a suggestion,
//! a press outside the list and the input getting the focus.
//!
//! [`TagSearch`]: super::tag_search::TagSearch

use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::{
    Id, button, column, container, row, scrollable, text, text_input,
};
use iced::{Alignment, Border, Element, Length, Padding, Pixels};

use super::command_palette::title_pieces;
use super::field_chrome::input_style;
use super::floating_list::FloatingList;
use super::input_keys::InputKeys;
use super::tag_search::{
    Suggestion, SuggestionKind, TagSearch, TagSearchAction,
};
use super::text_field::{PADDING_X, PADDING_Y};
use super::{menu, scrollbar};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, typography};

/// Rows the list shows before it scrolls.
const LIST_ROWS: f32 = 8.0;
/// Tallest the list grows: its rows and its padding.
const LIST_MAX_HEIGHT: f32 = LIST_ROWS * size::ROW + 2.0 * menu::MENU_PADDING;
/// Text size of a suggestion, as in a menu item (`font-size: 12.5px`).
const ROW_TEXT_SIZE: f32 = 12.5;
/// Horizontal padding of a suggestion, as in a menu item (`padding: 0 10px`).
const ROW_PADDING_X: f32 = 10.0;
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
///     SearchRow, TagSchema, TagSearch, TagSearchAction,
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
/// let schema = TagSchema::new(&["name"], vec![SearchRow::new("web-1")]);
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
    on_action: Option<Box<dyn Fn(TagSearchAction) -> Message + 'a>>,
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
        self.on_action = Some(Box::new(on_action));
        self
    }
}

impl<'a, Message: Clone + 'a> From<TagSearchField<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: TagSearchField<'a, Message>) -> Self {
        let palette = value.palette;
        let send = |action: TagSearchAction| {
            value.on_action.as_ref().map(|on_action| on_action(action))
        };

        let is_open = value.search.is_open() && !value.suggestions.is_empty();
        let list =
            is_open.then(|| list_view(&value.suggestions, &send, &palette));
        let highlighted = value
            .suggestions
            .iter()
            .position(Suggestion::is_highlighted);

        let mut bindings = vec![
            (Named::ArrowUp, send(TagSearchAction::MoveUp)),
            (Named::ArrowDown, send(TagSearchAction::MoveDown)),
            (Named::Enter, send(TagSearchAction::Apply)),
            (Named::Tab, send(TagSearchAction::Apply)),
        ];
        if is_open {
            bindings.push((Named::Escape, send(TagSearchAction::Close)));
        }
        if value.search.input().is_empty() {
            bindings
                .push((Named::Backspace, send(TagSearchAction::RemoveLastTag)));
        }
        let focused = send(TagSearchAction::Focused);
        let close = send(TagSearchAction::Close);
        let on_input = value.on_action.map(|on_action| {
            move |text| on_action(TagSearchAction::InputChanged(text))
        });

        let style = typography::TERM;
        let mut input = text_input(&value.placeholder, value.search.input())
            .font(fonts::MONO)
            .size(style.size())
            .line_height(Pixels(style.line_height()))
            .padding(Padding::from([PADDING_Y, PADDING_X]))
            .on_input_maybe(on_input)
            .style(input_style(&palette, false));
        if let Some(id) = value.id {
            input = input.id(id);
        }

        let mut keys = InputKeys::new(input);
        for (key, message) in bindings {
            if let Some(message) = message {
                keys = keys.on_key(key, message);
            }
        }
        if let Some(message) = focused {
            keys = keys.on_focus(message);
        }

        let mut floating = FloatingList::new(keys).on_close(close);
        if let Some(list) = list {
            floating = floating.list(list, highlighted, size::ROW);
        }

        floating.into()
    }
}

/// Builds the floating surface with one row per suggestion. Past
/// [`LIST_ROWS`] rows it scrolls.
fn list_view<'a, Message: Clone + 'a>(
    suggestions: &[Suggestion],
    send: &dyn Fn(TagSearchAction) -> Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut rows = column![].width(Length::Fill);
    for (index, suggestion) in suggestions.iter().enumerate() {
        let on_press = send(TagSearchAction::Pick(index));
        rows = rows.push(row_view(suggestion, on_press, palette));
    }

    let background = palette.surface_overlay();
    let shadow = palette.shadow_popover();

    container(
        scrollable(rows)
            .width(Length::Fill)
            .direction(scrollbar::vertical())
            .style(scrollbar::style(palette)),
    )
    .width(Length::Fill)
    .max_height(LIST_MAX_HEIGHT)
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

/// Builds one suggestion row: the matched part in `text-strong` bold, as in
/// the command palette. A key is in the accent color and ends with `:`, so it
/// reads differently from a name or a value.
fn row_view<'a, Message: Clone + 'a>(
    suggestion: &Suggestion,
    on_press: Option<Message>,
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

    button(
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .clip(true),
    )
    .width(Length::Fill)
    .height(size::ROW)
    .padding(Padding::from([0.0, ROW_PADDING_X]))
    .on_press_maybe(on_press)
    .style(menu::item_style(palette, suggestion.is_highlighted()))
    .into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;
    use iced::widget::{Space, button, column, container, text};
    use iced::{Length, Point};
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::glyph;
    use crate::components::tag_search::{SearchRow, TagSchema};
    use crate::components::toolbar::{Toolbar, ToolbarItem};
    use crate::size;

    const KEYS: &[&str] = &["name", "status"];
    const INPUT: &str = "tag-search-test-input";

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Search(TagSearchAction),
        Under,
    }

    fn schema() -> TagSchema {
        TagSchema::new(
            KEYS,
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
    fn each_key_of_an_open_list_sends_its_action() {
        let cases = [
            (Named::ArrowDown, TagSearchAction::MoveDown),
            (Named::ArrowUp, TagSearchAction::MoveUp),
            (Named::Enter, TagSearchAction::Apply),
            (Named::Tab, TagSearchAction::Apply),
            (Named::Escape, TagSearchAction::Close),
        ];

        for (key, action) in cases {
            assert_eq!(
                tap_in_input(&typed("s"), key),
                vec![Message::Search(action)],
                "key {key:?}"
            );
        }
    }

    #[test]
    fn enter_and_tab_with_no_suggestion_still_send_apply() {
        for key in [Named::Enter, Named::Tab] {
            assert_eq!(
                tap_in_input(&closed("status:x"), key),
                vec![Message::Search(TagSearchAction::Apply)],
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
                Message::Search(TagSearchAction::MoveDown),
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

    /// A search over 20 rows with the value at `moves` highlighted.
    fn long_list(moves: usize) -> (TagSearch, TagSchema) {
        let rows = (0..20)
            .map(|index| {
                SearchRow::new("row")
                    .value("status", format!("value-{index:02}"))
            })
            .collect();
        let schema = TagSchema::new(KEYS, rows);
        let mut search = TagSearch::default();
        let _ = search.perform(
            TagSearchAction::InputChanged("status:".to_owned()),
            &schema,
        );
        for _ in 0..moves {
            let _ = search.perform(TagSearchAction::MoveDown, &schema);
        }

        (search, schema)
    }

    #[test]
    fn a_long_list_shows_about_eight_rows_then_scrolls() {
        let (search, schema) = long_list(0);
        let mut ui = simulator(view(&search, &schema));

        let eighth = ui.find("value-07").expect("row should be listed");
        let ninth = ui.find("value-08").expect("row should be listed");

        assert!(eighth.visible_bounds().is_some());
        assert!(ninth.visible_bounds().is_none(), "the ninth row scrolls");
    }

    #[test]
    fn the_highlighted_row_is_kept_in_view() {
        let (search, schema) = long_list(15);
        let mut ui = simulator(view(&search, &schema));

        let highlighted = ui.find("value-15").expect("row should be listed");
        let first = ui.find("value-00").expect("row should be listed");

        assert!(highlighted.visible_bounds().is_some());
        assert!(first.visible_bounds().is_none(), "the list has scrolled");
    }

    #[test]
    fn a_press_on_a_scrolled_row_picks_that_row() {
        let (search, schema) = long_list(15);
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
                Message::Search(TagSearchAction::MoveDown),
                Message::Search(TagSearchAction::Pick(1)),
            ]
        );
    }
}
