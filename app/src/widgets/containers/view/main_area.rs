//! The Containers mode main area: the header (the title row, the toolbar with the
//! tag search and the create, Prune and Refresh buttons, then the tags) over
//! the table of the open list, or a card while no engine is known.

use iced::widget::text::Wrapping;
use iced::widget::{Id, column, container, svg, text};
use iced::{Alignment, Element, Length, Padding, Point};
use otty_ui_kit::components::data_table::{DataTable, SortDirection};
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::tag_row::TagRow;
use otty_ui_kit::components::tag_search_field::TagSearchField;
use otty_ui_kit::components::toolbar::{Toolbar, ToolbarItem};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::actions::event::ActionsIntent;
use super::super::event::ContainersIntent;
use super::super::model::{MainAreaViewModel, TableViewModel};
use super::super::resource_rows::PORTS_COLUMN;
use super::super::sort::SortOrder;
use super::super::types::ResourceKind;
use super::no_engine;
use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// Height of the title row: as tall as a button.
const TITLE_ROW_HEIGHT: f32 = size::CONTROL_MD;
/// Smallest width of the search field; it shrinks to it before any button
/// moves into the toolbar's "⋯" menu.
const SEARCH_MIN_WIDTH: f32 = 140.0;
/// Width of the search field when there is room; a typed `key:value` needs
/// more than a name did.
const SEARCH_MAX_WIDTH: f32 = 320.0;
/// Widget id of the search field.
const SEARCH_ID: &str = "containers_search";

/// Props for the Containers main area.
pub(crate) struct MainAreaProps<'a> {
    pub(crate) vm: MainAreaViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the main area.
pub(crate) fn view(props: MainAreaProps<'_>) -> Element<'_, ContainersIntent> {
    let palette = props.theme.theme.ui();

    match props.vm {
        MainAreaViewModel::Table(table) => {
            let page = table.page.clone();

            table_view(table, palette).map(move |intent| {
                ContainersIntent::FromPage {
                    page: page.clone(),
                    intent: Box::new(intent),
                }
            })
        },
        MainAreaViewModel::Searching => {
            card(i18n::t(Key::ContainersSearching), palette)
        },
        MainAreaViewModel::NoEngine(vm) => no_engine::view(vm, palette),
    }
}

/// The header, the error notice of a failed list and the table.
fn table_view<'a>(
    vm: TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let header = header(&vm, palette);

    let mut content = column![header].width(Length::Fill).height(Length::Fill);
    if let Some(error) = vm.error {
        let notice = Notice::new(
            i18n::containers_list_failed(error),
            svg::Handle::from_memory(icons::NOTICE_ERROR),
            palette,
        );
        content = content.push(notice_row(notice));
    }

    let mut table = DataTable::new(vm.columns, palette)
        .sorted_by(vm.sort.column(), sort_direction(vm.sort.order()))
        .on_sort(ContainersIntent::SortBy)
        .selected(vm.selected)
        .on_row_press(ContainersIntent::SelectRow)
        .on_link_press(|row, column, link| match column {
            PORTS_COLUMN => ContainersIntent::OpenPortLink { row, link },
            _ => ContainersIntent::OpenComposeProject(row),
        })
        .on_row_right_press(|index, position| ContainersIntent::OpenRowMenu {
            index,
            position,
        })
        .on_row_menu(i18n::t(Key::ContainersMoreActions), |index, corner| {
            // The table reports the button's bottom-left corner. The "⋯"
            // column is the last one, so the menu would open to the left of
            // that corner, in the middle of the table. The bottom-right
            // corner makes it open under the button, ending at its edge.
            let position = Point::new(corner.x + size::CONTROL_SM, corner.y);

            ContainersIntent::RowMenuPressed { index, position }
        });
    for (position, cells) in vm.rows.into_iter().enumerate() {
        table = if vm.rows_without_actions.contains(&position) {
            table.row_without_menu(cells)
        } else {
            table.row(cells)
        };
    }
    if let Some(message) = vm.empty_message {
        table = table.empty_message(message);
    }

    content.push(table).into()
}

/// The arrow of a sort order: up for ascending, down for descending.
fn sort_direction(order: SortOrder) -> SortDirection {
    match order {
        SortOrder::Ascending => SortDirection::Ascending,
        SortOrder::Descending => SortDirection::Descending,
    }
}

/// A notice under the header row, inside the page padding.
fn notice_row<'a>(
    notice: Notice<ContainersIntent>,
) -> Element<'a, ContainersIntent> {
    container(notice)
        .padding(Padding {
            top: 0.0,
            right: spacing::SPACE_5,
            bottom: spacing::SPACE_4,
            left: spacing::SPACE_5,
        })
        .into()
}

/// The header above the table: the title row, the toolbar and, when the table
/// has tags, the row of tags.
fn header<'a>(
    vm: &TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let mut header = column![title_row(vm, palette), toolbar(vm, palette)]
        .spacing(spacing::SPACE_3)
        .padding(Padding::from([spacing::SPACE_4, spacing::SPACE_5]));
    if !vm.search.tags().is_empty() {
        header = header.push(
            TagRow::new(vm.search.tags(), &vm.keys, palette)
                .remove_label(i18n::t(Key::ContainersTagRemove))
                .clear_all_label(i18n::t(Key::ContainersTagsClearAll))
                .on_action(ContainersIntent::TagSearch),
        );
    }

    header.into()
}

