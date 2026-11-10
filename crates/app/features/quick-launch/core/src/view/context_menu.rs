use iced::widget::{Space, container};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::geometry::{anchor_position, menu_height_for_items};
use otty_ui_kit::spacing;

use crate::event::QuickLaunchIntent;
use crate::model::ContextMenuViewModel;
use crate::types::{ContextMenuAction, ContextMenuTarget};

/// Height of one separator: the 1px line plus its 4px margins.
const SEPARATOR_HEIGHT: f32 = 9.0;

/// Render the quick launch context menu overlay, with the dismiss layer
/// over the main area, kept inside the main area it opened in.
pub(crate) fn view(
    vm: ContextMenuViewModel,
    theme: ThemeProps<'_>,
) -> Element<'_, QuickLaunchIntent> {
    let theme = theme.theme.ui();
    let mut menu = Menu::new(theme);
    let mut item_count = 0;
    let mut separator_count = 0;
    let mut push =
        |menu: Menu<QuickLaunchIntent>, key: Key, action, danger: bool| {
            item_count += 1;
            menu.item(
                MenuItem::new(i18n::t(key))
                    .danger(danger)
                    .on_press(QuickLaunchIntent::ContextMenuAction(action)),
            )
        };

    match &vm.target {
        ContextMenuTarget::Command(_) => {
            if vm.is_running {
                menu = push(menu, Key::Kill, ContextMenuAction::Kill, false);
            }
            menu = push(menu, Key::Edit, ContextMenuAction::Edit, false);
            menu = push(menu, Key::Rename, ContextMenuAction::Rename, false);
            menu =
                push(menu, Key::Duplicate, ContextMenuAction::Duplicate, false);
            separator_count += 1;
            menu = menu.separator();
            menu = push(menu, Key::Remove, ContextMenuAction::Remove, true);
        },
        ContextMenuTarget::Folder(_) => {
            menu = push(
                menu,
                Key::CreateQuickLaunch,
                ContextMenuAction::CreateCommand,
                false,
            );
            menu = push(
                menu,
                Key::CreateFolder,
                ContextMenuAction::CreateFolder,
                false,
            );
            separator_count += 1;
            menu = menu.separator();
            menu = push(menu, Key::Rename, ContextMenuAction::Rename, false);
            separator_count += 1;
            menu = menu.separator();
            menu = push(menu, Key::Delete, ContextMenuAction::Delete, true);
        },
        ContextMenuTarget::Background => {
            menu = push(
                menu,
                Key::CreateLaunch,
                ContextMenuAction::CreateCommand,
                false,
            );
            menu = push(
                menu,
                Key::CreateFolder,
                ContextMenuAction::CreateFolder,
                false,
            );
        },
    }

    let height = menu_height_for_items(item_count)
        + SEPARATOR_HEIGHT * separator_count as f32;
    let anchor = anchor_position(
        vm.cursor,
        vm.area_size,
        menu::MENU_WIDTH,
        height,
        spacing::SPACE_2,
    );

    let positioned_menu =
        container(Element::from(menu)).padding(iced::Padding {
            top: anchor.y,
            right: 0.0,
            bottom: 0.0,
            left: anchor.x,
        });

    // Dismiss layer: a press of any button away from the menu closes it.
    let dismiss_layer = iced::widget::mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(QuickLaunchIntent::ContextMenuDismiss)
    .on_right_press(QuickLaunchIntent::ContextMenuDismiss)
    .on_middle_press(QuickLaunchIntent::ContextMenuDismiss);

    iced::widget::Stack::with_children(vec![
        dismiss_layer.into(),
        positioned_menu.into(),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
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
