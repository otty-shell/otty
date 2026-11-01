//! The Docker mode status bar: the mode name and the OTTY version. Ticket 18
//! of `specs/2026-11-01-modes-and-docker` puts the engine and the counts
//! here.

use iced::Element;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};

use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Render the Docker mode status bar: the mode name on the left and
/// `OTTY <version>` on the right.
pub(crate) fn view<'a, Message: Clone + 'a>(
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
    fn the_status_bar_names_the_mode_and_the_version() {
        let themes = ThemeManager::new();
        let mut ui =
            simulator(view::<()>("0.4.0", ThemeProps::new(themes.current())));

        ui.find(i18n::t(Key::ModeDocker))
            .expect("mode name should be visible");
        ui.find("OTTY 0.4.0").expect("version should be visible");
    }
}
