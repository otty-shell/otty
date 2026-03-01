use iced::widget::{Column, Id, container, mouse_area, pane_grid, text_input};
use iced::{Background, Element, Length, Point, Size, alignment};

use crate::components::primitive::menu_item::{
    MenuItemEvent, MenuItemProps, view as menu_item_view,
};
use crate::geometry::{anchor_position, menu_height_for_items};
use crate::style::menu_panel_style;
use crate::theme::ThemeProps;
use crate::widgets::terminal_workspace::event::TerminalWorkspaceUiEvent;

const MENU_CONTAINER_WIDTH: f32 = 250.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_VERTICAL_PADDING: f32 = 20.0;
const MENU_MARGIN: f32 = 6.0;
const MENU_CONTAINER_PADDING_X: f32 = 10.0;

/// Props for rendering a pane context menu.
#[derive(Debug, Clone)]
pub(crate) struct PaneContextMenuProps<'a> {
    pub(crate) tab_id: u64,
    pub(crate) pane: pane_grid::Pane,
    pub(crate) cursor: Point,
    pub(crate) grid_size: Size,
    pub(crate) terminal_id: u64,
    pub(crate) focus_target: Id,
    pub(crate) has_block_selection: bool,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the context menu overlay for a terminal pane.
pub(crate) fn view<'a>(
    props: PaneContextMenuProps<'a>,
) -> Element<'a, TerminalWorkspaceUiEvent> {
    let mut buttons: Vec<Element<'a, TerminalWorkspaceUiEvent>> = Vec::new();

    buttons.push(menu_item(
        "Copy selection",
        props.theme,
        TerminalWorkspaceUiEvent::CopySelection {
            tab_id: props.tab_id,
            terminal_id: props.terminal_id,
        },
    ));
    buttons.push(menu_item(
        "Paste",
        props.theme,
        TerminalWorkspaceUiEvent::PasteIntoPrompt {
            tab_id: props.tab_id,
            terminal_id: props.terminal_id,
        },
    ));

    if props.has_block_selection {
        buttons.push(menu_item(
            "Copy content",
            props.theme,
            TerminalWorkspaceUiEvent::CopySelectedBlockContent {
                tab_id: props.tab_id,
                terminal_id: props.terminal_id,
            },
        ));
        buttons.push(menu_item(
            "Copy prompt",
            props.theme,
            TerminalWorkspaceUiEvent::CopySelectedBlockPrompt {
                tab_id: props.tab_id,
                terminal_id: props.terminal_id,
            },
        ));
        buttons.push(menu_item(
            "Copy command",
            props.theme,
            TerminalWorkspaceUiEvent::CopySelectedBlockCommand {
                tab_id: props.tab_id,
                terminal_id: props.terminal_id,
            },
        ));
    }

    buttons.push(menu_item(
        "Split horizontally",
        props.theme,
        TerminalWorkspaceUiEvent::SplitPane {
            tab_id: props.tab_id,
            pane: props.pane,
            axis: pane_grid::Axis::Horizontal,
        },
    ));
    buttons.push(menu_item(
        "Split vertically",
        props.theme,
        TerminalWorkspaceUiEvent::SplitPane {
            tab_id: props.tab_id,
            pane: props.pane,
            axis: pane_grid::Axis::Vertical,
        },
    ));
    buttons.push(menu_item(
        "Close",
        props.theme,
        TerminalWorkspaceUiEvent::ClosePane {
            tab_id: props.tab_id,
            pane: props.pane,
        },
    ));

    let menu_height = menu_height_for_items(
        buttons.len(),
        MENU_ITEM_HEIGHT,
        MENU_VERTICAL_PADDING,
    );
    let menu_column = buttons
        .into_iter()
        .fold(Column::new(), |column, button| column.push(button));
    let menu_column = menu_column
        .spacing(0)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    let anchor = anchor_position(
        props.cursor,
        props.grid_size,
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
    .on_press(TerminalWorkspaceUiEvent::CloseContextMenu {
        tab_id: props.tab_id,
    })
    .on_right_press(TerminalWorkspaceUiEvent::CloseContextMenu {
        tab_id: props.tab_id,
    })
    .on_move(move |position| {
        TerminalWorkspaceUiEvent::PaneGridCursorMoved {
            tab_id: props.tab_id,
            position,
        }
    });

    iced::widget::stack!(
        dismiss_layer,
        positioned_menu,
        focus_trap(props.tab_id, props.focus_target.clone())
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_item<'a>(
    label: &'a str,
    theme: ThemeProps<'a>,
    event: TerminalWorkspaceUiEvent,
) -> Element<'a, TerminalWorkspaceUiEvent> {
    let props = MenuItemProps { label, theme };
    menu_item_view(props).map(move |item_event| match item_event {
        MenuItemEvent::Pressed => event.clone(),
    })
}

fn focus_trap(
    tab_id: u64,
    id: Id,
) -> Element<'static, TerminalWorkspaceUiEvent> {
    text_input("", "")
        .on_input(move |_| TerminalWorkspaceUiEvent::ContextMenuInput {
            tab_id,
        })
        .padding(0)
        .size(1)
        .width(Length::Fixed(1.0))
        .id(id)
        .style(|theme: &iced::Theme, _status| {
            let color = theme.extended_palette().background.base.color;
            iced::widget::text_input::Style {
                background: Background::Color(color),
                border: iced::Border {
                    width: 0.0,
                    ..Default::default()
                },
                icon: color,
                placeholder: color,
                value: color,
                selection: color,
            }
        })
        .into()
}
