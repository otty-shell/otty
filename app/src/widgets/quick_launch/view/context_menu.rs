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
    let mut push = |menu: Menu<QuickLaunchIntent>, key: Key, action| {
        item_count += 1;
        menu.item(
            MenuItem::new(i18n::t(key))
                .on_press(QuickLaunchIntent::ContextMenuAction(action)),
        )
    };

    match &props.menu.target() {
        ContextMenuTarget::Command(path) => {
            if props.launching.contains_key(path) {
                menu = push(menu, Key::Kill, ContextMenuAction::Kill);
            }
            menu = push(menu, Key::Edit, ContextMenuAction::Edit);
            menu = push(menu, Key::Rename, ContextMenuAction::Rename);
            menu = push(menu, Key::Duplicate, ContextMenuAction::Duplicate);
            menu = push(menu, Key::Remove, ContextMenuAction::Remove);
        },
        ContextMenuTarget::Folder(_) => {
            menu =
                push(menu, Key::CreateFolder, ContextMenuAction::CreateFolder);
            menu = push(
                menu,
                Key::CreateQuickLaunch,
                ContextMenuAction::CreateCommand,
            );
            menu = push(menu, Key::Rename, ContextMenuAction::Rename);
            menu = push(menu, Key::Delete, ContextMenuAction::Delete);
        },
        ContextMenuTarget::Background => {
            menu =
                push(menu, Key::CreateFolder, ContextMenuAction::CreateFolder);
            menu =
                push(menu, Key::CreateLaunch, ContextMenuAction::CreateCommand);
        },
    }

    let anchor = anchor_position(
        props.menu.cursor(),
        props.area_size,
        menu::MENU_WIDTH,
        menu_height_for_items(item_count),
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
