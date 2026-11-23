//! The project screen of Agents mode: the title row with the folder name,
//! Project statistics, Open the folder in a terminal tab and New session;
//! Project info with the path, the profiles as chips, the sessions and the
//! first and the last activity; the toolbar with the
//! search over the sessions and Refresh; the tags; the notice of a gone
//! folder; the sessions table, whose Profile cells open their profile;
//! and the line that says how to continue a session (spec Decision 6,
//! canvas "Project: sessions table"; spec 2026-11-19, Decision 6).

use iced::widget::{Column, Id, container, row, svg};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::data_table::DataTable;
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::components::key_value_list::KeyValueList;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::split_button::SplitButton;
use otty_ui_kit::components::tag_row::TagRow;
use otty_ui_kit::components::tag_search_field::TagSearchField;
use otty_ui_kit::components::toolbar::{Toolbar, ToolbarItem};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    ProjectInfoViewModel, ProjectProfileViewModel, ProjectViewModel,
};
use super::blocks;
use super::chips::{self, Chip};

/// Smallest width of the search field; it shrinks to it before Refresh
/// moves into the toolbar's "⋯" menu.
const SEARCH_MIN_WIDTH: f32 = 140.0;
/// Width of the search field when there is room (`width: 320px`).
const SEARCH_MAX_WIDTH: f32 = 320.0;
/// Widget id of the search field.
const SEARCH_ID: &str = "agents_sessions_search";
/// The id of the box around the button of the title row that opens the
/// statistics of the project; the tests of the binary press it by it.
pub(crate) const PROJECT_STATS_BUTTON_ID: &str = "agents_project_stats";

/// Render the project screen. It fills the height of the main area: the
/// table scrolls down on its own under its fixed header. The main area
/// keeps the screen at the minimum width of a screen and scrolls it
/// sideways in less room.
pub(crate) fn view<'a>(
    vm: ProjectViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let info = blocks::titled(
        i18n::t(Key::AgentsProjectInfo),
        info(vm.info.clone(), palette),
        palette,
    );
    let mut page = Column::new()
        .push(title_row(&vm, palette))
        .push(info)
        .push(toolbar(&vm, palette));
    if !vm.search.tags().is_empty() {
        page = page.push(
            TagRow::new(vm.search.tags(), &vm.keys, palette)
                .remove_label(i18n::t(Key::TagRemove))
                .clear_all_label(i18n::t(Key::TagsClearAll))
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

    let hint =
        small_text(i18n::t(Key::AgentsSessionsHint), palette.text_muted());
    let table = container(table(vm, palette)).height(Length::Fill);

    page.push(table)
        .push(hint)
        .spacing(spacing::SPACE_3)
        .padding([spacing::SPACE_4, spacing::SPACE_5])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The folder name, then Project statistics, Open the folder in a terminal
/// tab and New session. The path of the folder is in Project info only
/// (issue 38). The buttons keep their size at the right; the name ends
/// with "…" when it does not fit. The last two buttons do nothing while
/// the folder is gone.
fn title_row<'a>(
    vm: &ProjectViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let title = typography::TITLE;

    let heading = EllipsisText::new(
        vm.name.clone(),
        title.size(),
        fonts::UI_SEMIBOLD,
        palette.text_strong(),
    );

    // The history of a gone folder is still there, so its statistics open.
    let statistics = IconButton::new(
        svg::Handle::from_memory(icons::CHART),
        i18n::t(Key::AgentsProjectStatistics),
        palette,
    )
    .on_press_maybe(
        vm.can_show_stats.then_some(AgentsIntent::ShowProjectStats),
    );
    let statistics = container(statistics).id(Id::new(PROJECT_STATS_BUTTON_ID));
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

    row![heading, statistics, open_folder, new_session]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .into()
}

/// Project info: the path, the chips of the profiles, which open them,
/// then the sessions and the first and the last activity, in the look of
/// a list of facts.
fn info<'a>(
    vm: ProjectInfoViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let path = KeyValueList::new(palette)
        .entry(i18n::t(Key::AgentsFieldPath), vm.path);
    let mut info = Column::new().push(path).spacing(spacing::SPACE_3);
    if !vm.profiles.is_empty() {
        info = info.push(blocks::keyed(
            i18n::t(Key::AgentsSectionProfiles),
            profile_chips(vm.profiles, palette),
            palette,
        ));
    }

    info.push(blocks::facts(vm.facts, palette)).into()
}

