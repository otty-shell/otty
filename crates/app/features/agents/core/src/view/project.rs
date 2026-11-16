//! The project screen of Agents mode: the title row with the folder, Open
//! the folder in a terminal tab and New session; the toolbar with the
//! search over the sessions and Refresh; the tags; the notice of a gone
//! folder; the sessions table; and the line that says how to continue a
//! session (spec Decision 6, canvas "Project: sessions table").

use iced::widget::{Column, Id, container, row, svg, text};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::data_table::{DataTable, SortDirection};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::split_button::SplitButton;
use otty_ui_kit::components::tag_row::TagRow;
use otty_ui_kit::components::tag_search_field::TagSearchField;
use otty_ui_kit::components::toolbar::{Toolbar, ToolbarItem};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::ProjectViewModel;
use super::super::session_sort::SortOrder;

/// Smallest width of the search field; it shrinks to it before Refresh
/// moves into the toolbar's "⋯" menu.
const SEARCH_MIN_WIDTH: f32 = 140.0;
/// Width of the search field when there is room (`width: 320px`).
const SEARCH_MAX_WIDTH: f32 = 320.0;
/// Widget id of the search field.
const SEARCH_ID: &str = "agents_sessions_search";

/// Render the project screen. It fills the main area: the table scrolls on
/// its own under its fixed header.
pub(crate) fn view<'a>(
    vm: ProjectViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut page = Column::new()
        .push(title_row(&vm, palette))
        .push(toolbar(&vm, palette));
    if !vm.search.tags().is_empty() {
        page = page.push(
            TagRow::new(vm.search.tags(), &vm.keys, palette)
                .remove_label(i18n::t(Key::AgentsTagRemove))
                .clear_all_label(i18n::t(Key::AgentsTagsClearAll))
                .on_action(AgentsIntent::SessionSearch),
        );
    }
    if vm.is_missing {
        page = page.push(Notice::new(
            i18n::t(Key::AgentsFolderMissing),
            svg::Handle::from_memory(icons::NOTICE_ERROR),
            palette,
        ));
    }

    let hint = small_text(i18n::t(Key::AgentsSessionsHint), palette);
    let table = container(table(vm, palette)).height(Length::Fill);

    page.push(table)
        .push(hint)
        .spacing(spacing::SPACE_3)
        .padding([spacing::SPACE_4, spacing::SPACE_5])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The folder name and its path, then Open the folder in a terminal tab
/// and New session. The buttons keep their size at the right; in too
/// little room the path gives way first, and the name ends with "…" only
/// when it alone does not fit. Both buttons do nothing while the folder is
/// gone.
fn title_row<'a>(
    vm: &ProjectViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let title = typography::TITLE;
    let small = typography::SMALL;

    // The name takes its room first; the path gets what is left.
    let name = EllipsisText::new(
        vm.name.clone(),
        title.size(),
        fonts::UI_SEMIBOLD,
        palette.text_strong(),
    )
    .shrink(0.0);
    let path = EllipsisText::new(
        vm.path.clone(),
        small.size(),
        fonts::MONO,
        palette.text_muted(),
    );
    let heading = row![name, path]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    let open_folder = IconButton::new(
        svg::Handle::from_memory(icons::MODE_TERMINAL),
        i18n::t(Key::AgentsOpenFolderTerminal),
        palette,
    )
    .on_press_maybe(
        (!vm.is_missing).then_some(AgentsIntent::OpenProjectFolder),
    );
    let mut new_session = SplitButton::new(
        svg::Handle::from_memory(icons::PLUS),
        i18n::t(Key::AgentsNewSession),
        i18n::t(Key::AgentsChooseAgent),
        palette,
    )
    .menu_open(vm.is_new_session_menu_open);
    if vm.can_start {
        new_session = new_session
            .on_press(AgentsIntent::NewSession)
            .on_menu(AgentsIntent::OpenNewSessionMenu);
    }

    row![heading, open_folder, new_session]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .into()
}

