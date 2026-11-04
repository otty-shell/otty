//! The Docker mode card shown when no engine answered: why, the command or
//! install pages that help, Check again and the "looked for" line.

use iced::widget::{Column, column, container, row, svg, text};
use iced::{Element, Length, Padding};
use otty_ui_kit::components::action_tile::ActionTile;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::command_box::CommandBox;
use otty_ui_kit::components::divider;
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::DockerIntent;
use super::super::no_engine::{
    CommandViewModel, LinkViewModel, NoEngineViewModel,
};
use crate::i18n::{self, Key};
use crate::icons;

/// Render the card of one no-engine case, centered in the main area.
pub(crate) fn view<'a>(
    vm: NoEngineViewModel,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let icon_color = palette.text_muted();
    let icon = svg(svg::Handle::from_memory(icons::MODE_DOCKER))
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

    let card = EmptyCard::new(vm.title, vm.description.as_str(), icon, palette)
        .action(body);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// A big button that opens an install page, with the site name on its
/// right.
fn install_tile<'a>(
    link: &LinkViewModel,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    ActionTile::new(
        svg::Handle::from_memory(icons::EXTERNAL_LINK),
        link.label,
        palette,
    )
    .note(link.site)
    .accent(true)
    .on_press(DockerIntent::OpenLink(link.url.to_string()))
    .into()
}

/// The label, the command with its Copy button and the hint under it.
fn command_block<'a>(
    command: &CommandViewModel,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let command_box =
        CommandBox::new(command.text, i18n::t(Key::DockerCopy), palette)
            .on_copy(DockerIntent::CopyCommand(command.text.to_string()));

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
) -> Element<'a, DockerIntent> {
    let kind = if !vm.commands.is_empty() {
        ButtonKind::Primary
    } else {
        ButtonKind::Secondary
    };

    let mut buttons = row![
        Button::new(i18n::t(Key::DockerCheckAgain), palette)
            .kind(kind)
            .on_press(DockerIntent::CheckAgain)
    ]
    .spacing(spacing::SPACE_3);
    for fix in &vm.fixes {
        buttons = buttons.push(
            Button::new(fix.label, palette)
                .kind(ButtonKind::Ghost)
                .on_press(DockerIntent::OpenLink(fix.url.to_string())),
        );
    }

    buttons.wrap().vertical_spacing(spacing::SPACE_2).into()
}

