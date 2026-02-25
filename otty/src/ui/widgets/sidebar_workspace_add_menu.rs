use iced::widget::{Column, container, mouse_area};
use iced::{Element, Length, Point, Size, alignment};

use crate::theme::ThemeProps;
use crate::ui::components::menu_item::{
    MenuItemEvent, MenuItemProps, view as menu_item_view,
};
use crate::ui::components::widget_helpers::{
    anchor_position, menu_height_for_items, menu_panel_style,
};
use crate::ui::widgets::sidebar_workspace::{
    SidebarWorkspaceAddMenuAction, SidebarWorkspaceEvent,
};

const MENU_CONTAINER_WIDTH: f32 = 220.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_VERTICAL_PADDING: f32 = 16.0;
const MENU_MARGIN: f32 = 6.0;
const MENU_CONTAINER_PADDING_X: f32 = 8.0;

/// Props for rendering the terminal add menu.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarWorkspaceAddMenuProps<'a> {
    pub(crate) cursor: Point,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) area_size: Size,
}

/// Events emitted by sidebar workspace add menu widget.
pub(crate) type SidebarWorkspaceAddMenuEvent = SidebarWorkspaceEvent;

pub(crate) fn view<'a>(
    props: SidebarWorkspaceAddMenuProps<'a>,
) -> Element<'a, SidebarWorkspaceAddMenuEvent> {
    let items = [
        menu_item(
            "Create tab",
            props.theme,
            SidebarWorkspaceAddMenuAction::CreateTab,
        ),
        menu_item(
            "Create command",
            props.theme,
            SidebarWorkspaceAddMenuAction::CreateCommand,
        ),
        menu_item(
            "Create folder",
            props.theme,
            SidebarWorkspaceAddMenuAction::CreateFolder,
        ),
    ];

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
        props.cursor,
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
    .on_press(SidebarWorkspaceEvent::TerminalAddMenuDismiss)
    .on_right_press(SidebarWorkspaceEvent::TerminalAddMenuDismiss)
    .on_move(|position| SidebarWorkspaceEvent::WorkspaceCursorMoved {
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
    action: SidebarWorkspaceAddMenuAction,
) -> Element<'a, SidebarWorkspaceAddMenuEvent> {
    let props = MenuItemProps { label, theme };
    menu_item_view(props).map(move |event| match event {
        MenuItemEvent::Pressed => {
            SidebarWorkspaceEvent::TerminalAddMenuAction(action)
        },
    })
}
