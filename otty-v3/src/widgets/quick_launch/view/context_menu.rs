use iced::widget::{Space, column, container};
use iced::{Element, Length, Size, Theme};

use crate::components::primitive::menu_item;
use crate::geometry::anchor_position;
use crate::style::menu_panel_style;
use crate::theme::ThemeProps;
use crate::widgets::quick_launch::event::QuickLaunchUiEvent;
use crate::widgets::quick_launch::model::{
    ContextMenuAction, ContextMenuTarget, LaunchInfo, NodePath,
};
use crate::widgets::quick_launch::state::ContextMenuState;

const MENU_WIDTH: f32 = 220.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_VERTICAL_PADDING: f32 = 16.0;
const MENU_MARGIN: f32 = 6.0;
const MENU_CONTAINER_PADDING: f32 = 8.0;

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
) -> Element<'_, QuickLaunchUiEvent, Theme, iced::Renderer> {
    let mut items: Vec<Element<'_, QuickLaunchUiEvent, Theme, iced::Renderer>> =
        Vec::new();

    match &props.menu.target {
        ContextMenuTarget::Command(path) => {
            let is_launching = props.launching.contains_key(path);
            if is_launching {
                items.push(menu_item_element(
                    "Kill",
                    ContextMenuAction::Kill,
                    props.theme,
                ));
            }
            items.push(menu_item_element(
                "Edit",
                ContextMenuAction::Edit,
                props.theme,
            ));
            items.push(menu_item_element(
                "Rename",
                ContextMenuAction::Rename,
                props.theme,
            ));
            items.push(menu_item_element(
                "Duplicate",
                ContextMenuAction::Duplicate,
                props.theme,
            ));
            items.push(menu_item_element(
                "Remove",
                ContextMenuAction::Remove,
                props.theme,
            ));
        },
        ContextMenuTarget::Folder(_) => {
            items.push(menu_item_element(
                "Create Folder",
                ContextMenuAction::CreateFolder,
                props.theme,
            ));
            items.push(menu_item_element(
                "Create Launch",
                ContextMenuAction::CreateCommand,
                props.theme,
            ));
            items.push(menu_item_element(
                "Rename",
                ContextMenuAction::Rename,
                props.theme,
            ));
            items.push(menu_item_element(
                "Delete",
                ContextMenuAction::Delete,
                props.theme,
            ));
        },
        ContextMenuTarget::Background => {
            items.push(menu_item_element(
                "Create Folder",
                ContextMenuAction::CreateFolder,
                props.theme,
            ));
            items.push(menu_item_element(
                "Create Launch",
                ContextMenuAction::CreateCommand,
                props.theme,
            ));
        },
    }

    let item_count = items.len();
    let menu_height = item_count as f32 * MENU_ITEM_HEIGHT
        + MENU_VERTICAL_PADDING
        + MENU_CONTAINER_PADDING * 2.0;

    let anchor = anchor_position(
        props.menu.cursor,
        props.area_size,
        MENU_WIDTH,
        menu_height,
        MENU_MARGIN,
    );

    let menu_content = column(items).spacing(0);

    let style_fn = menu_panel_style(props.theme);
    let menu_panel = container(menu_content)
        .width(Length::Fixed(MENU_WIDTH))
        .padding(MENU_CONTAINER_PADDING)
        .style(style_fn);

    let positioned_menu = container(menu_panel).padding(iced::Padding {
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
    .on_press(QuickLaunchUiEvent::ContextMenuDismiss);

    iced::widget::Stack::with_children(vec![
        dismiss_layer.into(),
        positioned_menu.into(),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_item_element<'a>(
    label: &'a str,
    action: ContextMenuAction,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickLaunchUiEvent, Theme, iced::Renderer> {
    menu_item::view(menu_item::MenuItemProps { label, theme })
        .map(move |_| QuickLaunchUiEvent::ContextMenuAction(action.clone()))
}
