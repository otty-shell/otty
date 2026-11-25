//! The Add plugin menu under the chevron of the Installed header: from a
//! repository, by its GitHub link, or from a folder. The frame opens,
//! places and closes it, as the menus of the other section headers.

use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::palette::UiPalette;

use crate::event::PluginsIntent;

/// The Add plugin menu: Add plugin… and Add plugin from folder….
pub(crate) fn menu(palette: &UiPalette) -> Menu<PluginsIntent> {
    let from_repository = MenuItem::new(i18n::t(Key::PluginsAddPlugin))
        .on_press(PluginsIntent::AddPlugin);
    let from_folder = MenuItem::new(i18n::t(Key::PluginsAddFromFolder))
        .on_press(PluginsIntent::AddFromFolder);

    Menu::new(palette).item(from_repository).item(from_folder)
}

#[cfg(test)]
mod tests {
    use iced::Element;
    use iced_test::simulator;

    use super::*;

    fn click(key: Key) -> Vec<PluginsIntent> {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(Element::from(menu(&palette)));

        ui.click(i18n::t(key)).expect("menu item should be visible");

        ui.into_messages().collect()
    }

    /// The owner, 2026-11-22: the menu adds a plugin from a repository.
    #[test]
    fn add_plugin_in_the_menu_adds_from_a_repository() {
        let sent = click(Key::PluginsAddPlugin);

        assert!(matches!(sent.as_slice(), [PluginsIntent::AddPlugin]));
    }

    /// The owner, 2026-11-22: the menu adds a plugin from a folder.
    #[test]
    fn add_plugin_from_folder_in_the_menu_adds_from_a_folder() {
        let sent = click(Key::PluginsAddFromFolder);

        assert!(matches!(sent.as_slice(), [PluginsIntent::AddFromFolder]));
    }
}
