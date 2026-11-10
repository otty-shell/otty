//! The Containers mode card shown when no engine answered: why, the command or
//! install pages that help, Check again and the "looked for" line.

use iced::widget::{Column, column, container, row, svg, text};
use iced::{Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::action_tile::ActionTile;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::command_box::CommandBox;
use otty_ui_kit::components::divider;
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::ContainersIntent;
use super::super::model::{
    NoEngineCommandViewModel, NoEngineLinkViewModel, NoEngineViewModel,
};

/// Render the card of one no-engine case: its icon, texts, blocks and buttons.
pub(crate) fn view<'a>(
    vm: NoEngineViewModel,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let icon_color = palette.text_muted();
    let icon = svg(svg::Handle::from_memory(icons::MODE_CONTAINERS))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| svg::Style {
            color: Some(icon_color),
        });

    let mut body = Column::new().spacing(spacing::SPACE_4);
    for link in &vm.installs {
        body = body.push(install_tile(link, palette));
    }
    for command in &vm.commands {
        body = body.push(command_block(command, palette));
    }
    body = body.push(buttons(&vm, palette));
    body = body.push(looked_for(vm.looked_for.clone(), palette));

    EmptyCard::new(vm.title, vm.description.as_str(), icon, palette)
        .action(body)
        .into()
}

/// A big button that opens an install page, with the site name on its
/// right.
fn install_tile<'a>(
    link: &NoEngineLinkViewModel,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    ActionTile::new(
        svg::Handle::from_memory(icons::EXTERNAL_LINK),
        link.label,
        palette,
    )
    .note(link.site)
    .accent(true)
    .on_press(ContainersIntent::OpenLink(link.url.to_string()))
    .into()
}

/// The label, the command with its Copy button and the hint under it.
fn command_block<'a>(
    command: &NoEngineCommandViewModel,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let command_box = CommandBox::new(
        command.text,
        svg::Handle::from_memory(icons::COPY),
        i18n::t(Key::ContainersCopy),
        palette,
    )
    .on_copy(ContainersIntent::CopyCommand(command.text.to_string()));

    column![
        small_text(command.label, palette),
        command_box,
        small_text(command.hint, palette),
    ]
    .spacing(spacing::SPACE_2)
    .into()
}

/// Check again, and How to fix for each page the case has. The row wraps, so
/// long translations of two pages stay inside the card.
fn buttons<'a>(
    vm: &NoEngineViewModel,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let kind = if !vm.commands.is_empty() {
        ButtonKind::Primary
    } else {
        ButtonKind::Secondary
    };

    let mut buttons = row![
        Button::new(i18n::t(Key::ContainersCheckAgain), palette)
            .kind(kind)
            .on_press(ContainersIntent::CheckAgain)
    ]
    .spacing(spacing::SPACE_3);
    for fix in &vm.fixes {
        buttons = buttons.push(
            Button::new(fix.label, palette)
                .kind(ButtonKind::Ghost)
                .on_press(ContainersIntent::OpenLink(fix.url.to_string())),
        );
    }

    buttons.wrap().vertical_spacing(spacing::SPACE_2).into()
}

/// The muted line under a rule: what OTTY looked for.
fn looked_for<'a>(
    line: String,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    column![
        divider::horizontal(palette),
        container(small_text(line, palette)).padding(Padding {
            top: spacing::SPACE_3,
            ..Padding::ZERO
        }),
    ]
    .into()
}

