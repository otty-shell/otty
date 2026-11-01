use iced::Element;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::size;

use super::super::event::QuickLaunchIntent;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Number of items in the menu: New launch and New folder.
const ITEM_COUNT: f32 = 2.0;

/// Props for the Launches menu under the Launches header chevron.
///
/// The menu is generic over the message type: the caller wraps each intent.
pub(crate) struct NewLaunchMenuProps<'a, Message> {
    pub(crate) theme: ThemeProps<'a>,
    /// Turns a quick launch intent into the caller's message.
    pub(crate) on_intent: fn(QuickLaunchIntent) -> Message,
}

/// Height of the menu, used to keep it inside the window.
pub(crate) fn menu_height() -> f32 {
    2.0 * menu::MENU_PADDING + size::ROW * ITEM_COUNT
}

/// Render the Launches menu: New launch and New folder.
pub(crate) fn view<'a, Message: Clone + 'a>(
    props: NewLaunchMenuProps<'a, Message>,
) -> Element<'a, Message> {
    let palette = props.theme.theme.ui();
    let on_intent = props.on_intent;

    let new_launch = MenuItem::new(i18n::t(Key::MenuNewLaunch))
        .on_press(on_intent(QuickLaunchIntent::HeaderCreateCommand));
    let new_folder = MenuItem::new(i18n::t(Key::MenuNewFolder))
        .on_press(on_intent(QuickLaunchIntent::HeaderCreateFolder));

    Menu::new(palette).item(new_launch).item(new_folder).into()
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::{NewLaunchMenuProps, menu_height, view};
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::quick_launch::QuickLaunchIntent;

    fn click(label: &str) -> Vec<QuickLaunchIntent> {
        let themes = ThemeManager::new();
        let menu = container(view(NewLaunchMenuProps {
            theme: ThemeProps::new(themes.current()),
            on_intent: |intent| intent,
        }));
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