/// The search field over the sessions, then Refresh; its label is the
/// tooltip. When the row is too narrow the field shrinks first, then
/// Refresh moves into the "⋯" menu.
fn toolbar<'a>(
    vm: &ProjectViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let search = vm.search;
    let suggestions = vm.suggestions.clone();
    let field_palette = palette.clone();

    Toolbar::new(i18n::t(Key::AgentsMoreActions), palette)
        .field(SEARCH_MIN_WIDTH, SEARCH_MAX_WIDTH, move || {
            TagSearchField::new(search, suggestions.clone(), &field_palette)
                .placeholder(i18n::t(Key::AgentsSessionsSearchPlaceholder))
                .id(Id::new(SEARCH_ID))
                .on_action(AgentsIntent::SessionSearch)
                .into()
        })
        .item(
            ToolbarItem::new(
                svg::Handle::from_memory(icons::REFRESH),
                i18n::t(Key::AgentsRefreshSessions),
            )
            .on_press(AgentsIntent::Refresh),
        )
        .into()
}

/// The sessions table: a press selects a row, a double press continues
/// it, a right press and the "⋯" of a row open its menu at the pointer.
fn table<'a>(
    vm: ProjectViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut table = DataTable::new(vm.columns, palette)
        .sorted_by(vm.sort.column(), sort_direction(vm.sort.order()))
        .on_sort(AgentsIntent::SortSessions)
        .selected(vm.selected)
        .on_row_press(AgentsIntent::SelectSession)
        .on_row_double_press(AgentsIntent::OpenSession)
        .on_row_right_press(|index, _| AgentsIntent::OpenSessionMenu(index))
        .on_row_menu(i18n::t(Key::AgentsSessionActions), |index, _| {
            AgentsIntent::OpenSessionMenu(index)
        });
    for cells in vm.rows {
        table = table.row(cells);
    }
    if let Some(message) = vm.empty_message {
        table = table.empty_message(message);
    }

    table.into()
}

/// The arrow of a sort order: up for ascending, down for descending.
fn sort_direction(order: SortOrder) -> SortDirection {
    match order {
        SortOrder::Ascending => SortDirection::Ascending,
        SortOrder::Descending => SortDirection::Descending,
    }
}

fn small_text<'a>(
    content: &'a str,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let small = typography::SMALL;

    text(content)
        .size(small.size())
        .line_height(iced::Pixels(small.line_height()))
        .font(fonts::UI)
        .color(palette.text_muted())
        .into()
}

#[cfg(test)]
mod tests {
    use iced::{Event, Point, mouse};
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::data_table::TableCell;
    use otty_ui_kit::components::tag_search::{
        Tag, TagSchema, TagSearch, TagSearchAction,
    };
    use otty_ui_kit::size;

    use super::*;
    use crate::model::{ProjectViewModel, session_columns};
    use crate::session_sort::SessionSort;
    use crate::sessions_table;

    /// Width of the main area in the tests.
    const WIDTH: f32 = 900.0;
    /// Right edge of the title row and the toolbar: the main area less its
    /// padding.
    const RIGHT_END: f32 = WIDTH - spacing::SPACE_5;

