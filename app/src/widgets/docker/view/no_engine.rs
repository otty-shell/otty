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
    if let Some(command) = &vm.command {
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

/// Check again, and How to fix when the case has a page for it.
fn buttons<'a>(
    vm: &NoEngineViewModel,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let has_command = vm.command.is_some();
    let kind = if has_command {
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
    if let Some(url) = vm.fix_url {
        buttons = buttons.push(
            Button::new(i18n::t(Key::DockerHowToFix), palette)
                .kind(ButtonKind::Ghost)
                .on_press(DockerIntent::OpenLink(url.to_string())),
        );
    }

    buttons.into()
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
            NoEngineCase::NoPermission,
            NoEngineCase::PodmanNoPermission,
        ] {
            let sent = intents(case, i18n::t(Key::DockerCheckAgain));

            assert_eq!(sent, vec![DockerIntent::CheckAgain]);
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
