//! The Docker mode main area: the header row (title, engine chip, filter,
//! create button, Refresh) over the table of the open list, or a card while
//! no engine is known.

use iced::widget::text::Wrapping;
use iced::widget::{Id, Space, column, container, row, svg, text};
use iced::{Alignment, Element, Length, Padding};
use otty_ui_kit::components::button::{Button, ButtonSize};
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::data_table::DataTable;
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::DockerIntent;
use super::super::model::{MainViewModel, TableViewModel};
use super::no_engine;
use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// Height of the header row above the table.
const HEADER_HEIGHT: f32 = 48.0;
/// Width of the name filter.
const FILTER_WIDTH: f32 = 190.0;
/// Widget id of the name filter.
const FILTER_ID: &str = "docker_filter";

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

/// The header row, the error notice of a failed list and the table.
fn table_view<'a>(
    vm: TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let header = header_row(&vm, palette);

    let mut content = column![header].width(Length::Fill).height(Length::Fill);
    if let Some(error) = vm.error {
        let notice = Notice::new(
            i18n::docker_list_failed(error),
            svg::Handle::from_memory(icons::NOTICE_ERROR),
            palette,
        );
        content = content.push(container(notice).padding(Padding {
            top: 0.0,
            right: spacing::SPACE_5,
            bottom: spacing::SPACE_4,
            left: spacing::SPACE_5,
        }));
    }

    let mut table = DataTable::new(vm.columns, palette)
        .selected(vm.selected)
        .on_row_press(DockerIntent::SelectRow);
    for cells in vm.rows {
        table = table.row(cells);
    }
    if let Some(message) = vm.empty_message {
        table = table.empty_message(message);
    }

    content.push(table).into()
}

/// Title, engine chip, filter, the create button (disabled until its
/// ticket) and Refresh.
fn header_row<'a>(
    vm: &TableViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let heading = typography::HEADING;

    let title = text(i18n::t(vm.resource.title_key()))
        .size(heading.size())
        .font(fonts::UI_SEMIBOLD)
        .color(palette.text_strong())
        .wrapping(Wrapping::None);
    let engine = Chip::new(vm.engine_label.as_str(), palette)
        .target(TargetKind::Local)
        .note(i18n::t(Key::ThisComputerNote));
    let filter = TextField::new("", vm.filter, palette)
        .placeholder(i18n::t(Key::DockerFilterPlaceholder))
        .width(FILTER_WIDTH)
        .id(Id::new(FILTER_ID))
        .on_input(DockerIntent::FilterChanged);
    let create = Button::new(i18n::t(vm.resource.create_key()), palette)
        .size(ButtonSize::Small);
    let refresh = IconButton::new(
        svg::Handle::from_memory(icons::REFRESH),
        i18n::t(Key::DockerRefresh),
        palette,
    )
    .on_press(DockerIntent::Refresh);

    row![
        title,
        engine,
        Space::new().width(Length::Fill),
        filter,
        create,
        refresh
    ]
    .spacing(spacing::SPACE_4)
    .align_y(Alignment::Center)
    .height(HEADER_HEIGHT)
    .padding(Padding::from([0.0, spacing::SPACE_5]))
    .into()
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
    use iced_test::simulator;
    use otty_ui_kit::components::data_table::{TableCell, TableColumn};
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;
    use crate::theme::ThemeManager;
    use crate::widgets::docker::no_engine::{
        self as no_engine_vm, NoEngineCase,
    };
    use crate::widgets::docker::types::ResourceKind;

    fn table<'a>(rows: Vec<Vec<TableCell>>) -> MainViewModel<'a> {
        MainViewModel::Table(TableViewModel {
            resource: ResourceKind::Containers,
            engine_label: String::from("Docker 29.9.1"),
            filter: "",
            columns: vec![
                TableColumn::new("Name", 3),
                TableColumn::new("Status", 2),
            ],
            rows,
            selected: Some(0),
            empty_message: None,
            error: None,
        })
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
    fn the_header_shows_title_engine_filter_and_create_button() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::DockerContainers))
            .expect("title should show");
        ui.find("Docker 29.9.1").expect("engine chip should show");
        ui.find(i18n::t(Key::DockerRunContainer))
            .expect("create button should show");
        ui.click(i18n::t(Key::DockerRunContainer))
            .expect("create button should be there");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert!(intents.is_empty(), "the create button is disabled");
    }

    #[test]
    fn typing_in_the_filter_changes_it() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(Id::new(FILTER_ID))
            .expect("filter should be visible");
        let _ = ui.typewrite("w");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![DockerIntent::FilterChanged(String::from("w"))]
        );
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
}
