use iced::widget::{Space, container};
use iced::{Element, Length, Size, Theme};
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::spacing;

use super::super::event::QuickLaunchIntent;
use super::super::state::ContextMenuState;
use super::super::types::{
    ContextMenuAction, ContextMenuTarget, LaunchInfo, NodePath,
};
use crate::geometry::{anchor_position, menu_height_for_items};
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Height of one separator: the 1px line plus its 4px margins.
const SEPARATOR_HEIGHT: f32 = 9.0;

/// Props for the quick launch context menu.
pub(crate) struct ContextMenuProps<'a> {
    pub(crate) menu: &'a ContextMenuState,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) area_size: Size,
    pub(crate) launching: &'a std::collections::HashMap<NodePath, LaunchInfo>,
}

/// Render the quick launch context menu overlay.
pub(crate) fn view(
    props: ContextMenuProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    let theme = props.theme.theme.ui();
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

    match &props.menu.target() {
        ContextMenuTarget::Command(path) => {
            if props.launching.contains_key(path) {
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
        props.menu.cursor(),
        props.area_size,
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

    // Dismiss layer
    let dismiss_layer = iced::widget::mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(QuickLaunchIntent::ContextMenuDismiss);

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
    use std::collections::HashMap;

    use iced::{Point, Size};
    use iced_test::simulator;

    use super::{ContextMenuProps, view};
    use crate::i18n::{self, Key};
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::quick_launch::event::QuickLaunchIntent;
    use crate::widgets::quick_launch::state::ContextMenuState;
    use crate::widgets::quick_launch::types::{
        ContextMenuAction, ContextMenuTarget, LaunchInfo, NodePath,
    };

    fn click_item(
        target: ContextMenuTarget,
        launching: &HashMap<NodePath, LaunchInfo>,
        key: Key,
    ) -> Vec<QuickLaunchIntent> {
        let menu = ContextMenuState::default()
            .with_target(target)
            .with_cursor(Point::new(40.0, 40.0));
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ContextMenuProps {
            menu: &menu,
            theme: ThemeProps::new(themes.current()),
            area_size: Size::new(1024.0, 768.0),
            launching,
        }));

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
        let messages = click_item(command(), &HashMap::new(), Key::Edit);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Edit
            )]
        ));
    }

    #[test]
    fn given_command_menu_when_rename_pressed_then_rename_action_is_sent() {
        let messages = click_item(command(), &HashMap::new(), Key::Rename);

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
        let messages = click_item(command(), &HashMap::new(), Key::Duplicate);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Duplicate
            )]
        ));
    }

    #[test]
    fn given_command_menu_when_remove_pressed_then_remove_action_is_sent() {
        let messages = click_item(command(), &HashMap::new(), Key::Remove);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Remove
            )]
        ));
    }

    #[test]
    fn given_running_command_when_kill_pressed_then_kill_action_is_sent() {
        let path = vec![String::from("build")];
        let mut launching = HashMap::new();
        launching.insert(
            path.clone(),
            LaunchInfo {
                id: 1,
                launch_ticks: 0,
                is_indicator_highlighted: false,
                cancel: Default::default(),
            },
        );

        let messages =
            click_item(ContextMenuTarget::Command(path), &launching, Key::Kill);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Kill
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_create_folder_pressed_then_action_is_sent() {
        let messages = click_item(folder(), &HashMap::new(), Key::CreateFolder);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::CreateFolder
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_create_launch_pressed_then_action_is_sent() {
        let messages =
            click_item(folder(), &HashMap::new(), Key::CreateQuickLaunch);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::CreateCommand
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_rename_pressed_then_rename_action_is_sent() {
        let messages = click_item(folder(), &HashMap::new(), Key::Rename);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Rename
            )]
        ));
    }

    #[test]
    fn given_folder_menu_when_delete_pressed_then_delete_action_is_sent() {
        let messages = click_item(folder(), &HashMap::new(), Key::Delete);

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::Delete
            )]
        ));
    }

    #[test]
    fn given_background_menu_when_create_launch_pressed_then_action_is_sent() {
        let messages = click_item(
            ContextMenuTarget::Background,
            &HashMap::new(),
            Key::CreateLaunch,
        );

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::ContextMenuAction(
                ContextMenuAction::CreateCommand
            )]
        ));
    }
}