/// The chips of the profiles with a session in the project: the agent
/// icon and the profile name; a press opens the profile.
fn profile_chips<'a>(
    profiles: Vec<ProjectProfileViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let chips = profiles
        .into_iter()
        .map(|chip| Chip {
            icon: chip.icon,
            name: chip.name,
            on_press: AgentsIntent::SelectProfile(chip.profile),
        })
        .collect();

    chips::view(chips, palette)
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

    Toolbar::new(i18n::t(Key::MoreActions), palette)
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
/// it, a right press and the "⋯" of a row open its menu at the pointer,
/// and the profile of a row opens its page.
fn table<'a>(
    vm: ProjectViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut table = DataTable::new(vm.columns, palette)
        .sorted_by(vm.sort)
        .on_sort(AgentsIntent::SortSessions)
        .selected(vm.selected)
        .on_row_press(AgentsIntent::SelectSession)
        .on_row_double_press(AgentsIntent::OpenSession)
        .on_row_right_press(|index, _| AgentsIntent::OpenSessionMenu(index))
        .on_row_menu(i18n::t(Key::AgentsSessionActions), |index, _| {
            AgentsIntent::OpenSessionMenu(index)
        })
        .on_link_press(|index, _, _| AgentsIntent::OpenSessionProfile(index));
    for cells in vm.rows {
        table = table.row(cells);
    }
    if let Some(message) = vm.empty_message {
        table = table.empty_message(message);
    }

    table.into()
}

#[cfg(test)]
mod tests {

    use iced::Point;
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::data_table::TableCell;
    use otty_ui_kit::components::tag_search::{
        Tag, TagSchema, TagSearch, TagSearchAction,
    };
    use otty_ui_kit::size;

    use super::*;
    use crate::model::{FactViewModel, ProjectViewModel, session_columns};
    use crate::session_sort::newest_first;
    use crate::sessions_table;
    use crate::testing::{codex, right_click};

    /// Width of the main area in the tests.
    const WIDTH: f32 = 900.0;
    /// Right edge of the title row and the toolbar: the main area less its
    /// padding.
    const RIGHT_END: f32 = WIDTH - spacing::SPACE_5;