    fn vm(search: &TagSearch, is_missing: bool) -> ProjectViewModel<'_> {
        ProjectViewModel {
            name: String::from("otty"),
            path: String::from("/work/otty"),
            is_missing,
            can_start: !is_missing,
            is_new_session_menu_open: false,
            search,
            keys: sessions_table::keys(),
            suggestions: Vec::new(),
            columns: session_columns(),
            rows: vec![
                vec![
                    TableCell::plain("Fix the tabs"),
                    TableCell::plain("Codex"),
                    TableCell::muted("Codex"),
                    TableCell::mono("main"),
                    TableCell::muted("5 min ago"),
                ],
                vec![
                    TableCell::plain("Add docs"),
                    TableCell::plain("Claude Code"),
                    TableCell::muted("Claude Code"),
                    TableCell::muted(""),
                    TableCell::muted("2 h ago"),
                ],
            ],
            selected: None,
            sort: SessionSort::default(),
            empty_message: None,
        }
    }

    fn screen<'a>(
        vm: ProjectViewModel<'a>,
        themes: &ThemeManager,
    ) -> Element<'a, AgentsIntent> {
        container(view(vm, themes.current().ui()))
            .width(WIDTH)
            .height(600.0)
            .into()
    }

    /// What a press on the text `label` sends.
    fn sent(vm: ProjectViewModel<'_>, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm, &themes));

        ui.click(label).expect("the text should show");

        ui.into_messages().collect()
    }

    /// What a press `left` pixels left of [`RIGHT_END`], at the height of
    /// the text `beside`, sends.
    fn sent_at(
        vm: ProjectViewModel<'_>,
        beside: &str,
        left: f32,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm, &themes));
        let row = ui.find(beside).expect("the text should show").bounds();

        ui.point_at(Point::new(RIGHT_END - left, row.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    /// Where the buttons of the title row are, from its right edge: the
    /// chevron, the plus, then Open the folder in a terminal tab.
    fn chevron() -> f32 {
        size::SPLIT_CHEVRON / 2.0
    }

    fn plus() -> f32 {
        size::SPLIT_CHEVRON + size::CONTROL_SM / 2.0
    }

    fn open_folder() -> f32 {
        size::SPLIT_CHEVRON
            + size::CONTROL_SM
            + spacing::SPACE_3
            + size::CONTROL_MD / 2.0
    }

    fn right_click() -> [Event; 2] {
        [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
        ]
    }

    #[test]
    fn the_screen_shows_the_folder_the_columns_the_sessions_and_the_hint() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));

        for label in [
            "otty",
            "/work/otty",
            i18n::t(Key::AgentsColumnTitle),
            i18n::t(Key::AgentsColumnUpdated),
            "Fix the tabs",
            "main",
            "2 h ago",
            i18n::t(Key::AgentsSessionsHint),
        ] {
            assert!(ui.find(label).is_ok(), "{label:?} should show");
        }
        assert!(ui.find(i18n::t(Key::AgentsFolderMissing)).is_err());
    }

    #[test]
    fn a_row_press_selects_its_session_and_a_title_press_sorts() {
        let search = TagSearch::default();

        let selected = sent(vm(&search, false), "Add docs");
        let sorted = sent(vm(&search, false), i18n::t(Key::AgentsColumnAgent));

        assert_eq!(selected, vec![AgentsIntent::SelectSession(1)]);
        assert_eq!(sorted, vec![AgentsIntent::SortSessions(1)]);
    }

    #[test]
    fn a_double_click_on_a_row_opens_its_session() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));

        let row = ui.find("Fix the tabs").expect("the row should show");
        ui.point_at(row.bounds().center());
        let _ = ui.simulate(iced_test::simulator::click());
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert!(
            messages.contains(&AgentsIntent::OpenSession(0)),
            "got {messages:?}"
        );
    }

    #[test]
    fn a_right_click_on_a_row_opens_its_menu() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));

        let row = ui.find("Add docs").expect("the row should show");
        ui.point_at(row.bounds().center());
        let _ = ui.simulate(right_click());

        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(messages, vec![AgentsIntent::OpenSessionMenu(1)]);
    }

    #[test]
    fn the_title_row_buttons_open_the_folder_and_start_a_session() {
        let search = TagSearch::default();

        let folder = sent_at(vm(&search, false), "otty", open_folder());
        let started = sent_at(vm(&search, false), "otty", plus());
        let listed = sent_at(vm(&search, false), "otty", chevron());

        assert_eq!(folder, vec![AgentsIntent::OpenProjectFolder]);
        assert_eq!(started, vec![AgentsIntent::NewSession]);
        assert_eq!(listed, vec![AgentsIntent::OpenNewSessionMenu]);
    }

    #[test]
    fn refresh_at_the_end_of_the_toolbar_reads_the_sessions_again() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));
        let field = ui
            .find(Id::new(SEARCH_ID))
            .expect("the search field should show")
            .bounds();

        ui.point_at(Point::new(
            RIGHT_END - size::CONTROL_MD / 2.0,
            field.center_y(),
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(messages, vec![AgentsIntent::Refresh]);
    }

    /// Enter in the search over the table belongs to the search: the field
    /// takes it, so it never reaches the app as a session to continue.
    #[test]
    fn enter_in_the_search_is_taken_by_the_field() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));

        ui.click(Id::new(SEARCH_ID))
            .expect("the search field should show");
        let status = ui.tap_key(iced::keyboard::key::Named::Enter);

        assert_eq!(status, iced::event::Status::Captured);
        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert!(
            messages
                .contains(&AgentsIntent::SessionSearch(TagSearchAction::Apply)),
            "got {messages:?}"
        );
        assert!(!messages.contains(&AgentsIntent::ContinueSelected));
    }

    #[test]
    fn a_gone_folder_shows_the_notice_and_its_buttons_send_nothing() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, true), &themes));
        ui.find(i18n::t(Key::AgentsFolderMissing))
            .expect("the notice should show");
        drop(ui);

        let folder = sent_at(vm(&search, true), "otty", open_folder());
        let started = sent_at(vm(&search, true), "otty", plus());
        let listed = sent_at(vm(&search, true), "otty", chevron());

        assert!(folder.is_empty());
        assert!(started.is_empty());
        assert!(listed.is_empty());
    }

    #[test]
    fn a_tag_shows_under_the_toolbar_and_clear_all_sends_its_action() {
        let mut tagged = TagSearch::default();
        let schema = TagSchema::new(sessions_table::keys(), Vec::new());
        for action in [
            TagSearchAction::InputChanged(String::from("Agent:Codex")),
            TagSearchAction::Close,
            TagSearchAction::Apply,
        ] {
            let _ = tagged.perform(action, &schema);
        }
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&tagged, false), &themes));

        let label = Tag::new("agent", "Codex").label(&sessions_table::keys());
        ui.find(label.as_str()).expect("the tag should show");
        ui.click(i18n::t(Key::AgentsTagsClearAll))
            .expect("clear all should show");

        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![AgentsIntent::SessionSearch(TagSearchAction::ClearTags)]
        );
    }

    /// The screen in a main area `width` wide, with a long path.
    fn narrow_screen<'a>(
        search: &'a TagSearch,
        width: f32,
        themes: &ThemeManager,
    ) -> Element<'a, AgentsIntent> {
        let vm = ProjectViewModel {
            path: String::from("/home/user/work/clients/acme/otty"),
            ..vm(search, false)
        };

        container(view(vm, themes.current().ui()))
            .width(width)
            .height(600.0)
            .into()
    }

    #[test]
    fn in_a_narrow_main_area_the_path_gives_way_first_and_the_buttons_stay() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let narrow = 300.0;
        let mut wide = simulator(narrow_screen(&search, WIDTH, &themes));
        let wide_name = wide.find("otty").expect("the name shows").bounds();
        let wide_path = wide
            .find("/home/user/work/clients/acme/otty")
            .expect("the path shows")
            .bounds();
        drop(wide);

        let mut ui = simulator(narrow_screen(&search, narrow, &themes));
        let name = ui.find("otty").expect("the name shows").bounds();
        let path = ui
            .find("/home/user/work/clients/acme/otty")
            .expect("the path shows")
            .bounds();
        let right_end = narrow - spacing::SPACE_5;
        ui.point_at(Point::new(right_end - open_folder(), name.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        assert!((name.width - wide_name.width).abs() < 0.5, "{name:?}");
        assert!(path.width < wide_path.width, "{path:?} gives way");
        assert!(
            path.x + path.width
                <= right_end - open_folder() - size::CONTROL_MD / 2.0,
            "the path ends before the buttons: {path:?}"
        );
        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(messages, vec![AgentsIntent::OpenProjectFolder]);
    }

    /// The right edge of the Updated column title in a main area `width`
    /// wide.
    fn updated_title_end(width: f32) -> f32 {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let screen = container(view(vm(&search, false), themes.current().ui()))
            .width(width)
            .height(600.0);
        let mut ui = simulator(screen);
        let title = ui
            .find(i18n::t(Key::AgentsColumnUpdated))
            .expect("the column shows")
            .bounds();

        title.x + title.width
    }

    #[test]
    fn the_table_fits_a_medium_main_area_and_scrolls_sideways_in_the_smallest()
    {
        let medium = 800.0;
        let smallest = size::SCREEN_MIN;

        assert!(updated_title_end(medium) <= medium - spacing::SPACE_5);
        assert!(updated_title_end(smallest) > smallest - spacing::SPACE_5);
    }

    #[test]
    fn in_the_smallest_main_area_the_toolbar_keeps_the_search_and_refresh() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let smallest = size::SCREEN_MIN;
        let mut ui = simulator(narrow_screen(&search, smallest, &themes));
        let field = ui
            .find(Id::new(SEARCH_ID))
            .expect("the search field should show")
            .bounds();

        ui.point_at(Point::new(
            smallest - spacing::SPACE_5 - size::CONTROL_MD / 2.0,
            field.center_y(),
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        assert!(field.width >= SEARCH_MIN_WIDTH, "{field:?}");
        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(messages, vec![AgentsIntent::Refresh]);
    }
}
