//! The Docker mode main area: the header (the title row, the toolbar with the
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

use super::super::event::DockerIntent;
use super::super::model::{MainViewModel, TableViewModel};
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
const SEARCH_ID: &str = "docker_search";

/// Props for the Docker main area.
pub(crate) struct MainAreaProps<'a> {
    pub(crate) vm: MainViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the main area.
pub(crate) fn view(props: MainAreaProps<'_>) -> Element<'_, DockerIntent> {
    let palette = props.theme.theme.ui();

    match props.vm {
        MainViewModel::Table(table) => table_view(table, palette),
        MainViewModel::Searching => {
            card(i18n::t(Key::DockerSearching), palette)
        },
        MainViewModel::NoEngine(vm) => no_engine::view(vm, palette),
    }
}

/// The header, the error notice of a failed list and the table.
fn table_view<'a>(
    vm: TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let header = header(&vm, palette);

    let mut content = column![header].width(Length::Fill).height(Length::Fill);
    if let Some(error) = vm.error {
        let notice = Notice::new(
            i18n::docker_list_failed(error),
            svg::Handle::from_memory(icons::NOTICE_ERROR),
            palette,
        );
        content = content.push(notice_row(notice));
    }
    if let Some(text) = vm.notice {
        let notice = Notice::new(
            text,
            svg::Handle::from_memory(icons::NOTICE_ERROR),
            palette,
        )
        .on_dismiss(DockerIntent::DismissNotice, i18n::t(Key::DockerDismiss));
        content = content.push(notice_row(notice));
    }

    let mut table = DataTable::new(vm.columns, palette)
        .sorted_by(vm.sort.column(), sort_direction(vm.sort.order()))
        .on_sort(DockerIntent::SortBy)
        .selected(vm.selected)
        .on_row_press(DockerIntent::SelectRow)
        .on_link_press(|row, column, link| match column {
            PORTS_COLUMN => DockerIntent::OpenPortLink { row, link },
            _ => DockerIntent::OpenComposeProject(row),
        })
        .on_row_right_press(|index, position| DockerIntent::OpenRowMenu {
            index,
            position,
        })
        .on_row_menu(i18n::t(Key::DockerMoreActions), |index, corner| {
            // The table reports the button's bottom-left corner. The "⋯"
            // column is the last one, so the menu would open to the left of
            // that corner, in the middle of the table. The bottom-right
            // corner makes it open under the button, ending at its edge.
            let position = Point::new(corner.x + size::CONTROL_SM, corner.y);

            DockerIntent::RowMenuPressed { index, position }
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
fn notice_row<'a>(notice: Notice<DockerIntent>) -> Element<'a, DockerIntent> {
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
) -> Element<'a, DockerIntent> {
    let mut header = column![title_row(vm, palette), toolbar(vm, palette)]
        .spacing(spacing::SPACE_3)
        .padding(Padding::from([spacing::SPACE_4, spacing::SPACE_5]));
    if !vm.search.tags().is_empty() {
        header = header.push(
            TagRow::new(vm.search.tags(), &vm.keys, palette)
                .remove_label(i18n::t(Key::DockerTagRemove))
                .clear_all_label(i18n::t(Key::DockerTagsClearAll))
                .on_action(DockerIntent::TagSearch),
        );
    }

    header.into()
}

/// The title.
fn title_row<'a>(
    vm: &TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
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
) -> Element<'a, DockerIntent> {
    let search = vm.search;
    let suggestions = vm.suggestions.clone();
    let field_palette = palette.clone();

    let create = ToolbarItem::new(
        svg::Handle::from_memory(create_icon(vm.resource)),
        i18n::t(vm.resource.create_key()),
    )
    .on_press_maybe(vm.can_create.then_some(DockerIntent::OpenCreate));
    let mut toolbar = Toolbar::new(i18n::t(Key::DockerMoreActions), palette)
        .field(SEARCH_MIN_WIDTH, SEARCH_MAX_WIDTH, move || {
            TagSearchField::new(search, suggestions.clone(), &field_palette)
                .placeholder(i18n::t(Key::DockerSearchPlaceholder))
                .id(Id::new(SEARCH_ID))
                .on_action(DockerIntent::TagSearch)
                .into()
        })
        .item(create);
    if let Some(prune) = &vm.prune {
        toolbar = toolbar.item(
            ToolbarItem::new(
                svg::Handle::from_memory(icons::PRUNE),
                prune.label,
            )
            .on_press_maybe(prune.is_enabled.then_some(DockerIntent::Prune)),
        );
    }

    toolbar
        .item(
            ToolbarItem::new(
                svg::Handle::from_memory(icons::REFRESH),
                i18n::t(Key::DockerRefresh),
            )
            .on_press(DockerIntent::Refresh),
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

/// A centered card with the Docker icon and `description`, while the search
/// runs.
fn card<'a>(
    description: &'static str,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let icon_color = palette.text_muted();

    let icon = svg(svg::Handle::from_memory(icons::MODE_DOCKER))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| svg::Style {
            color: Some(icon_color),
        });

    let card =
        EmptyCard::new(i18n::t(Key::ModeDocker), description, icon, palette);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;
    use iced_test::simulator;
    use otty_docker::{Container, EngineKind, fixtures};
    use otty_ui_kit::components::data_table::{
        CellPart, TableCell, TableColumn,
    };
    use otty_ui_kit::components::status_badge::StatusTone;
    use otty_ui_kit::components::tag_search::{
        Suggestion, TagSearch, TagSearchAction,
    };

    use super::*;
    use crate::i18n::Locale;
    use crate::theme::ThemeManager;
    use crate::widgets::docker::action_model::PruneViewModel;
    use crate::widgets::docker::model;
    use crate::widgets::docker::no_engine::{
        self as no_engine_vm, NoEngineCase,
    };
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::sort::TableSorts;
    use crate::widgets::docker::state::DockerState;
    use crate::widgets::docker::table_search::{keys, keys_in};
    use crate::widgets::docker::types::{LoadedLists, ResourceKind};

    const NOW: i64 = 1_793_557_960;

    /// A search with nothing typed and no tags.
    static EMPTY_SEARCH: std::sync::LazyLock<TagSearch> =
        std::sync::LazyLock::new(TagSearch::default);

    fn table<'a>(rows: Vec<Vec<TableCell>>) -> MainViewModel<'a> {
        MainViewModel::Table(TableViewModel {
            resource: ResourceKind::Containers,
            search: &EMPTY_SEARCH,
            keys: keys(ResourceKind::Containers),
            suggestions: Vec::new(),
            sort: TableSorts::default().get(ResourceKind::Containers),
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
                label: i18n::t(Key::DockerPruneStopped),
                is_enabled: true,
            }),
            notice: None,
            can_create: false,
        })
    }

    fn with<'a>(
        change: impl FnOnce(TableViewModel<'a>) -> TableViewModel<'a>,
    ) -> MainViewModel<'a> {
        match table(rows()) {
            MainViewModel::Table(table) => MainViewModel::Table(change(table)),
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::SelectRow(1)]);
    }

    #[test]
    fn a_press_on_a_column_title_sorts_by_that_column() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click("Name").expect("title should be visible");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::SortBy(0)]);
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::OpenComposeProject(0)]);
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

        let first: Vec<DockerIntent> = first.into_messages().collect();
        let second: Vec<DockerIntent> = second.into_messages().collect();
        assert_eq!(first, vec![DockerIntent::OpenPortLink { row: 0, link: 0 }]);
        assert_eq!(
            second,
            vec![DockerIntent::OpenPortLink { row: 0, link: 1 }]
        );
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
        vm: MainViewModel<'a>,
        width: f32,
        theme: ThemeProps<'a>,
    ) -> Element<'a, DockerIntent> {
        container(view(MainAreaProps { vm, theme }))
            .width(width)
            .into()
    }

    /// Center of the toolbar button `from_end` places before the last one
    /// (0: Refresh, 1: Prune, 2: the create button), in a main area `width`
    /// wide. The buttons are centered on the filter's line.
    fn toolbar_button(
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
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
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
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
            .find(i18n::t(Key::DockerContainers))
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

        assert!(ui.find(i18n::t(Key::DockerRunContainer)).is_err());
        assert!(ui.find(i18n::t(Key::DockerPruneStopped)).is_err());
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::Refresh]);
    }

    #[test]
    fn at_a_narrow_width_prune_and_refresh_are_in_the_more_menu() {
        let themes = ThemeManager::new();
        let mut ui = simulator(area(
            table(rows()),
            NARROW,
            ThemeProps::new(themes.current()),
        ));
        assert!(ui.find(i18n::t(Key::DockerPruneStopped)).is_err());

        let more = toolbar_button(&mut ui, NARROW, 0);
        click_at(&mut ui, more);
        ui.click(i18n::t(Key::DockerPruneStopped))
            .expect("prune should be in the menu");
        click_at(&mut ui, more);
        ui.click(i18n::t(Key::DockerRefresh))
            .expect("refresh should be in the menu");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::Prune, DockerIntent::Refresh]);
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                DockerIntent::OpenCreate,
                DockerIntent::TagSearch(TagSearchAction::Focused),
                DockerIntent::TagSearch(TagSearchAction::InputChanged(
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert!(
            matches!(
                intents.as_slice(),
                [DockerIntent::RowMenuPressed { index: 1, .. }]
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::SelectRow(1)]);
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
        ui.click(i18n::t(Key::DockerTagsClearAll))
            .expect("clear all should show");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                DockerIntent::TagSearch(TagSearchAction::RemoveTag(1)),
                DockerIntent::TagSearch(TagSearchAction::ClearTags),
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

        assert!(ui.find(i18n::t(Key::DockerTagsClearAll)).is_err());
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                DockerIntent::TagSearch(TagSearchAction::Focused),
                DockerIntent::TagSearch(TagSearchAction::InputChanged(
                    String::from("w")
                )),
            ]
        );
    }

    // The whole flow, over the real state and reducer.

    /// Docker mode with the fixture engine and its lists.
    fn loaded_state() -> DockerState {
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
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
        state: &mut DockerState,
        step: impl FnOnce(&mut iced_test::Simulator<'_, DockerIntent>),
    ) {
        let themes = ThemeManager::new();
        let intents: Vec<DockerIntent> = {
            let vm = model::main_area(state, NOW);
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
    fn type_in_field(state: &mut DockerState, text: &str) {
        for character in text.chars() {
            in_frame(state, |ui| {
                ui.click(Id::new(SEARCH_ID))
                    .expect("field should be visible");
                let _ = ui.typewrite(character.to_string().as_str());
            });
        }
    }

    fn tap_in_field(state: &mut DockerState, key: Named) {
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
    fn row_count(state: &DockerState) -> usize {
        match model::main_area(state, NOW) {
            MainViewModel::Table(table) => table.rows.len(),
            _ => 0,
        }
    }

    #[test]
    fn typing_a_key_and_a_value_with_tab_gives_a_tag_and_fewer_rows() {
        let mut state = loaded_state();
        let before = row_count(&state);
        let search = |state: &DockerState| {
            state.table_search(ResourceKind::Containers).clone()
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
            vec![format!("Status:{}", i18n::t(Key::DockerStateRunning))]
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

        let search = state.table_search(ResourceKind::Containers);
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

        let search = state.table_search(ResourceKind::Containers);
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

        let MainViewModel::Table(table) = model::main_area(&state, NOW) else {
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
            ui.click(i18n::t(Key::DockerStatePaused))
                .expect("value should be listed");
        });
        let tags = state.table_search(ResourceKind::Containers).tags().len();

        in_frame(&mut state, |ui| {
            click_at(ui, iced::Point::new(WIDE - 2.0, 590.0));
        });

        assert_eq!(tags, 1);
        assert!(!state.table_search(ResourceKind::Containers).is_open());
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
            assert!(ui.find(i18n::t(Key::DockerNothingMatches)).is_ok());
        });
    }

    #[test]
    fn a_failed_list_shows_the_engine_message_in_a_notice() {
        let themes = ThemeManager::new();
        let vm = match table(Vec::new()) {
            MainViewModel::Table(table) => {
                MainViewModel::Table(TableViewModel {
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

        ui.find(i18n::docker_list_failed("engine said no").as_str())
            .expect("notice should show the error");
    }

    #[test]
    fn while_searching_the_main_area_shows_a_card() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: MainViewModel::Searching,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::DockerSearching))
            .expect("card text should show");
    }

    #[test]
    fn without_an_engine_the_main_area_shows_the_case_and_check_again() {
        let themes = ThemeManager::new();
        let vm = no_engine_vm::view_model(
            NoEngineCase::PodmanApiOff,
            "",
            &[String::from("/run/podman/podman.sock")],
        );
        let mut ui = simulator(view(MainAreaProps {
            vm: MainViewModel::NoEngine(vm),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::DockerPodmanOffTitle))
            .expect("case title should show");
        ui.click(i18n::t(Key::DockerCheckAgain))
            .expect("check again should be visible");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::CheckAgain]);
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::OpenCreate]);
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::Prune]);
    }

    #[test]
    fn a_running_prune_disables_its_button() {
        let themes = ThemeManager::new();
        let vm = with(|table| TableViewModel {
            prune: Some(PruneViewModel {
                label: i18n::t(Key::DockerPruneStopped),
                is_enabled: false,
            }),
            ..table
        });
        let mut ui =
            simulator(area(vm, WIDE, ThemeProps::new(themes.current())));

        let prune = toolbar_button(&mut ui, WIDE, 1);
        click_at(&mut ui, prune);

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert!(intents.is_empty());
    }

    #[test]
    fn a_failed_action_shows_a_notice_that_can_be_dismissed() {
        let themes = ThemeManager::new();
        let vm = with(|table| TableViewModel {
            notice: Some("Could not stop web-1: busy"),
            ..table
        });
        let mut ui = simulator(view(MainAreaProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        let notice = ui
            .find("Could not stop web-1: busy")
            .expect("notice should show");
        let bounds = notice.bounds();

        // The 16px dismiss cross sits one 10px gap right of the text.
        ui.point_at(iced::Point::new(
            bounds.x + bounds.width + 10.0 + 8.0,
            bounds.y + bounds.height / 2.0,
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::DismissNotice]);
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

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![DockerIntent::OpenRowMenu {
                index: 1,
                position: pointer
            }]
        );
    }
}