    fn vm(search: &TagSearch, is_missing: bool) -> ProjectViewModel<'_> {
        ProjectViewModel {
            name: String::from("otty"),
            is_missing,
            can_start: !is_missing,
            is_new_session_menu_open: false,
            can_show_stats: true,
            info: info(),
            search,
            keys: sessions_table::keys(),
            suggestions: Vec::new(),
            columns: session_columns(),
            rows: vec![
                vec![
                    TableCell::plain("Fix the tabs"),
                    profile_cell("codex", "Codex · Work"),
                    TableCell::muted("Codex"),
                    TableCell::muted("5 min ago"),
                ],
                vec![
                    TableCell::plain("Add docs"),
                    profile_cell("claude-code", "Claude Code · Home"),
                    TableCell::muted("Claude Code"),
                    TableCell::muted("2 h ago"),
                ],
            ],
            selected: None,
            sort: newest_first(),
            empty_message: None,
        }
    }

    /// The Profile cell of a row: the agent `icon` and the profile `name`
    /// as a link.
    fn profile_cell(icon: &str, name: &str) -> TableCell {
        TableCell::icon_link(
            crate::testing::agent_icon(icon).svg_or(icons::MODE_AGENTS),
            name,
        )
    }

    /// Project info with the profile Codex · Main.
    fn info() -> ProjectInfoViewModel {
        ProjectInfoViewModel {
            path: String::from("~/work/otty"),
            profiles: vec![ProjectProfileViewModel {
                profile: codex(),
                icon: crate::testing::agent_icon("codex"),
                name: String::from("Codex · Main"),
            }],
            facts: vec![
                FactViewModel::new("Sessions", String::from("2")),
                FactViewModel::new(
                    "First activity",
                    String::from("Nov 12, 2026, 09:00"),
                ),
            ],
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

    /// Project statistics, before Open the folder in a terminal tab.
    fn statistics() -> f32 {
        open_folder() + size::CONTROL_MD + spacing::SPACE_3
    }

    #[test]
    fn the_screen_shows_the_folder_the_columns_the_sessions_and_the_hint() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));

        for label in [
            "otty",
            i18n::t(Key::AgentsColumnTitle),
            i18n::t(Key::AgentsColumnUpdated),
            "Fix the tabs",
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
        let sorted = sent(vm(&search, false), i18n::t(Key::AgentsAgent));

        assert_eq!(selected, vec![AgentsIntent::SelectSession(1)]);
        assert_eq!(sorted, vec![AgentsIntent::SortSessions(2)]);
    }

    /// Spec 2026-11-19, Decision 6: the Profile cell of a row is a link
    /// to the profile, and the row is not pressed.
    #[test]
    fn the_profile_of_a_row_opens_its_page() {
        let search = TagSearch::default();

        let opened = sent(vm(&search, false), "Claude Code · Home");

        assert_eq!(opened, vec![AgentsIntent::OpenSessionProfile(1)]);
    }

    /// Spec 2026-11-19, Decision 6: Project info has the path, the
    /// profiles as chips that open them, and the facts. Spec 2026-11-23,
    /// Decision 1: it has no accounts.
    #[test]
    fn project_info_shows_the_path_its_chips_and_its_facts() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));
        for label in [
            i18n::t(Key::AgentsProjectInfo),
            i18n::t(Key::AgentsFieldPath),
            "~/work/otty",
            i18n::t(Key::AgentsSectionProfiles),
            "Sessions",
            "Nov 12, 2026, 09:00",
        ] {
            assert!(ui.find(label).is_ok(), "{label:?} should show");
        }
        assert!(ui.find(i18n::t(Key::AgentsSectionAccounts)).is_err());
        drop(ui);

        let profile = sent(vm(&search, false), "Codex · Main");

        assert_eq!(profile, vec![AgentsIntent::SelectProfile(codex())]);
    }

    /// Owner, issue 38: the path of the folder shows once, in Project
    /// info, not under the title.
    #[test]
    fn the_path_shows_in_project_info_and_not_by_the_title() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm(&search, false), &themes));

        let title = ui.find("otty").expect("the name shows").bounds();
        let path = ui.find("~/work/otty").expect("the path shows").bounds();

        assert!(path.y > title.y + title.height, "{path:?} under {title:?}");
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
    fn project_statistics_open_also_for_a_gone_folder_and_only_when_on() {
        let search = TagSearch::default();

        let shown = sent_at(vm(&search, false), "otty", statistics());
        let gone = sent_at(vm(&search, true), "otty", statistics());
        let off = sent_at(
            ProjectViewModel {
                can_show_stats: false,
                ..vm(&search, false)
            },
            "otty",
            statistics(),
        );

        assert_eq!(shown, vec![AgentsIntent::ShowProjectStats]);
        assert_eq!(gone, vec![AgentsIntent::ShowProjectStats]);
        assert!(off.is_empty());
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
        ui.click(i18n::t(Key::TagsClearAll))
            .expect("clear all should show");

        let messages: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![AgentsIntent::SessionSearch(TagSearchAction::ClearTags)]
        );
    }

    /// In a narrow main area the name gives way and the buttons of the
    /// title row keep their place.
    #[test]
    fn in_a_narrow_main_area_the_buttons_of_the_title_row_stay() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let narrow = 300.0;
        let vm = ProjectViewModel {
            name: String::from("a-folder-with-a-rather-long-name"),
            ..vm(&search, false)
        };
        let screen = container(view(vm, themes.current().ui()))
            .width(narrow)
            .height(600.0);
        let mut ui = simulator(screen);

        let name = ui
            .find("a-folder-with-a-rather-long-name")
            .expect("the name shows")
            .bounds();
        let right_end = narrow - spacing::SPACE_5;
        ui.point_at(Point::new(right_end - open_folder(), name.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        assert!(
            name.x + name.width
                <= right_end - statistics() - size::CONTROL_MD / 2.0,
            "the name ends before the buttons: {name:?}"
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

    /// Issue 38: the whole table, its "⋯" column too, fits the smallest
    /// screen, so the table never scrolls sideways itself: in less room
    /// the main area keeps the page at that width and scrolls it sideways,
    /// as every screen, and nothing scrolls twice.
    #[test]
    fn the_whole_table_fits_the_smallest_screen() {
        let smallest = size::SCREEN_MIN;
        let menu_column = size::CONTROL_SM + 2.0 * spacing::SPACE_2;
        let updated_min = 96.0;
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let screen = container(view(vm(&search, false), themes.current().ui()))
            .width(smallest)
            .height(600.0);
        let mut ui = simulator(screen);

        let title = ui
            .find(i18n::t(Key::AgentsColumnUpdated))
            .expect("the column shows")
            .bounds();

        let column_end = title.x - spacing::SPACE_4 + updated_min;
        assert!(
            column_end <= smallest - spacing::SPACE_5 - menu_column + 0.5,
            "the Updated column ends at {column_end}"
        );
        assert!(updated_title_end(800.0) <= 800.0 - spacing::SPACE_5);
    }

    #[test]
    fn in_the_smallest_main_area_the_toolbar_keeps_the_search_and_refresh() {
        let search = TagSearch::default();
        let themes = ThemeManager::new();
        let smallest = size::SCREEN_MIN;
        let screen = container(view(vm(&search, false), themes.current().ui()))
            .width(smallest)
            .height(600.0);
        let mut ui = simulator(screen);
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
