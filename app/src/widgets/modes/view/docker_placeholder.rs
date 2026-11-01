//! Temporary Docker mode slots until the `docker` widget exists (tickets
//! 09 and 10 of `specs/2026-11-01-modes-and-docker` replace them).

use iced::widget::{container, svg};
use iced::{Element, Length};
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};

use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// Render the Docker mode main area: a centered card with the Docker icon,
/// the mode name and what the mode will show.
pub(crate) fn main_area<'a, Message: Clone + 'a>(
    theme: ThemeProps<'a>,
) -> Element<'a, Message> {
    let palette = theme.theme.ui();
    let icon_color = palette.text_muted();

    let icon = svg(svg::Handle::from_memory(icons::MODE_DOCKER))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| svg::Style {
            color: Some(icon_color),
        });

    let card = EmptyCard::new(
        i18n::t(Key::ModeDocker),
        i18n::t(Key::DockerPlaceholderDescription),
        icon,
        palette,
    );

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Render the Docker mode status bar: the mode name on the left and
/// `OTTY <version>` on the right.
pub(crate) fn status_bar<'a, Message: Clone + 'a>(
    version: &str,
    theme: ThemeProps<'a>,
) -> Element<'a, Message> {
    StatusBar::new(theme.theme.ui())
        .left(StatusItem::text(i18n::t(Key::ModeDocker)))
        .right(StatusItem::text(format!("OTTY {version}")))
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::theme::ThemeManager;

    #[test]
    fn the_main_area_names_the_mode_and_what_it_will_show() {
        let themes = ThemeManager::new();
        let mut ui =
            simulator(main_area::<()>(ThemeProps::new(themes.current())));

        ui.find(i18n::t(Key::ModeDocker))
            .expect("mode name should be visible");
        ui.find(i18n::t(Key::DockerPlaceholderDescription))
            .expect("description should be visible");
    }

    #[test]
    fn the_status_bar_names_the_mode_and_the_version() {
        let themes = ThemeManager::new();
        let mut ui = simulator(status_bar::<()>(
            "0.4.0",
            ThemeProps::new(themes.current()),
        ));

        ui.find(i18n::t(Key::ModeDocker))
            .expect("mode name should be visible");
        ui.find("OTTY 0.4.0").expect("version should be visible");
    }
}
