use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::{Menu, MenuItem};

use crate::event::QuickLaunchIntent;

/// The Launches menu: New launch and New folder.
pub(crate) fn menu(theme: ThemeProps<'_>) -> Menu<QuickLaunchIntent> {
    let new_launch = MenuItem::new(i18n::t(Key::MenuNewLaunch))
        .on_press(QuickLaunchIntent::HeaderCreateCommand);
    let new_folder = MenuItem::new(i18n::t(Key::MenuNewFolder))
        .on_press(QuickLaunchIntent::HeaderCreateFolder);

    Menu::new(theme.theme.ui())
        .item(new_launch)
        .item(new_folder)
}

#[cfg(test)]
mod tests {
    use iced::Element;
    use iced_test::simulator;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};

    use super::menu;
    use crate::event::QuickLaunchIntent;

    fn click(label: &str) -> Vec<QuickLaunchIntent> {
        let themes = ThemeManager::new();
        let menu = menu(ThemeProps::new(themes.current()));
        let mut ui = simulator(Element::from(menu));

        ui.click(label).expect("menu item should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn given_new_launch_item_when_pressed_then_launch_creation_starts() {
        let intents = click("New launch\u{2026}");

        assert!(matches!(
            intents.as_slice(),
            [QuickLaunchIntent::HeaderCreateCommand]
        ));
    }

    #[test]
    fn given_new_folder_item_when_pressed_then_folder_creation_starts() {
        let intents = click("New folder");

        assert!(matches!(
            intents.as_slice(),
            [QuickLaunchIntent::HeaderCreateFolder]
        ));
    }
}