/// The title.
fn title_row<'a>(
    vm: &TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let heading = typography::HEADING;

    let title = text(i18n::t(vm.resource.title_key()))
        .size(heading.size())
        .font(fonts::UI_SEMIBOLD)
        .color(palette.text_strong())
        .wrapping(Wrapping::None);

    container(title)
        .height(TITLE_ROW_HEIGHT)
        .align_y(Alignment::Center)
        .into()
}

/// The search field, then the create, Prune and Refresh icon buttons; their
/// labels are the tooltips. When the row is too narrow the field shrinks
/// first, then the last buttons move into the "⋯" menu.
fn toolbar<'a>(
    vm: &TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let search = vm.search;
    let suggestions = vm.suggestions.clone();
    let field_palette = palette.clone();

    let create = ToolbarItem::new(
        svg::Handle::from_memory(create_icon(vm.resource)),
        i18n::t(vm.resource.create_key()),
    )
    .on_press_maybe(vm.can_create.then_some(ContainersIntent::OpenCreate));
    let mut toolbar =
        Toolbar::new(i18n::t(Key::ContainersMoreActions), palette)
            .field(SEARCH_MIN_WIDTH, SEARCH_MAX_WIDTH, move || {
                TagSearchField::new(search, suggestions.clone(), &field_palette)
                    .placeholder(i18n::t(Key::ContainersSearchPlaceholder))
                    .id(Id::new(SEARCH_ID))
                    .on_action(ContainersIntent::TagSearch)
                    .into()
            })
            .item(create);
    if let Some(prune) = &vm.prune {
        toolbar = toolbar.item(
            ToolbarItem::new(
                svg::Handle::from_memory(icons::BROOM),
                prune.label,
            )
            .on_press_maybe(
                prune
                    .is_enabled
                    .then_some(ContainersIntent::Actions(ActionsIntent::Prune)),
            ),
        );
    }

    toolbar
        .item(
            ToolbarItem::new(
                svg::Handle::from_memory(icons::REFRESH),
                i18n::t(Key::ContainersRefresh),
            )
            .on_press(ContainersIntent::Refresh),
        )
        .into()
}

/// The icon of the create button of a list: a download arrow for Pull
/// image, a plus for the others.
fn create_icon(resource: ResourceKind) -> &'static [u8] {
    match resource {
        ResourceKind::Images => icons::PULL,
        _ => icons::PLUS,
    }
}