fn small_text<'a>(
    content: impl text::IntoFragment<'a>,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
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
    use iced::widget::column;
    use iced::{Point, Size, Theme, mouse};
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};
    use otty_app_testing::temp_dir::TempDir;
    use otty_app_theme::theme::ThemeManager;
    use otty_containers::EngineKind;
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::size;

    use super::*;
    use crate::types::NoEngineCase;

    /// The card in the area it has in the app.
    fn page<'a>(
        vm: NoEngineViewModel,
        palette: &UiPalette,
    ) -> Element<'a, ContainersIntent> {
        ScreenArea::new(0, view(vm, palette), palette).into()
    }

    /// Every card with a command, with the commands it copies.
    const CARDS_WITH_COMMANDS: [(NoEngineCase, &[&str]); 6] = [
        (
            NoEngineCase::DockerNotRunning,
            &["sudo systemctl start docker"],
        ),
        (
            NoEngineCase::PodmanApiOff,
            &["systemctl --user enable --now podman.socket"],
        ),
        (
            NoEngineCase::NoPermission,
            &["sudo usermod -aG docker $USER"],
        ),
        (
            NoEngineCase::PodmanNoPermission,
            &["systemctl --user enable --now podman.socket"],
        ),
        (
            NoEngineCase::DockerAndPodmanOff,
            &[
                "sudo systemctl start docker",
                "systemctl --user enable --now podman.socket",
            ],
        ),
        (
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
            &[
                "sudo usermod -aG docker $USER",
                "systemctl --user enable --now podman.socket",
            ],
        ),
    ];
    /// Every case of the card.
    const ALL_CASES: [NoEngineCase; 8] = [
        NoEngineCase::NotInstalled,
        NoEngineCase::DockerNotRunning,
        NoEngineCase::PodmanApiOff,
        NoEngineCase::DockerAndPodmanOff,
        NoEngineCase::NoPermission,
        NoEngineCase::PodmanNoPermission,
        NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
        NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
    ];
    /// How far the middle of a copy icon is from the right edge of its box:
    /// the padding of the box and half of the icon button.
    const ICON_INSET: f32 = 6.0 + size::CONTROL_SM / 2.0;
    /// The width of the simulator's default window.
    const DEFAULT_WINDOW_WIDTH: f32 = 1024.0;
    /// The right edge of a card at its full width in the default window.
    const CARD_RIGHT: f32 =
        (DEFAULT_WINDOW_WIDTH + otty_ui_kit::size::EMPTY_CARD) / 2.0;
    /// A main area too narrow for the card: both panels dragged wide can leave
    /// this or less.
    const NARROW_MAIN: Size = Size::new(278.0, 900.0);
    /// A main area narrow and low enough that every card overflows both ways.
    const SMALL_MAIN: Size = Size::new(278.0, 200.0);
    /// The padding, gap and icon of a command box beside its text.
    const BOX_AROUND_COMMAND: f32 = 50.0;
    /// Far enough to reach the end of any card.
    const FAR: f32 = -10_000.0;
    /// The empty space on each side of a card in an area narrower than the
    /// minimum of a screen: the card is centered in that minimum.
    const CARD_MARGIN: f32 = (size::SCREEN_MIN - size::EMPTY_CARD) / 2.0;
    /// A command longer than a card, which no card has today.
    const TOO_LONG: &str = "systemctl --user enable --now podman.socket \
                            --and --some --more --words --that --do --not --fit";

    /// Moves the pointer to the copy icon on the line of `command`, whose box
    /// ends at `right`, and presses it.
    fn press_copy_icon(
        ui: &mut Simulator<'_, ContainersIntent>,
        command: &str,
        right: f32,
    ) {
        let line = ui.find(command).expect("the command should show");
        let position = Point::new(right - ICON_INSET, line.bounds().center_y());

        ui.point_at(position);
        let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved {
            position,
        })]);
        let _ = ui.simulate(simulator::click());
    }

    /// Scrolls under `pointer`, after a frame as in the running app: a scroller
    /// that has never been drawn takes the first wheel for itself. As in the
    /// tables, a negative number scrolls right or down.
    fn wheel_at(
        ui: &mut Simulator<'_, ContainersIntent>,
        pointer: Point,
        x: f32,
        y: f32,
    ) {
        ui.point_at(pointer);
        let _ = ui.simulate([iced::Event::Window(
            iced::window::Event::RedrawRequested(iced::time::Instant::now()),
        )]);
        let _ =
            ui.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x, y },
            })]);
    }

    /// How far a scroller is scrolled and how wide and high its content is
    /// against what shows: the area when `of_area`, otherwise the command.
    fn scroller(
        ui: &mut Simulator<'_, ContainersIntent>,
        of_area: bool,
    ) -> (Point, Size, Size) {
        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                bounds,
                content_bounds,
                translation,
                ..
            } if (bounds.height == size::CONTROL_SM) != of_area => Some((
                Point::new(translation.x, translation.y),
                content_bounds.size(),
                bounds.size(),
            )),
            _ => None,
        })
        .expect("the scroller should exist")
    }

    /// A card whose first command is `command`, whatever its length.
    fn card_with_command(command: &'static str) -> NoEngineViewModel {
        let mut vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerNotRunning,
            "",
            &[],
        );
        if let Some(first) = vm.commands.first_mut() {
            first.text = command;
        }

        vm
    }

    fn intents(case: NoEngineCase, click: &str) -> Vec<ContainersIntent> {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let places = [String::from("/var/run/docker.sock")];
        let mut ui = simulator(page(
            NoEngineViewModel::for_case(case, "/var/run/docker.sock", &places),
            palette,
        ));

        ui.click(click).expect("the button should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn the_install_buttons_open_their_pages() {
        let docker = intents(
            NoEngineCase::NotInstalled,
            i18n::t(Key::ContainersInstallDocker),
        );
        let podman = intents(
            NoEngineCase::NotInstalled,
            i18n::t(Key::ContainersInstallPodman),
        );

        assert_eq!(
            docker,
            vec![ContainersIntent::OpenLink(String::from(
                "https://docs.docker.com/get-started/get-docker/"
            ))]
        );
        assert_eq!(
            podman,
            vec![ContainersIntent::OpenLink(String::from(
                "https://podman.io/docs/installation"
            ))]
        );
    }

    #[test]
    fn how_to_fix_opens_the_page_of_the_case() {
        let cases = [
            (
                NoEngineCase::DockerNotRunning,
                "https://docs.docker.com/engine/daemon/start/",
            ),
            (
                NoEngineCase::PodmanApiOff,
                "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html",
            ),
            (
                NoEngineCase::NoPermission,
                "https://docs.docker.com/engine/install/linux-postinstall/",
            ),
            (
                NoEngineCase::PodmanNoPermission,
                "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html",
            ),
        ];

        for (case, url) in cases {
            let sent = intents(case, i18n::t(Key::ContainersHowToFix));

            assert_eq!(
                sent,
                vec![ContainersIntent::OpenLink(String::from(url))]
            );
        }
    }

    #[test]
    fn copy_sends_the_command_of_the_case() {
        let cases = [
            (
                NoEngineCase::DockerNotRunning,
                "sudo systemctl start docker",
            ),
            (
                NoEngineCase::PodmanApiOff,
                "systemctl --user enable --now podman.socket",
            ),
            (NoEngineCase::NoPermission, "sudo usermod -aG docker $USER"),
            (
                NoEngineCase::PodmanNoPermission,
                "systemctl --user enable --now podman.socket",
            ),
        ];

        for (case, command) in cases {
            let themes = ThemeManager::new();
            let places = [String::from("/var/run/docker.sock")];
            let mut ui = simulator(page(
                NoEngineViewModel::for_case(
                    case,
                    "/var/run/docker.sock",
                    &places,
                ),
                themes.current().ui(),
            ));

            press_copy_icon(&mut ui, command, CARD_RIGHT);

            let sent: Vec<ContainersIntent> = ui.into_messages().collect();
            assert_eq!(
                sent,
                vec![ContainersIntent::CopyCommand(String::from(command))]
            );
        }
    }

    #[test]
    fn no_card_shows_the_word_copy_because_it_is_a_tooltip() {
        let themes = ThemeManager::new();

        for (case, _) in CARDS_WITH_COMMANDS {
            let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);
            let mut ui = simulator(page(vm, themes.current().ui()));

            assert!(ui.find(i18n::t(Key::ContainersCopy)).is_err(), "{case:?}");
        }
    }

    /// Draws `block` with the pointer on its copy icon and checks it against
    /// the picture saved in `reference`. iced draws tooltips as overlays that
    /// widget queries do not reach, so the text is checked on the drawing.
    fn hovered_block_matches(
        block: Element<'_, ContainersIntent>,
        reference: &std::path::Path,
    ) -> bool {
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            Size::new(420.0, 200.0),
            block,
        );
        let line = ui
            .find("sudo systemctl start docker")
            .expect("the command should show");
        let position = Point::new(420.0 - ICON_INSET, line.bounds().center_y());

        ui.point_at(position);
        let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved {
            position,
        })]);

        ui.snapshot(&Theme::Dark)
            .expect("the block should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn a_hovered_copy_icon_shows_the_copy_text_as_its_tooltip() {
        let folder = TempDir::new("containers-copy");
        let reference = folder.path().join("tooltip");
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerNotRunning,
            "",
            &[],
        );
        let command = vm.commands.first().expect("the card has a command");
        let block_with_label =
            |label: &'static str| -> Element<'_, ContainersIntent> {
                column![
                    small_text(command.label, palette),
                    CommandBox::new(
                        command.text,
                        svg::Handle::from_memory(icons::COPY),
                        label,
                        palette,
                    )
                    .on_copy(
                        ContainersIntent::CopyCommand(command.text.to_string())
                    ),
                    small_text(command.hint, palette),
                ]
                .spacing(spacing::SPACE_2)
                .into()
            };

        assert!(hovered_block_matches(
            command_block(command, palette),
            &reference
        ));
        assert!(hovered_block_matches(
            block_with_label(i18n::t(Key::ContainersCopy)),
            &reference
        ));
        assert!(!hovered_block_matches(
            block_with_label("Another text"),
            &reference
        ));
    }

    /// In a narrow area the card keeps its minimum and the area scrolls, and
    /// each command still copies itself, from an icon scrolled into view.
    #[test]
    fn in_a_narrow_main_area_each_icon_copies_its_own_command() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();

        for (case, commands) in CARDS_WITH_COMMANDS {
            for command in commands {
                let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);
                let mut ui = Simulator::with_size(
                    iced::Settings::default(),
                    NARROW_MAIN,
                    page(vm, palette),
                );

                wheel_at(&mut ui, Point::new(100.0, 100.0), FAR, 0.0);
                press_copy_icon(
                    &mut ui,
                    command,
                    NARROW_MAIN.width - CARD_MARGIN,
                );

                let sent: Vec<ContainersIntent> = ui.into_messages().collect();
                assert_eq!(
                    sent,
                    vec![ContainersIntent::CopyCommand(command.to_string())],
                    "{case:?}"
                );
            }
        }
    }

    /// The height between a label and its note is the same for every command
    /// and width: a line of text in the box, its padding and two gaps.
    #[test]
    fn a_command_block_has_one_height_on_every_card_and_width() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let expected = 36.0 + 2.0 * spacing::SPACE_2;

        for width in [640.0, NARROW_MAIN.width, 200.0] {
            for (case, _) in CARDS_WITH_COMMANDS {
                let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);

                for command in &vm.commands {
                    let mut ui = Simulator::with_size(
                        iced::Settings::default(),
                        Size::new(width, 900.0),
                        command_block(command, palette),
                    );

                    let label = ui.find(command.label).expect("label").bounds();
                    let hint = ui.find(command.hint).expect("hint").bounds();

                    assert_eq!(
                        hint.y - (label.y + label.height),
                        expected,
                        "{} at {width}",
                        command.text
                    );
                }
            }
        }
    }

    /// The exact command is in the widget tree on one line, not shortened, so
    /// it can be read by scrolling.
    #[test]
    fn a_command_wider_than_its_box_stays_whole_on_one_line() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();

        for (case, commands) in CARDS_WITH_COMMANDS {
            let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);

            for (block, command) in vm.commands.iter().zip(commands) {
                let mut ui = Simulator::with_size(
                    iced::Settings::default(),
                    NARROW_MAIN,
                    command_block(block, palette),
                );

                let line = ui.find(*command).expect("the exact text").bounds();

                assert!(line.height < 20.0, "{command} wrapped");
            }
        }
    }

    /// A card holds each command with no scrolling of its own. The area is what
    /// scrolls. The fonts of the kit are not loaded here: that
    /// would change what other tests draw halfway through their comparisons.
    #[test]
    fn the_card_shows_every_command_whole() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();

        for (case, commands) in CARDS_WITH_COMMANDS {
            let mut ui = Simulator::with_size(
                iced::Settings::default(),
                SMALL_MAIN,
                page(
                    NoEngineViewModel::for_case(case, "/some.sock", &[]),
                    palette,
                ),
            );

            for command in commands {
                let line = ui.find(*command).expect("the exact text").bounds();

                assert!(
                    line.width <= size::EMPTY_CARD - BOX_AROUND_COMMAND,
                    "{command} is {} wide",
                    line.width
                );
            }
        }
    }

    #[test]
    fn in_a_small_area_every_button_of_every_card_can_be_reached() {
        for case in ALL_CASES {
            let themes = ThemeManager::new();
            let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);
            let installs: Vec<&str> =
                vm.installs.iter().map(|link| link.label).collect();
            let fixes: Vec<&str> =
                vm.fixes.iter().map(|fix| fix.label).collect();
            let labels = installs
                .into_iter()
                .chain([i18n::t(Key::ContainersCheckAgain)])
                .chain(fixes);

            for label in labels {
                let mut ui = Simulator::with_size(
                    iced::Settings::default(),
                    SMALL_MAIN,
                    page(vm.clone(), themes.current().ui()),
                );

                scroll_into_view(&mut ui, label);
                ui.click(label).expect("the button should be in reach");

                let sent: Vec<ContainersIntent> = ui.into_messages().collect();
                assert_eq!(sent.len(), 1, "{case:?}: {label}");
            }
        }
    }

    /// Scrolls until `label` shows clear of the bars of [`SMALL_MAIN`]: down
    /// in small steps, from the left end of the area and then from its right
    /// end, since a card does not show whole either way.
    fn scroll_into_view(ui: &mut Simulator<'_, ContainersIntent>, label: &str) {
        let pointer = Point::new(10.0, 10.0);
        let clear_height = SMALL_MAIN.height - 4.0;
        let clear_width = SMALL_MAIN.width - 4.0;

        for sideways in [0.0, FAR] {
            wheel_at(ui, pointer, sideways, -FAR);

            for _ in 0..60 {
                let shown = ui.find(label).expect("the text").visible_bounds();

                if shown.is_some_and(|shown| {
                    shown.y + shown.height <= clear_height
                        && shown.center_x() <= clear_width
                }) {
                    return;
                }

                wheel_at(ui, pointer, 0.0, -20.0);
            }
        }

        panic!("{label} did not come into view");
    }

    /// The last thing on a card is the "looked for" line. It must come into
    /// view in the far corner of the area, on every card.
    #[test]
    fn in_a_small_area_the_end_of_every_card_can_be_reached() {
        let themes = ThemeManager::new();

        for case in ALL_CASES {
            let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);
            let looked_for = vm.looked_for.clone();
            let mut ui = Simulator::with_size(
                iced::Settings::default(),
                SMALL_MAIN,
                page(vm, themes.current().ui()),
            );

            wheel_at(&mut ui, Point::new(10.0, 10.0), FAR, FAR);
            let line = ui.find(looked_for.as_str()).expect("the line");

            let shown = line.visible_bounds().expect("it should be in view");
            assert!(shown.y + shown.height <= SMALL_MAIN.height, "{case:?}");
            assert!(shown.x >= 0.0, "{case:?}");
        }
    }

    /// With room for it no card scrolls, whichever the case.
    #[test]
    fn in_a_large_area_no_card_scrolls() {
        let themes = ThemeManager::new();

        for case in ALL_CASES {
            let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);
            let mut ui = Simulator::with_size(
                iced::Settings::default(),
                Size::new(DEFAULT_WINDOW_WIDTH, 1000.0),
                page(vm, themes.current().ui()),
            );

            let (_, content, showing) = scroller(&mut ui, true);
            wheel_at(&mut ui, Point::new(10.0, 10.0), FAR, FAR);
            let (offset, _, _) = scroller(&mut ui, true);

            assert_eq!(content, showing, "{case:?}");
            assert_eq!(offset, Point::ORIGIN, "{case:?}");
        }
    }

    /// The card's drawing with room for it must not change: it is the old
    /// layout, a card in a centered container, in every case.
    #[test]
    fn with_room_every_card_draws_as_in_a_centered_container() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let folder = TempDir::new("no-engine-card");

        for (index, case) in ALL_CASES.into_iter().enumerate() {
            let reference = folder.path().join(format!("case-{index}"));
            let vm = NoEngineViewModel::for_case(case, "/some.sock", &[]);
            let plain = container(view(vm.clone(), palette))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            let plain_matches = Simulator::with_size(
                iced::Settings::default(),
                Size::new(900.0, 800.0),
                plain,
            )
            .snapshot(&Theme::Dark)
            .expect("the card should draw")
            .matches_hash(&reference)
            .expect("reference should be readable");
            let page_matches = Simulator::with_size(
                iced::Settings::default(),
                Size::new(900.0, 800.0),
                page(vm, palette),
            )
            .snapshot(&Theme::Dark)
            .expect("the card should draw")
            .matches_hash(&reference)
            .expect("reference should be readable");

            assert!(plain_matches);
            assert!(page_matches, "{case:?}");
        }
    }

    /// A card with [`TOO_LONG`] in an area small enough to scroll both ways,
    /// and a point over the command.
    fn card_with_a_too_long_command<'a>(
        palette: &UiPalette,
    ) -> (Simulator<'a, ContainersIntent>, Point) {
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            Size::new(SMALL_MAIN.width, 280.0),
            page(card_with_command(TOO_LONG), palette),
        );
        let line = ui.find(TOO_LONG).expect("the command").bounds();
        assert!(line.center_y() < 270.0, "it should show: {line:?}");

        (ui, Point::new(line.x + 10.0, line.center_y()))
    }

    #[test]
    fn a_sideways_wheel_over_a_command_too_long_for_the_card_moves_the_command()
    {
        let themes = ThemeManager::new();
        let (mut ui, over_command) =
            card_with_a_too_long_command(themes.current().ui());

        wheel_at(&mut ui, over_command, -20.0, 0.0);
        let (command, _, _) = scroller(&mut ui, false);
        let (area, _, _) = scroller(&mut ui, true);

        assert_eq!(command.x, 20.0);
        assert_eq!(area, Point::ORIGIN);
    }

    /// The command can not move any further, so the next wheel goes on to the
    /// area. Within one gesture the wheel stays with the scroller it started
    /// on, as everywhere in iced.
    #[test]
    fn at_the_end_of_a_command_the_next_sideways_wheel_moves_the_area() {
        let themes = ThemeManager::new();
        let (mut ui, over_command) =
            card_with_a_too_long_command(themes.current().ui());

        wheel_at(&mut ui, over_command, FAR, 0.0);
        let (command, _, _) = scroller(&mut ui, false);
        let _ = ui.simulate([iced::Event::Mouse(
            mouse::Event::ButtonReleased(mouse::Button::Left),
        )]);
        wheel_at(&mut ui, over_command, -20.0, 0.0);
        let (command_after, _, _) = scroller(&mut ui, false);
        let (area, _, _) = scroller(&mut ui, true);

        assert!(command.x > 0.0);
        assert_eq!(command_after, command);
        assert_eq!(area, Point::new(20.0, 0.0));
    }

    /// A box of one line has no use for a plain wheel, so it scrolls the area.
    #[test]
    fn a_plain_wheel_over_a_command_moves_the_area() {
        let themes = ThemeManager::new();
        let (mut ui, over_command) =
            card_with_a_too_long_command(themes.current().ui());

        wheel_at(&mut ui, over_command, 0.0, -10.0);
        let (command, _, _) = scroller(&mut ui, false);
        let (area, _, _) = scroller(&mut ui, true);

        assert_eq!(command, Point::ORIGIN);
        assert_eq!(area, Point::new(0.0, 10.0));
    }

    /// A press on the box's own bar scrolls the command and not the area.
    #[test]
    fn the_bar_of_a_command_too_long_for_the_card_moves_only_the_command() {
        let themes = ThemeManager::new();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            Size::new(size::SCREEN_MIN, 900.0),
            page(card_with_command(TOO_LONG), themes.current().ui()),
        );
        let line = ui.find(TOO_LONG).expect("the command").bounds();
        let bar_y = line.center_y() + size::CONTROL_SM / 2.0 - 2.0;

        for (index, x) in [20.0, 120.0].into_iter().enumerate() {
            let position = Point::new(line.x + x, bar_y);
            ui.point_at(position);
            let _ =
                ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved {
                    position,
                })]);
            if index == 0 {
                let _ = ui.simulate([iced::Event::Mouse(
                    mouse::Event::ButtonPressed(mouse::Button::Left),
                )]);
            }
        }

        let (command, _, _) = scroller(&mut ui, false);
        let (area, _, _) = scroller(&mut ui, true);
        assert!(command.x > 0.0, "the command should move");
        assert_eq!(area, Point::ORIGIN);
    }

    #[test]
    fn a_scrolled_command_inside_a_scrolled_area_still_copies_itself() {
        let themes = ThemeManager::new();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            Size::new(300.0, 900.0),
            page(card_with_command(TOO_LONG), themes.current().ui()),
        );
        let line = ui.find(TOO_LONG).expect("the command").bounds();
        let over_command = Point::new(line.x + 10.0, line.center_y());

        wheel_at(&mut ui, over_command, -20.0, 0.0);
        wheel_at(&mut ui, Point::new(50.0, 5.0), FAR, 0.0);
        press_copy_icon(&mut ui, TOO_LONG, 300.0 - CARD_MARGIN);

        let sent: Vec<ContainersIntent> = ui.into_messages().collect();
        assert_eq!(
            sent,
            vec![ContainersIntent::CopyCommand(TOO_LONG.to_string())]
        );
    }

    #[test]
    fn check_again_is_on_every_case() {
        for case in [
            NoEngineCase::NotInstalled,
            NoEngineCase::DockerNotRunning,
            NoEngineCase::PodmanApiOff,
            NoEngineCase::DockerAndPodmanOff,
            NoEngineCase::NoPermission,
            NoEngineCase::PodmanNoPermission,
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
            NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
        ] {
            let sent = intents(case, i18n::t(Key::ContainersCheckAgain));

            assert_eq!(sent, vec![ContainersIntent::CheckAgain]);
        }
    }

    /// A press on the button called `label` of the card of both engines.
    fn both_off_intents(label: &str) -> Vec<ContainersIntent> {
        let themes = ThemeManager::new();
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerAndPodmanOff,
            "",
            &[],
        );
        let mut ui = simulator(page(vm, themes.current().ui()));

        ui.click(label).expect("the button should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn both_engines_off_shows_both_commands_and_their_notes() {
        let themes = ThemeManager::new();
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerAndPodmanOff,
            "",
            &[],
        );
        let texts: Vec<&str> = vm
            .commands
            .iter()
            .flat_map(|command| [command.label, command.text, command.hint])
            .collect();
        assert_eq!(texts.len(), 6);
        let mut ui = simulator(page(vm, themes.current().ui()));

        for text in texts {
            ui.find(text).expect("the text should show");
        }
    }

    /// The card has two copy icons, so each block is pressed on its own.
    #[test]
    fn each_command_of_both_engines_off_copies_itself() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerAndPodmanOff,
            "",
            &[],
        );
        assert_eq!(vm.commands.len(), 2);

        for (command, expected) in vm.commands.iter().zip([
            "sudo systemctl start docker",
            "systemctl --user enable --now podman.socket",
        ]) {
            let mut ui = simulator(command_block(command, palette));

            press_copy_icon(&mut ui, expected, DEFAULT_WINDOW_WIDTH);

            let sent: Vec<ContainersIntent> = ui.into_messages().collect();
            assert_eq!(
                sent,
                vec![ContainersIntent::CopyCommand(String::from(expected))]
            );
        }
    }

    #[test]
    fn each_how_to_fix_of_both_engines_off_opens_its_own_page() {
        let docker = both_off_intents(i18n::t(Key::ContainersHowToFixDocker));
        let podman = both_off_intents(i18n::t(Key::ContainersHowToFixPodman));

        assert_eq!(
            docker,
            vec![ContainersIntent::OpenLink(String::from(
                "https://docs.docker.com/engine/daemon/start/"
            ))]
        );
        assert_eq!(
            podman,
            vec![ContainersIntent::OpenLink(String::from(
                "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html"
            ))]
        );
    }

    /// What each card of a denied socket with the other engine off copies and
    /// opens, in the order of its command blocks.
    fn denied_and_other_off_cards()
    -> [(EngineKind, [&'static str; 2], [&'static str; 2]); 2] {
        let podman_page = "https://docs.podman.io/en/latest/markdown/podman-system-service.1.html";
        let podman_command = "systemctl --user enable --now podman.socket";

        [
            (
                EngineKind::Docker,
                ["sudo usermod -aG docker $USER", podman_command],
                [
                    "https://docs.docker.com/engine/install/linux-postinstall/",
                    podman_page,
                ],
            ),
            (
                EngineKind::Podman,
                [podman_command, "sudo systemctl start docker"],
                [podman_page, "https://docs.docker.com/engine/daemon/start/"],
            ),
        ]
    }

    #[test]
    fn a_denied_socket_with_the_other_off_shows_both_blocks_and_copies_each() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();

        for (engine, commands, _) in denied_and_other_off_cards() {
            let vm = NoEngineViewModel::for_case(
                NoEngineCase::NoPermissionAndOtherOff(engine),
                "/some.sock",
                &[],
            );
            assert_eq!(vm.commands.len(), 2);

            let mut card = simulator(page(vm.clone(), palette));
            card.find(vm.title).expect("the title should show");
            card.find(vm.description.as_str())
                .expect("the text should show");
            for command in &vm.commands {
                for text in [command.label, command.text, command.hint] {
                    card.find(text).expect("the text should show");
                }
            }

            for (command, expected) in vm.commands.iter().zip(commands) {
                let mut block = simulator(command_block(command, palette));

                press_copy_icon(&mut block, expected, DEFAULT_WINDOW_WIDTH);

                let sent: Vec<ContainersIntent> =
                    block.into_messages().collect();
                assert_eq!(
                    sent,
                    vec![ContainersIntent::CopyCommand(String::from(expected))]
                );
            }
        }
    }

    #[test]
    fn a_denied_socket_with_the_other_off_opens_a_page_for_each_engine() {
        for (engine, _, pages) in denied_and_other_off_cards() {
            let case = NoEngineCase::NoPermissionAndOtherOff(engine);
            let labels =
                NoEngineViewModel::for_case(case, "/some.sock", &[]).fixes;
            assert_eq!(labels.len(), 2);

            for (fix, page) in labels.iter().zip(pages) {
                let sent = intents(case, fix.label);

                assert_eq!(
                    sent,
                    vec![ContainersIntent::OpenLink(String::from(page))]
                );
            }
        }
    }

    #[test]
    fn the_card_shows_its_title_and_what_was_looked_for() {
        let themes = ThemeManager::new();
        let vm = NoEngineViewModel::for_case(
            NoEngineCase::DockerNotRunning,
            "",
            &[String::from("/var/run/docker.sock")],
        );
        let title = vm.title;
        let looked_for = vm.looked_for.clone();
        let mut ui = simulator(page(vm, themes.current().ui()));

        ui.find(title).expect("title should show");
        ui.find(looked_for.as_str())
            .expect("the looked for line should show");
    }
}
