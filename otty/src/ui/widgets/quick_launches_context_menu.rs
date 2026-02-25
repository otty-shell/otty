use std::collections::HashMap;

use iced::widget::{Column, container, mouse_area};
use iced::{Element, Length, Size, alignment};

use super::services::{
    anchor_position, menu_height_for_items, menu_panel_style,
};
use crate::features::quick_launch::{
    ContextMenuAction, ContextMenuState, ContextMenuTarget, LaunchInfo,
    NodePath, QuickLaunchEvent as FeatureQuickLaunchEvent,
};
use crate::theme::ThemeProps;
use crate::ui::components::menu_item::{
    MenuItemEvent, MenuItemProps, view as menu_item_view,
};

const MENU_CONTAINER_WIDTH: f32 = 220.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_VERTICAL_PADDING: f32 = 16.0;
const MENU_MARGIN: f32 = 6.0;
const MENU_CONTAINER_PADDING_X: f32 = 8.0;

/// Props for rendering the quick launches context menu.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QuickLaunchesContextMenuProps<'a> {
    pub(crate) menu: &'a ContextMenuState,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) area_size: Size,
    pub(crate) launching: &'a HashMap<NodePath, LaunchInfo>,
}

/// Events emitted by quick launch context menu widget.
pub(crate) type QuickLaunchesContextMenuEvent = FeatureQuickLaunchEvent;

pub(crate) fn view<'a>(
    props: QuickLaunchesContextMenuProps<'a>,
) -> Element<'a, QuickLaunchesContextMenuEvent> {
    let mut items: Vec<Element<'a, QuickLaunchesContextMenuEvent>> = Vec::new();

    match props.menu.target() {
        ContextMenuTarget::Command(path) => {
            let is_launching = props.launching.contains_key(path);
            if is_launching {
                items.push(menu_item(
                    "Kill",
                    props.theme,
                    ContextMenuAction::Kill,
                ));
            }
            items.push(menu_item("Edit", props.theme, ContextMenuAction::Edit));
            items.push(menu_item(
                "Rename",
                props.theme,
                ContextMenuAction::Rename,
            ));
            items.push(menu_item(
                "Duplicate",
                props.theme,
                ContextMenuAction::Duplicate,
            ));
            items.push(menu_item(
                "Remove",
                props.theme,
                ContextMenuAction::Remove,
            ));
        },
        ContextMenuTarget::Folder(_) => {
            items.push(menu_item(
                "Create folder",
                props.theme,
                ContextMenuAction::CreateFolder,
            ));
            items.push(menu_item(
                "Create launch",
                props.theme,
                ContextMenuAction::CreateCommand,
            ));
            items.push(menu_item(
                "Rename",
                props.theme,
                ContextMenuAction::Rename,
            ));
            items.push(menu_item(
                "Delete",
                props.theme,
                ContextMenuAction::Delete,
            ));
        },
        ContextMenuTarget::Background => {
            items.push(menu_item(
                "Create folder",
                props.theme,
                ContextMenuAction::CreateFolder,
            ));
            items.push(menu_item(
                "Create launch",
                props.theme,
                ContextMenuAction::CreateCommand,
            ));
        },
    }

    let menu_height = menu_height_for_items(
        items.len(),
        MENU_ITEM_HEIGHT,
        MENU_VERTICAL_PADDING,
    );
    let menu_column = items
        .into_iter()
        .fold(Column::new(), |column, button| column.push(button))
        .spacing(0)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    let anchor = anchor_position(
        props.menu.cursor(),
        props.area_size,
        MENU_CONTAINER_WIDTH,
        menu_height,
        MENU_MARGIN,
    );

    let padding = iced::Padding {
        top: anchor.y,
        left: anchor.x,
        ..iced::Padding::ZERO
    };

    let menu_container = container(menu_column)
        .padding([MENU_CONTAINER_PADDING_X, 0.0])
        .width(MENU_CONTAINER_WIDTH)
        .style(menu_panel_style(props.theme));

    let positioned_menu = container(menu_container)
        .padding(padding)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top);

    let dismiss_layer = mouse_area(
        container(iced::widget::text(""))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(QuickLaunchesContextMenuEvent::ContextMenuDismiss)
    .on_right_press(QuickLaunchesContextMenuEvent::ContextMenuDismiss)
    .on_move(|position| QuickLaunchesContextMenuEvent::CursorMoved {
        position,
    });

    iced::widget::stack!(dismiss_layer, positioned_menu)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn menu_item<'a>(
    label: &'a str,
    theme: ThemeProps<'a>,
    action: ContextMenuAction,
) -> Element<'a, QuickLaunchesContextMenuEvent> {
    let props = MenuItemProps { label, theme };
    menu_item_view(props).map(move |event| match event {
        MenuItemEvent::Pressed => {
            QuickLaunchesContextMenuEvent::ContextMenuAction(action)
        },
    })
}