/// A card with the Containers icon and `description`, while the search runs.
fn card<'a>(
    description: &'static str,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let icon_color = palette.text_muted();

    let icon = svg(svg::Handle::from_memory(icons::MODE_CONTAINERS))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| svg::Style {
            color: Some(icon_color),
        });

    EmptyCard::new(i18n::t(Key::ModeContainers), description, icon, palette)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;
    use iced_test::simulator;
    use otty_containers::{Container, EngineKind, fixtures};
    use otty_ui_kit::components::data_table::{
        CellPart, TableCell, TableColumn,
    };
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::components::status_badge::StatusTone;
    use otty_ui_kit::components::tag_search::{
        Suggestion, TagSearch, TagSearchAction,
    };

    use super::*;
    use crate::i18n::Locale;
    use crate::testing::temp_dir::TempDir;
    use crate::theme::ThemeManager;
    use crate::widgets::containers::actions::PruneViewModel;
    use crate::widgets::containers::model::NoEngineViewModel;
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::resource_rows;
    use crate::widgets::containers::sort::{TableSort, TableSorts};
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::table_search::{keys, keys_in};
    use crate::widgets::containers::types::{
        LoadedLists, NoEngineCase, Place, ResourceKind,
    };

    const NOW: i64 = 1_793_557_960;

    /// A search with nothing typed and no tags.
    static EMPTY_SEARCH: std::sync::LazyLock<TagSearch> =
        std::sync::LazyLock::new(TagSearch::default);
    /// The page the tables of these tests belong to.
    static PAGE: std::sync::LazyLock<Place> =
        std::sync::LazyLock::new(containers_page);

    /// The containers page of the fixture Docker engine.
    fn containers_page() -> Place {
        Place::new(
            fixtures::engine(EngineKind::Docker).address().clone(),
            ResourceKind::Containers,
        )
    }

    /// What the main area sent, without the page each table intent carries.
    fn sent(
        ui: iced_test::Simulator<'_, ContainersIntent>,
    ) -> Vec<ContainersIntent> {
        ui.into_messages()
            .map(|intent| match intent {
                ContainersIntent::FromPage { intent, .. } => *intent,
                other => other,
            })
            .collect()
    }

    fn table<'a>(rows: Vec<Vec<TableCell>>) -> MainAreaViewModel<'a> {
        MainAreaViewModel::Table(TableViewModel {
            page: &PAGE,
            resource: ResourceKind::Containers,
            search: &EMPTY_SEARCH,
            keys: keys(ResourceKind::Containers),
            suggestions: Vec::new(),
            sort: TableSort::default_for(ResourceKind::Containers),
            columns: vec![
                TableColumn::new("Name", 3).sortable(true),
                TableColumn::new("Status", 2).sortable(true),
            ],
            rows,
            rows_without_actions: Vec::new(),
            selected: Some(0),
            empty_message: None,
            error: None,
            prune: Some(PruneViewModel {
                label: i18n::t(Key::ContainersPruneStopped),
                is_enabled: true,
            }),
            can_create: false,
        })
    }

    fn with<'a>(
        change: impl FnOnce(TableViewModel<'a>) -> TableViewModel<'a>,
    ) -> MainAreaViewModel<'a> {
        match table(rows()) {
            MainAreaViewModel::Table(table) => {
                MainAreaViewModel::Table(change(table))
            },
            other => other,
        }
    }

    fn rows() -> Vec<Vec<TableCell>> {
        vec![
            vec![
                TableCell::mono("web-1"),
                TableCell::badge("Running", StatusTone::Ok),
            ],
            vec![
                TableCell::mono("db-1"),
                TableCell::badge("Exited (3)", StatusTone::Error),
            ],
        ]
    }

    #[test]
    fn a_press_on_a_table_row_selects_it() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click("db-1").expect("second row should be visible");

        let intents = sent(ui);
        assert_eq!(intents, vec![ContainersIntent::SelectRow(1)]);
    }

    #[test]
    fn a_press_on_a_column_title_sorts_by_that_column() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click("Name").expect("title should be visible");

        let intents = sent(ui);
        assert_eq!(intents, vec![ContainersIntent::SortBy(0)]);
    }

    #[test]
    fn a_press_on_a_compose_link_opens_that_project() {
        let themes = ThemeManager::new();
        let rows = vec![
            vec![TableCell::mono("web-1"), TableCell::link("otty-site")],
            vec![TableCell::mono("db-1"), TableCell::muted("")],
        ];
        let mut ui = simulator(view(MainAreaProps {
            vm: with(|mut table| {
                table.columns = vec![
                    TableColumn::new("Name", 3),
                    TableColumn::new("Compose", 2),
                ];
                table.rows = rows;
                table
            }),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click("otty-site").expect("link should be visible");

        let intents = sent(ui);
        assert_eq!(intents, vec![ContainersIntent::OpenComposeProject(0)]);
    }

    #[test]
    fn a_press_on_a_port_link_opens_that_port_without_selecting_the_row() {
        let themes = ThemeManager::new();
        let ports = TableCell::parts(vec![
            CellPart::link("8080:80/tcp"),
            CellPart::text(", "),
            CellPart::link("5432/tcp"),
        ]);
        let mut columns = vec![TableColumn::new("Name", 1); PORTS_COLUMN];
        columns.push(TableColumn::new("Ports", 4));
        let mut row = vec![TableCell::mono("web-1"); PORTS_COLUMN];
        row.push(ports);
        let make = || {
            let (columns, row) = (columns.clone(), row.clone());
            view(MainAreaProps {
                vm: with(|mut table| {
                    table.columns = columns;
                    table.rows = vec![row];
                    table
                }),
                theme: ThemeProps::new(themes.current()),
            })
        };

        let mut first = simulator(make());
        first.click("8080:80/tcp").expect("link should be visible");
        let mut second = simulator(make());
        second.click("5432/tcp").expect("link should be visible");

        let first = sent(first);
        let second = sent(second);
        assert_eq!(
            first,
            vec![ContainersIntent::OpenPortLink { row: 0, link: 0 }]
        );
        assert_eq!(
            second,
            vec![ContainersIntent::OpenPortLink { row: 0, link: 1 }]
        );
    }

    #[test]
    fn the_ports_title_sorts_and_a_port_link_under_it_still_opens_its_port() {
        let themes = ThemeManager::new();
        let ports = TableCell::parts(vec![CellPart::link("8080:80/tcp")]);
        let mut row = vec![TableCell::mono("web-1"); PORTS_COLUMN];
        row.push(ports);
        row.push(TableCell::muted("1 d"));
        let make = |sort: TableSort, width: f32| {
            let row = row.clone();
            area(
                with(move |mut table| {
                    table.columns = resource_rows::columns_in(
                        Locale::En,
                        ResourceKind::Containers,
                    );
                    table.rows = vec![row];
                    table.sort = sort;
                    table
                }),
                width,
                ThemeProps::new(themes.current()),
            )
        };
        let mut sorts = TableSorts::default();
        sorts.press(&PAGE, PORTS_COLUMN);
        let by_ports = sorts.get(&PAGE);
        let by_status = TableSort::default_for(ResourceKind::Containers);
        // The narrowest area that still shows the column without scrolling
        // sideways: only the minimums and the row menu fit.
        let narrow = 572.0 + ROW_MENU_COLUMN;

        for (sort, width) in [
            (by_status, WIDE),
            (by_status, narrow),
            (by_ports, WIDE),
            (by_ports, narrow),
        ] {
            let mut title = simulator(make(sort, width));
            title.click("Ports").expect("title should be visible");
            assert_eq!(
                sent(title),
                vec![ContainersIntent::SortBy(PORTS_COLUMN)],
                "{sort:?} at {width}"
            );

            let mut link = simulator(make(sort, width));
            link.click("8080:80/tcp").expect("link should be visible");
            assert_eq!(
                sent(link),
                vec![ContainersIntent::OpenPortLink { row: 0, link: 0 }],
                "{sort:?} at {width}"
            );
        }
    }

    /// Width of a wide main area in the tests.
    const WIDE: f32 = 800.0;
    /// Width of a main area whose toolbar holds the filter, the create
    /// button and the "⋯" button with Prune and Refresh.
    const NARROW: f32 = 250.0;
    /// Width of a toolbar button and the gap between buttons.
    const BUTTON: f32 = otty_ui_kit::size::CONTROL_MD;
    /// Width of the table's last column with the row "⋯" buttons.
    const ROW_MENU_COLUMN: f32 = 36.0;

    /// The main area of `vm`, `width` wide.
    fn area<'a>(
        vm: MainAreaViewModel<'a>,
        width: f32,
        theme: ThemeProps<'a>,
    ) -> Element<'a, ContainersIntent> {
        container(view(MainAreaProps { vm, theme }))
            .width(width)
            .into()
    }

    /// Center of the toolbar button `from_end` places before the last one
    /// (0: Refresh, 1: Prune, 2: the create button), in a main area `width`
    /// wide. The buttons are centered on the filter's line.
    fn toolbar_button(
        ui: &mut iced_test::Simulator<'_, ContainersIntent>,
        width: f32,
        from_end: usize,
    ) -> iced::Point {
        let field = ui
            .find(Id::new(SEARCH_ID))
            .expect("field should be visible")
            .bounds();

        iced::Point::new(
            width
                - spacing::SPACE_5
                - BUTTON / 2.0
                - from_end as f32 * (BUTTON + spacing::SPACE_2),
            field.center_y(),
        )
    }

    fn click_at(
        ui: &mut iced_test::Simulator<'_, ContainersIntent>,
        point: iced::Point,
    ) {
        ui.point_at(point);
        let _ = ui.simulate(iced_test::simulator::click());
    }

    /// A search with `tags`.
    fn tagged(tags: &[(&'static str, &str)]) -> TagSearch {
        let mut search = TagSearch::default();
        for (key, value) in tags {
            let _ = search.set_tag(key, value);
        }

        search
    }

    #[test]
    fn title_search_tags_and_table_come_in_that_order() {
        let themes = ThemeManager::new();
        let search = tagged(&[("compose", "otty-site")]);
        let vm = with(|table| TableViewModel {
            search: &search,
            ..table
        });
        let mut ui =
            simulator(area(vm, WIDE, ThemeProps::new(themes.current())));

        let title = ui
            .find(i18n::t(Key::ContainersContainers))
            .expect("title should show")
            .bounds();
        let field = ui
            .find(Id::new(SEARCH_ID))
            .expect("field should show")
            .bounds();
        let tag = ui
            .find("Compose:otty-site")
            .expect("tag should show")
            .bounds();
        let table = ui.find("Name").expect("header should show").bounds();

        assert!(title.y + title.height <= field.y, "title above the field");
        assert!(field.y + field.height <= tag.y, "field above the tag");
        assert!(tag.y + tag.height <= table.y, "tag above the table");
        assert!(ui.find("Docker 29.9.1").is_err(), "the engine chip is gone");
        assert!(
            ui.find(i18n::t(Key::ThisComputerNote)).is_err(),
            "no this computer note"
        );
    }

    #[test]
    fn the_toolbar_buttons_are_icons_whose_labels_are_not_shown_as_text() {
        let themes = ThemeManager::new();
        let mut ui = simulator(area(
            table(rows()),
            WIDE,
            ThemeProps::new(themes.current()),
        ));

        assert!(ui.find(i18n::t(Key::ContainersRunContainer)).is_err());
        assert!(ui.find(i18n::t(Key::ContainersPruneStopped)).is_err());
    }

    #[test]
    fn a_disabled_create_button_sends_nothing() {
        let themes = ThemeManager::new();
        let mut ui = simulator(area(
            table(rows()),
            WIDE,
            ThemeProps::new(themes.current()),
        ));

        let create = toolbar_button(&mut ui, WIDE, 2);
        click_at(&mut ui, create);

        let intents = sent(ui);
        assert!(intents.is_empty(), "the create button is disabled");
    }

    #[test]
    fn the_refresh_button_loads_the_lists_again() {
        let themes = ThemeManager::new();
        let mut ui = simulator(area(
            table(rows()),
            WIDE,
            ThemeProps::new(themes.current()),
        ));

        let refresh = toolbar_button(&mut ui, WIDE, 0);
        click_at(&mut ui, refresh);

        let intents = sent(ui);
        assert_eq!(intents, vec![ContainersIntent::Refresh]);
    }

    #[test]
    fn at_a_narrow_width_prune_and_refresh_are_in_the_more_menu() {
        let themes = ThemeManager::new();
        let mut ui = simulator(area(
            table(rows()),
            NARROW,
            ThemeProps::new(themes.current()),
        ));
        assert!(ui.find(i18n::t(Key::ContainersPruneStopped)).is_err());

        let more = toolbar_button(&mut ui, NARROW, 0);
        click_at(&mut ui, more);
        ui.click(i18n::t(Key::ContainersPruneStopped))
            .expect("prune should be in the menu");
        click_at(&mut ui, more);
        ui.click(i18n::t(Key::ContainersRefresh))
            .expect("refresh should be in the menu");

        let intents = sent(ui);
        assert_eq!(
            intents,
            vec![
                ContainersIntent::Actions(ActionsIntent::Prune),
                ContainersIntent::Refresh
            ]
        );
    }

    #[test]
    fn at_a_narrow_width_the_create_button_and_the_filter_still_work() {
        let themes = ThemeManager::new();
        let vm = with(|table| TableViewModel {
            resource: ResourceKind::Volumes,
            can_create: true,
            ..table
        });
        let mut ui =
            simulator(area(vm, NARROW, ThemeProps::new(themes.current())));

        let create = toolbar_button(&mut ui, NARROW, 1);
        click_at(&mut ui, create);
        ui.click(Id::new(SEARCH_ID))
            .expect("field should be visible");
        let _ = ui.typewrite("w");

        let intents = sent(ui);
        assert_eq!(
            intents,
            vec![
                ContainersIntent::OpenCreate,
                ContainersIntent::TagSearch(TagSearchAction::Focused),
                ContainersIntent::TagSearch(TagSearchAction::InputChanged(
                    String::from("w")
                )),
            ]
        );
    }

    #[test]
    fn the_row_menu_button_opens_that_rows_menu() {
        let themes = ThemeManager::new();
        let mut ui = simulator(area(
            table(rows()),
            WIDE,
            ThemeProps::new(themes.current()),
        ));
        let cell = ui.find("db-1").expect("second row should be visible");
        let row_center = cell.bounds().center_y();

        click_at(
            &mut ui,
            iced::Point::new(WIDE - ROW_MENU_COLUMN / 2.0, row_center),
        );

        let intents = sent(ui);
        assert!(
            matches!(
                intents.as_slice(),
                [ContainersIntent::RowMenuPressed { index: 1, .. }]
            ),
            "got {intents:?}"
        );
    }

    #[test]
    fn a_row_without_actions_has_no_menu_button() {
        let themes = ThemeManager::new();
        let vm = with(|table| TableViewModel {
            rows_without_actions: vec![1],
            ..table
        });
        let mut ui =
            simulator(area(vm, WIDE, ThemeProps::new(themes.current())));
        let cell = ui.find("db-1").expect("second row should be visible");
        let row_center = cell.bounds().center_y();

        click_at(
            &mut ui,
            iced::Point::new(WIDE - ROW_MENU_COLUMN / 2.0, row_center),
        );

        let intents = sent(ui);
        assert_eq!(intents, vec![ContainersIntent::SelectRow(1)]);
    }

    #[test]
    fn the_cross_of_a_tag_and_clear_all_send_their_actions() {
        let themes = ThemeManager::new();
        let search = tagged(&[("compose", "otty-site"), ("status", "Running")]);
        let vm = with(|table| TableViewModel {
            search: &search,
            ..table
        });
        let mut ui =
            simulator(area(vm, WIDE, ThemeProps::new(themes.current())));
        let tag = ui.find("Status:Running").expect("tag should show").bounds();

        // The cross sits one gap right of the chip label.
        click_at(
            &mut ui,
            iced::Point::new(
                tag.x + tag.width + spacing::SPACE_2 + 12.0,
                tag.y + tag.height / 2.0,
            ),
        );
        ui.click(i18n::t(Key::ContainersTagsClearAll))
            .expect("clear all should show");

        let intents = sent(ui);
        assert_eq!(
            intents,
            vec![
                ContainersIntent::TagSearch(TagSearchAction::RemoveTag(1)),
                ContainersIntent::TagSearch(TagSearchAction::ClearTags),
            ]
        );
    }

    #[test]
    fn a_tag_shows_the_header_of_the_keys_the_model_gives() {
        let themes = ThemeManager::new();
        let search = tagged(&[("status", "Running")]);
        let vm = with(|table| TableViewModel {
            search: &search,
            keys: keys_in(Locale::Ru, ResourceKind::Containers),
            ..table
        });
        let mut ui =
            simulator(area(vm, WIDE, ThemeProps::new(themes.current())));

        assert!(ui.find("Состояние:Running").is_ok());
        assert!(ui.find("Status:Running").is_err());
        assert!(ui.find("status:Running").is_err());
    }

    #[test]
    fn without_tags_there_is_no_tags_row() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::ContainersTagsClearAll)).is_err());
    }

    #[test]
    fn typing_in_the_search_field_sends_the_text() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(Id::new(SEARCH_ID))
            .expect("field should be visible");
        let _ = ui.typewrite("w");

        let intents = sent(ui);
        assert_eq!(
            intents,
            vec![
                ContainersIntent::TagSearch(TagSearchAction::Focused),
                ContainersIntent::TagSearch(TagSearchAction::InputChanged(
                    String::from("w")
                )),
            ]
        );
    }

    // The whole flow, over the real state and reducer.

    /// Containers mode with the fixture engine and its lists.
    fn loaded_state() -> ContainersState {
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
                engine: docker.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );

        state
    }

    /// Draws the main area of `state`, does `step` in it and reduces what it
    /// sent, as the app does between frames.
    fn in_frame(
        state: &mut ContainersState,
        step: impl FnOnce(&mut iced_test::Simulator<'_, ContainersIntent>),
    ) {
        let themes = ThemeManager::new();
        let intents: Vec<ContainersIntent> = {
            let vm = MainAreaViewModel::from_state(state, NOW);
            let mut ui =
                simulator(area(vm, WIDE, ThemeProps::new(themes.current())));
            step(&mut ui);
            ui.into_messages().collect()
        };

        for intent in intents {
            let _ = reduce(state, intent);
        }
    }

    /// Clicks the field and types one character per frame, as a person does.
    fn type_in_field(state: &mut ContainersState, text: &str) {
        for character in text.chars() {
            in_frame(state, |ui| {
                ui.click(Id::new(SEARCH_ID))
                    .expect("field should be visible");
                let _ = ui.typewrite(character.to_string().as_str());
            });
        }
    }

    fn tap_in_field(state: &mut ContainersState, key: Named) {
        in_frame(state, |ui| {
            ui.click(Id::new(SEARCH_ID))
                .expect("field should be visible");
            ui.tap_key(key);
        });
    }

    /// The tags of the containers table as the user reads them.
    fn tag_labels(search: &TagSearch) -> Vec<String> {
        let keys = keys(ResourceKind::Containers);

        search.tags().iter().map(|tag| tag.label(&keys)).collect()
    }

    /// The rows the main area of `state` shows.
    fn row_count(state: &ContainersState) -> usize {
        match MainAreaViewModel::from_state(state, NOW) {
            MainAreaViewModel::Table(table) => table.rows.len(),
            _ => 0,
        }
    }

    #[test]
    fn typing_a_key_and_a_value_with_tab_gives_a_tag_and_fewer_rows() {
        let mut state = loaded_state();
        let before = row_count(&state);
        let search = |state: &ContainersState| {
            state.table_search(&containers_page()).clone()
        };

        type_in_field(&mut state, "sta");
        assert_eq!(row_count(&state), 0, "plain text filters by name");
        tap_in_field(&mut state, Named::Tab);
        assert_eq!(search(&state).input(), "Status:");
        assert_eq!(
            row_count(&state),
            before,
            "a half typed tag filters nothing"
        );

        type_in_field(&mut state, "run");
        tap_in_field(&mut state, Named::Tab);

        let labels = tag_labels(&search(&state));
        assert_eq!(
            labels,
            vec![format!("Status:{}", i18n::t(Key::ContainersStateRunning))]
        );
        assert_eq!(search(&state).input(), "");
        assert_eq!(row_count(&state), 2);
        assert!(before > 2);
    }

    #[test]
    fn enter_on_a_typed_key_and_text_adds_that_text_as_a_tag() {
        let mut state = loaded_state();
        let before = row_count(&state);

        type_in_field(&mut state, "Status:ru");
        tap_in_field(&mut state, Named::Enter);

        let search = state.table_search(&containers_page());
        assert_eq!(tag_labels(search), vec!["Status:ru"]);
        assert_eq!(search.input(), "");
        assert_eq!(row_count(&state), 2, "only the running containers");
        assert!(before > 2);
    }

    #[test]
    fn enter_on_plain_text_filters_by_name_keeps_the_text_and_closes_the_list()
    {
        let mut state = loaded_state();
        let before = row_count(&state);

        type_in_field(&mut state, "otty");
        tap_in_field(&mut state, Named::Enter);

        let search = state.table_search(&containers_page());
        assert_eq!(search.input(), "otty");
        assert!(search.tags().is_empty());
        assert!(!search.is_open());
        assert_eq!(row_count(&state), before, "every fixture name has otty");
    }

    #[test]
    fn focusing_the_empty_field_lists_the_keys_of_the_table() {
        let mut state = loaded_state();

        in_frame(&mut state, |ui| {
            ui.click(Id::new(SEARCH_ID))
                .expect("field should be visible");
        });

        let MainAreaViewModel::Table(table) =
            MainAreaViewModel::from_state(&state, NOW)
        else {
            panic!("expected a table");
        };
        let keys: Vec<&str> =
            table.suggestions.iter().map(Suggestion::text).collect();
        assert_eq!(
            keys,
            ["Name", "Status", "Image", "Compose", "Ports", "Age"]
        );
    }

    #[test]
    fn pressing_a_suggestion_applies_it_and_the_list_closes_on_a_press_outside()
    {
        let mut state = loaded_state();
        type_in_field(&mut state, "status:");

        in_frame(&mut state, |ui| {
            ui.click(i18n::t(Key::ContainersStatePaused))
                .expect("value should be listed");
        });
        let tags = state.table_search(&containers_page()).tags().len();

        in_frame(&mut state, |ui| {
            click_at(ui, iced::Point::new(WIDE - 2.0, 590.0));
        });

        assert_eq!(tags, 1);
        assert!(!state.table_search(&containers_page()).is_open());
    }

    #[test]
    fn what_a_table_sends_says_which_page_it_came_from() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click("db-1").expect("second row should be visible");

        let intents: Vec<ContainersIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![ContainersIntent::FromPage {
                page: containers_page(),
                intent: Box::new(ContainersIntent::SelectRow(1)),
            }]
        );
    }

    #[test]
    fn a_press_drawn_on_a_page_a_lost_engine_left_selects_nothing() {
        let podman = fixtures::engine(EngineKind::Podman);
        let resources = fixtures::resources(EngineKind::Podman);
        let mut state = loaded_state();
        for intent in [
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
            ContainersIntent::ListsLoaded {
                engine: podman.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        ] {
            let _ = reduce(&mut state, intent);
        }
        let themes = ThemeManager::new();
        let late: Vec<ContainersIntent> = {
            let vm = MainAreaViewModel::from_state(&state, NOW);
            let mut ui =
                simulator(area(vm, WIDE, ThemeProps::new(themes.current())));
            ui.click("otty-fx-exit3").expect("the row should be shown");
            ui.into_messages().collect()
        };

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Podman,
            ])),
        );
        let selected = state.selected_id().map(str::to_string);
        assert_eq!(late.len(), 1);
        for intent in late {
            let _ = reduce(&mut state, intent);
        }

        assert_eq!(state.place().map(Place::engine), Some(podman.address()));
        assert_eq!(state.selected_id().map(str::to_string), selected);
    }

    #[test]
    fn a_row_press_after_filtering_hits_the_row_that_is_shown() {
        let mut state = loaded_state();
        type_in_field(&mut state, "status:ex");
        // The first row is the typed text; the second is the first value.
        tap_in_field(&mut state, Named::ArrowDown);
        tap_in_field(&mut state, Named::ArrowDown);
        tap_in_field(&mut state, Named::Enter);
        tap_in_field(&mut state, Named::Escape);

        in_frame(&mut state, |ui| {
            ui.click("otty-fx-exit3").expect("the row should be shown");
        });

        let engine = state.open_engine().expect("docker engine");
        let selected = engine.containers().selected(Container::id);
        assert_eq!(selected.map(Container::name), Some("otty-fx-exit3"));
    }

    #[test]
    fn a_table_with_tags_and_no_rows_says_nothing_matches_the_search() {
        let mut state = loaded_state();
        type_in_field(&mut state, "status:");
        type_in_field(&mut state, "zzz");
        tap_in_field(&mut state, Named::Enter);
        tap_in_field(&mut state, Named::Escape);

        in_frame(&mut state, |ui| {
            assert!(ui.find(i18n::t(Key::ContainersNothingMatches)).is_ok());
        });
    }

    #[test]
    fn a_failed_list_shows_the_engine_message_in_a_notice() {
        let themes = ThemeManager::new();
        let vm = match table(Vec::new()) {
            MainAreaViewModel::Table(table) => {
                MainAreaViewModel::Table(TableViewModel {
                    error: Some("engine said no"),
                    ..table
                })
            },
            other => other,
        };
        let mut ui = simulator(view(MainAreaProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::containers_list_failed("engine said no").as_str())
            .expect("notice should show the error");
    }

    #[test]
    fn while_searching_the_main_area_shows_a_card() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: MainAreaViewModel::Searching,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::ContainersSearching))
            .expect("card text should show");
    }

    /// The card that shows while the search runs, in an area of `area`.
    fn searching_card_in<'a>(
        themes: &'a ThemeManager,
        area: iced::Size,
    ) -> iced_test::Simulator<'a, ContainersIntent> {
        iced_test::Simulator::with_size(
            iced::Settings::default(),
            area,
            ScreenArea::new(
                0,
                view(MainAreaProps {
                    vm: MainAreaViewModel::Searching,
                    theme: ThemeProps::new(themes.current()),
                }),
                themes.current().ui(),
            ),
        )
    }

    #[test]
    fn in_a_small_area_the_searching_card_scrolls_both_ways() {
        use iced::mouse;
        use iced_test::selector::Candidate;

        let themes = ThemeManager::new();
        let mut ui = searching_card_in(&themes, iced::Size::new(278.0, 90.0));
        let text = i18n::t(Key::ContainersSearching);

        let cut_off = ui.find(text).expect("the text").visible_bounds();
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate([iced::Event::Window(
            iced::window::Event::RedrawRequested(iced::time::Instant::now()),
        )]);
        let wheel =
            |ui: &mut iced_test::Simulator<'_, ContainersIntent>, x, y| {
                let _ = ui.simulate([iced::Event::Mouse(
                    mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Pixels { x, y },
                    },
                )]);
            };
        wheel(&mut ui, 0.0, -10_000.0);
        let shown = ui.find(text).expect("the text").visible_bounds();
        wheel(&mut ui, -10_000.0, 0.0);
        let (offset, content) = ui
            .find(|candidate: Candidate<'_>| match candidate {
                Candidate::Scrollable {
                    content_bounds,
                    translation,
                    ..
                } => Some((translation, content_bounds)),
                _ => None,
            })
            .expect("the area should scroll");

        assert!(cut_off.is_none_or(|bounds| bounds.y + bounds.height >= 90.0));
        assert_eq!(content.width, size::SCREEN_MIN);
        assert_eq!(offset.x, size::SCREEN_MIN - 278.0);
        assert!(offset.y > 0.0);
        let shown = shown.expect("the text should come into view");
        assert!(shown.y + shown.height <= 90.0);
    }

    /// With room for it the drawing must not change: it is the old layout, a
    /// card in a centered container.
    #[test]
    fn with_room_the_searching_card_draws_as_in_a_centered_container() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let folder = TempDir::new("searching-card");

        for (index, area) in [
            iced::Size::new(900.0, 700.0),
            iced::Size::new(901.0, 701.0),
            iced::Size::new(size::SCREEN_MIN, 300.0),
        ]
        .into_iter()
        .enumerate()
        {
            let reference = folder.path().join(format!("area-{index}"));
            let icon_color = palette.text_muted();
            let icon = svg(svg::Handle::from_memory(icons::MODE_CONTAINERS))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_, _| svg::Style {
                    color: Some(icon_color),
                });
            let plain = container(EmptyCard::<ContainersIntent>::new(
                i18n::t(Key::ModeContainers),
                i18n::t(Key::ContainersSearching),
                icon,
                palette,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

            let plain_matches = iced_test::Simulator::with_size(
                iced::Settings::default(),
                area,
                plain,
            )
            .snapshot(&iced::Theme::Dark)
            .expect("the card should draw")
            .matches_hash(&reference)
            .expect("reference should be readable");
            let page_matches = searching_card_in(&themes, area)
                .snapshot(&iced::Theme::Dark)
                .expect("the card should draw")
                .matches_hash(&reference)
                .expect("reference should be readable");

            assert!(plain_matches);
            assert!(page_matches, "at {area:?}");
        }
    }

    #[test]
    fn without_an_engine_the_main_area_shows_the_case_and_check_again() {
        let themes = ThemeManager::new();
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::PodmanApiOff,
            "",
            &[String::from("/run/podman/podman.sock")],
        );
        let mut ui = simulator(view(MainAreaProps {
            vm: MainAreaViewModel::NoEngine(vm),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::ContainersPodmanOffTitle))
            .expect("case title should show");
        ui.click(i18n::t(Key::ContainersCheckAgain))
            .expect("check again should be visible");

        let intents = sent(ui);
        assert_eq!(intents, vec![ContainersIntent::CheckAgain]);
    }

    #[test]
    fn an_enabled_create_button_opens_the_create_dialog() {
        let themes = ThemeManager::new();
        let vm = with(|table| TableViewModel {
            resource: ResourceKind::Volumes,
            can_create: true,
            ..table
        });
        let mut ui =
            simulator(area(vm, WIDE, ThemeProps::new(themes.current())));

        let create = toolbar_button(&mut ui, WIDE, 2);
        click_at(&mut ui, create);

        let intents = sent(ui);
        assert_eq!(intents, vec![ContainersIntent::OpenCreate]);
    }

    #[test]
    fn the_prune_button_asks_to_prune() {
        let themes = ThemeManager::new();
        let mut ui = simulator(area(
            table(rows()),
            WIDE,
            ThemeProps::new(themes.current()),
        ));

        let prune = toolbar_button(&mut ui, WIDE, 1);
        click_at(&mut ui, prune);

        let intents = sent(ui);
        assert_eq!(
            intents,
            vec![ContainersIntent::Actions(ActionsIntent::Prune)]
        );
    }

    #[test]
    fn a_running_prune_disables_its_button() {
        let themes = ThemeManager::new();
        let vm = with(|table| TableViewModel {
            prune: Some(PruneViewModel {
                label: i18n::t(Key::ContainersPruneStopped),
                is_enabled: false,
            }),
            ..table
        });
        let mut ui =
            simulator(area(vm, WIDE, ThemeProps::new(themes.current())));

        let prune = toolbar_button(&mut ui, WIDE, 1);
        click_at(&mut ui, prune);

        let intents = sent(ui);
        assert!(intents.is_empty());
    }

    #[test]
    fn a_right_press_on_a_row_opens_its_menu_at_the_pointer() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));
        let cell = ui.find("db-1").expect("second row should be visible");
        let bounds = cell.bounds();
        let pointer = iced::Point::new(bounds.x + 2.0, bounds.y + 2.0);

        ui.point_at(pointer);
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )),
        ]);

        let intents = sent(ui);
        assert_eq!(
            intents,
            vec![ContainersIntent::OpenRowMenu {
                index: 1,
                position: pointer
            }]
        );
    }
}
