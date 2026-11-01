//! The Docker mode main area: the header row (title, filter,
//! create button, Refresh) over the table of the open list, or a card while
//! no engine is known.

use iced::widget::text::Wrapping;
use iced::widget::{Id, Space, column, container, row, svg, text};
use iced::{Alignment, Element, Length, Padding};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::data_table::DataTable;
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::components::notice::Notice;
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
        .selected(vm.selected)
        .on_row_press(DockerIntent::SelectRow)
        .on_link_press(|row, _column| DockerIntent::OpenComposeProject(row))
        .on_row_right_press(|index, position| DockerIntent::OpenRowMenu {
            index,
            position,
        });
    for cells in vm.rows {
        table = table.row(cells);
    }
    if let Some(message) = vm.empty_message {
        table = table.empty_message(message);
    }

    content.push(table).into()
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

/// Title, the Compose project chip, filter, the Prune button,
/// the create button (Run container waits for ticket 14) and Refresh.
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
    let compose = vm.compose_chip.as_deref().map(|label| {
        Chip::new(label, palette).on_clear(
            i18n::t(Key::DockerShowAllContainers),
            DockerIntent::ClearComposeProject,
        )
    });
    let filter = TextField::new("", vm.filter, palette)
        .placeholder(i18n::t(Key::DockerFilterPlaceholder))
        .width(FILTER_WIDTH)
        .id(Id::new(FILTER_ID))
        .on_input(DockerIntent::FilterChanged);
    let mut create = Button::new(i18n::t(vm.resource.create_key()), palette)
        .size(ButtonSize::Small);
    if vm.can_create {
        create = create.on_press(DockerIntent::OpenCreate);
    }
    let refresh = IconButton::new(
        svg::Handle::from_memory(icons::REFRESH),
        i18n::t(Key::DockerRefresh),
        palette,
    )
    .on_press(DockerIntent::Refresh);

    let prune = vm.prune.as_ref().map(|prune| {
        let button = Button::new(prune.label, palette)
            .kind(ButtonKind::Ghost)
            .size(ButtonSize::Small);
        if prune.is_enabled {
            button.on_press(DockerIntent::Prune)
        } else {
            button
        }
    });

    let mut header = row![title];
    if let Some(compose) = compose {
        header = header.push(compose);
    }
    header = header.push(Space::new().width(Length::Fill)).push(filter);
    if let Some(prune) = prune {
        header = header.push(prune);
    }

    header
        .push(create)
        .push(refresh)
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
    use crate::widgets::docker::action_model::PruneViewModel;
    use crate::widgets::docker::no_engine::{
        self as no_engine_vm, NoEngineCase,
    };
    use crate::widgets::docker::types::ResourceKind;

    fn table<'a>(rows: Vec<Vec<TableCell>>) -> MainViewModel<'a> {
        MainViewModel::Table(TableViewModel {
            resource: ResourceKind::Containers,
            filter: "",
            compose_chip: None,
            columns: vec![
                TableColumn::new("Name", 3),
                TableColumn::new("Status", 2),
            ],
            rows,
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

    fn with(
        change: impl FnOnce(TableViewModel<'static>) -> TableViewModel<'static>,
    ) -> MainViewModel<'static> {
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
    fn the_header_shows_title_engine_filter_and_create_button() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::DockerContainers))
            .expect("title should show");
        assert!(ui.find("Docker 29.9.1").is_err(), "the engine chip is gone");
        assert!(
            ui.find(i18n::t(Key::ThisComputerNote)).is_err(),
            "no this computer note"
        );
        ui.find(i18n::t(Key::DockerRunContainer))
            .expect("create button should show");
        ui.click(i18n::t(Key::DockerRunContainer))
            .expect("create button should be there");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert!(intents.is_empty(), "the create button is disabled");
    }

    #[test]
    fn the_compose_chip_shows_and_its_cross_shows_all_containers() {
        let themes = ThemeManager::new();
        let vm = match table(rows()) {
            MainViewModel::Table(table) => {
                MainViewModel::Table(TableViewModel {
                    compose_chip: Some(i18n::docker_compose_chip("otty-site")),
                    ..table
                })
            },
            other => other,
        };
        let mut ui = simulator(view(MainAreaProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));
        let chip = ui
            .find("compose: otty-site")
            .expect("compose chip should show");
        let bounds = chip.bounds();

        // The cross sits one gap right of the chip label.
        ui.point_at(iced::Point::new(
            bounds.x + bounds.width + spacing::SPACE_2 + 12.0,
            bounds.y + bounds.height / 2.0,
        ));
        let _ = ui.simulate(iced_test::simulator::click());

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::ClearComposeProject]);
    }

    #[test]
    fn without_a_compose_project_there_is_no_chip() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find("compose: otty-site").is_err());
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

    #[test]
    fn an_enabled_create_button_opens_the_create_dialog() {
        let themes = ThemeManager::new();
        let vm = with(|table| TableViewModel {
            resource: ResourceKind::Volumes,
            can_create: true,
            ..table
        });
        let mut ui = simulator(view(MainAreaProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::DockerCreateVolume))
            .expect("create volume should show");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::OpenCreate]);
    }

    #[test]
    fn the_prune_button_asks_to_prune() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(MainAreaProps {
            vm: table(rows()),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::DockerPruneStopped))
            .expect("prune should show");

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
        let mut ui = simulator(view(MainAreaProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::DockerPruneStopped))
            .expect("prune should show");

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