/// The muted line under a rule: what OTTY looked for.
fn looked_for<'a>(
    line: String,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
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
) -> Element<'a, DockerIntent> {
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
    use iced_test::simulator;
    use otty_docker::EngineKind;

    use super::*;
    use crate::theme::ThemeManager;
    use crate::widgets::docker::no_engine::{NoEngineCase, view_model};

    fn intents(case: NoEngineCase, click: &str) -> Vec<DockerIntent> {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let places = [String::from("/var/run/docker.sock")];
        let mut ui = simulator(view(
            view_model(case, "/var/run/docker.sock", &places),
            palette,
        ));

        ui.click(click).expect("the button should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn the_install_buttons_open_their_pages() {
        let docker = intents(
            NoEngineCase::NotInstalled,
            i18n::t(Key::DockerInstallDocker),
        );
        let podman = intents(
            NoEngineCase::NotInstalled,
            i18n::t(Key::DockerInstallPodman),
        );

        assert_eq!(
            docker,
            vec![DockerIntent::OpenLink(String::from(
                "https://docs.docker.com/get-started/get-docker/"
            ))]
        );
        assert_eq!(
            podman,
            vec![DockerIntent::OpenLink(String::from(
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
            let sent = intents(case, i18n::t(Key::DockerHowToFix));

            assert_eq!(sent, vec![DockerIntent::OpenLink(String::from(url))]);
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
            let sent = intents(case, i18n::t(Key::DockerCopy));

            assert_eq!(
                sent,
                vec![DockerIntent::CopyCommand(String::from(command))]
            );
        }
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
            let sent = intents(case, i18n::t(Key::DockerCheckAgain));

            assert_eq!(sent, vec![DockerIntent::CheckAgain]);
        }
    }

    /// A press on the button called `label` of the card of both engines.
    fn both_off_intents(label: &str) -> Vec<DockerIntent> {
        let themes = ThemeManager::new();
        let vm = view_model(NoEngineCase::DockerAndPodmanOff, "", &[]);
        let mut ui = simulator(view(vm, themes.current().ui()));

        ui.click(label).expect("the button should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn both_engines_off_shows_both_commands_and_their_notes() {
        let themes = ThemeManager::new();
        let vm = view_model(NoEngineCase::DockerAndPodmanOff, "", &[]);
        let texts: Vec<&str> = vm
            .commands
            .iter()
            .flat_map(|command| [command.label, command.text, command.hint])
            .collect();
        assert_eq!(texts.len(), 6);
        let mut ui = simulator(view(vm, themes.current().ui()));

        for text in texts {
            ui.find(text).expect("the text should show");
        }
    }

    /// The card has two Copy buttons with one label, so each block is pressed
    /// on its own.
    #[test]
    fn each_command_of_both_engines_off_copies_itself() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let vm = view_model(NoEngineCase::DockerAndPodmanOff, "", &[]);
        assert_eq!(vm.commands.len(), 2);

        for (command, expected) in vm.commands.iter().zip([
            "sudo systemctl start docker",
            "systemctl --user enable --now podman.socket",
        ]) {
            let mut ui = simulator(command_block(command, palette));

            ui.click(i18n::t(Key::DockerCopy))
                .expect("the button should be visible");

            let sent: Vec<DockerIntent> = ui.into_messages().collect();
            assert_eq!(
                sent,
                vec![DockerIntent::CopyCommand(String::from(expected))]
            );
        }
    }

    #[test]
    fn each_how_to_fix_of_both_engines_off_opens_its_own_page() {
        let docker = both_off_intents(i18n::t(Key::DockerHowToFixDocker));
        let podman = both_off_intents(i18n::t(Key::DockerHowToFixPodman));

        assert_eq!(
            docker,
            vec![DockerIntent::OpenLink(String::from(
                "https://docs.docker.com/engine/daemon/start/"
            ))]
        );
        assert_eq!(
            podman,
            vec![DockerIntent::OpenLink(String::from(
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
            let vm = view_model(
                NoEngineCase::NoPermissionAndOtherOff(engine),
                "/some.sock",
                &[],
            );
            assert_eq!(vm.commands.len(), 2);

            let mut card = simulator(view(vm.clone(), palette));
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

                block
                    .click(i18n::t(Key::DockerCopy))
                    .expect("the button should be visible");

                let sent: Vec<DockerIntent> = block.into_messages().collect();
                assert_eq!(
                    sent,
                    vec![DockerIntent::CopyCommand(String::from(expected))]
                );
            }
        }
    }

    #[test]
    fn a_denied_socket_with_the_other_off_opens_a_page_for_each_engine() {
        for (engine, _, pages) in denied_and_other_off_cards() {
            let case = NoEngineCase::NoPermissionAndOtherOff(engine);
            let labels = view_model(case, "/some.sock", &[]).fixes;
            assert_eq!(labels.len(), 2);

            for (fix, page) in labels.iter().zip(pages) {
                let sent = intents(case, fix.label);

                assert_eq!(
                    sent,
                    vec![DockerIntent::OpenLink(String::from(page))]
                );
            }
        }
    }

    #[test]
    fn the_card_shows_its_title_and_what_was_looked_for() {
        let themes = ThemeManager::new();
        let vm = view_model(
            NoEngineCase::DockerNotRunning,
            "",
            &[String::from("/var/run/docker.sock")],
        );
        let title = vm.title;
        let looked_for = vm.looked_for.clone();
        let mut ui = simulator(view(vm, themes.current().ui()));

        ui.find(title).expect("title should show");
        ui.find(looked_for.as_str())
            .expect("the looked for line should show");
    }
}
