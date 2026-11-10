use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::size;

use crate::event::QuickLaunchIntent;

/// Number of items in the menu: New launch and New folder.
const ITEM_COUNT: f32 = 2.0;

/// Height of the menu, used to keep it inside the window.
pub(crate) fn menu_height() -> f32 {
    2.0 * menu::MENU_PADDING + size::ROW * ITEM_COUNT
}

/// Render the Launches menu: New launch and New folder.
pub(crate) fn view(theme: ThemeProps<'_>) -> Element<'_, QuickLaunchIntent> {
    let palette = theme.theme.ui();

    let new_launch = MenuItem::new(i18n::t(Key::MenuNewLaunch))
        .on_press(QuickLaunchIntent::HeaderCreateCommand);
    let new_folder = MenuItem::new(i18n::t(Key::MenuNewFolder))
        .on_press(QuickLaunchIntent::HeaderCreateFolder);

    Menu::new(palette).item(new_launch).item(new_folder).into()
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};
    use otty_ui_kit::size;

    use super::{menu_height, view};
    use crate::event::QuickLaunchIntent;

    fn click(label: &str) -> Vec<QuickLaunchIntent> {
        let themes = ThemeManager::new();
        let menu = container(view(ThemeProps::new(themes.current())));
        let mut ui = simulator(menu);

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

    #[test]
    fn given_menu_when_height_computed_then_it_fits_two_rows() {
        assert!(menu_height() > 2.0 * size::ROW);
    }
}
