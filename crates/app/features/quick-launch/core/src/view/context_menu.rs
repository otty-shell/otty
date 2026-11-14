use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::context_menu::context_menu;
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::palette::UiPalette;

use crate::event::QuickLaunchIntent;
use crate::model::ContextMenuViewModel;
use crate::types::{ContextMenuAction, ContextMenuTarget};

/// Render the quick launch context menu at the pointer, kept inside the
/// main area it opened in. A press away from it closes it.
pub(crate) fn view(
    vm: ContextMenuViewModel,
    theme: ThemeProps<'_>,
) -> Element<'_, QuickLaunchIntent> {
    context_menu(
        menu(&vm.target, vm.is_running, theme.theme.ui()),
        vm.cursor,
        vm.area_size,
        QuickLaunchIntent::ContextMenuDismiss,
    )
}

/// The menu of `target`: a launch is edited, renamed, duplicated or
/// removed, and killed while it runs; a folder takes new entries and is
/// renamed or deleted; the empty area takes new entries.
fn menu(
    target: &ContextMenuTarget,
    is_running: bool,
    palette: &UiPalette,
) -> Menu<QuickLaunchIntent> {
    use ContextMenuAction::*;

    let menu = Menu::new(palette);

    match target {
        ContextMenuTarget::Command(_) => {
            let menu = if is_running {
                menu.item(item(Key::Kill, Kill))
            } else {
                menu
            };

            menu.item(item(Key::Edit, Edit))
                .item(item(Key::Rename, Rename))
                .item(item(Key::Duplicate, Duplicate))
                .separator()
                .item(item(Key::Remove, Remove).danger(true))
        },
        ContextMenuTarget::Folder(_) => menu
            .item(item(Key::CreateQuickLaunch, CreateCommand))
            .item(item(Key::CreateFolder, CreateFolder))
            .separator()
            .item(item(Key::Rename, Rename))
            .separator()
            .item(item(Key::Delete, Delete).danger(true)),
        ContextMenuTarget::Background => menu
            .item(item(Key::CreateLaunch, CreateCommand))
            .item(item(Key::CreateFolder, CreateFolder)),
    }
}

fn item(key: Key, action: ContextMenuAction) -> MenuItem<QuickLaunchIntent> {
    MenuItem::new(i18n::t(key))
        .on_press(QuickLaunchIntent::ContextMenuAction(action))
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_testing::mouse::middle_click;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};

    use super::view;
    use crate::event::QuickLaunchIntent;
    use crate::model::ContextMenuViewModel;
    use crate::types::{ContextMenuAction, ContextMenuTarget};

    fn click_item(
        target: ContextMenuTarget,
        is_running: bool,
        key: Key,
    ) -> Vec<QuickLaunchIntent> {
        let vm = ContextMenuViewModel {
            target,
            cursor: Point::new(40.0, 40.0),
            area_size: Size::new(1024.0, 768.0),
            is_running,
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, ThemeProps::new(themes.current())));

        ui.click(i18n::t(key)).expect("menu item should be visible");

        ui.into_messages().collect()
    }

    fn command() -> ContextMenuTarget {
        ContextMenuTarget::Command(vec![String::from("build")])
    }

    fn folder() -> ContextMenuTarget {
        ContextMenuTarget::Folder(vec![String::from("dir")])
    }

    #[test]
    fn given_command_menu_when_edit_pressed_then_edit_action_is_sent() {
        let messages = click_item(command(), false, Key::Edit);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Edit
            )]
        ));
    }

    #[test]
    fn given_command_menu_when_rename_pressed_then_rename_action_is_sent() {
        let messages = click_item(command(), false, Key::Rename);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Rename
            )]
        ));
    }

    #[test]
    fn given_command_menu_when_duplicate_pressed_then_duplicate_action_is_sent()
    {
        let messages = click_item(command(), false, Key::Duplicate);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Duplicate
            )]
        ));
    }

    #[test]
    fn given_command_menu_when_remove_pressed_then_remove_action_is_sent() {
        let messages = click_item(command(), false, Key::Remove);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Remove
            )]
        ));
    }

    #[test]
    fn given_running_command_when_kill_pressed_then_kill_action_is_sent() {
        let messages = click_item(command(), true, Key::Kill);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Kill
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_create_folder_pressed_then_action_is_sent() {
        let messages = click_item(folder(), false, Key::CreateFolder);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::CreateFolder
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_create_launch_pressed_then_action_is_sent() {
        let messages = click_item(folder(), false, Key::CreateQuickLaunch);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::CreateCommand
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_rename_pressed_then_rename_action_is_sent() {
        let messages = click_item(folder(), false, Key::Rename);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Rename
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_delete_pressed_then_delete_action_is_sent() {
        let messages = click_item(folder(), false, Key::Delete);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Delete
            )]
        ));
    }

    /// A right press away from the menu closes it, as a left press does,
    /// instead of falling through to the tree or the terminal under it.
    #[test]
    fn a_right_press_outside_the_menu_closes_it() {
        let vm = ContextMenuViewModel {
            target: command(),
            cursor: Point::new(40.0, 40.0),
            area_size: Size::new(1024.0, 768.0),
            is_running: false,
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, ThemeProps::new(themes.current())));

        ui.point_at(Point::new(900.0, 700.0));
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )),
        ]);

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuDismiss]
        ));
    }

    /// A middle press away from the menu closes it too, instead of falling
    /// through to the Sessions row or the terminal under it.
    #[test]
    fn a_middle_press_outside_the_menu_closes_it() {
        let vm = ContextMenuViewModel {
            target: command(),
            cursor: Point::new(40.0, 40.0),
            area_size: Size::new(1024.0, 768.0),
            is_running: false,
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, ThemeProps::new(themes.current())));

        ui.point_at(Point::new(900.0, 700.0));
        let _ = ui.simulate(middle_click());

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuDismiss]
        ));
    }

    #[test]
    fn given_background_menu_when_create_launch_pressed_then_action_is_sent() {
        let messages =
            click_item(ContextMenuTarget::Background, false, Key::CreateLaunch);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::CreateCommand
            )]
        ));
    }
}
