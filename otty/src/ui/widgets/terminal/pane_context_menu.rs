use iced::border::Radius;
use iced::widget::text_input;
use iced::widget::{Column, Id, container, mouse_area};
use iced::{Background, Element, Length, Point, Size, alignment};

use crate::features::terminal::event::TerminalEvent;
use crate::features::terminal::pane_context_menu::PaneContextMenuState;
use crate::theme::ThemeProps;
use crate::ui::components::menu_item::{
    MenuItem, MenuItemEvent, MenuItemProps,
};

const MENU_CONTAINER_WIDTH: f32 = 250.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_VERTICAL_PADDING: f32 = 20.0;
const MENU_MARGIN: f32 = 6.0;
const MENU_CONTAINER_PADDING_X: f32 = 10.0;

/// Props for rendering a pane context menu.
#[derive(Debug, Clone, Copy)]
pub(super) struct Props<'a> {
    pub(super) menu: &'a PaneContextMenuState,
    pub(super) has_block_selection: bool,
    pub(super) theme: ThemeProps<'a>,
}

pub fn view<'a>(props: Props<'a>) -> Element<'a, TerminalEvent> {
    let mut buttons: Vec<Element<'a, TerminalEvent>> = Vec::new();

    buttons.push(menu_item(
        "Copy selection",
        props.theme,
        TerminalEvent::CopySelection {
            terminal_id: props.menu.terminal_id,
        },
    ));
    buttons.push(menu_item(
        "Paste",
        props.theme,
        TerminalEvent::PasteIntoPrompt {
            terminal_id: props.menu.terminal_id,
        },
    ));

    if props.has_block_selection {
        buttons.push(menu_item(
            "Copy content",
            props.theme,
            TerminalEvent::CopySelectedBlockContent {
                terminal_id: props.menu.terminal_id,
            },
        ));
        buttons.push(menu_item(
            "Copy prompt",
            props.theme,
            TerminalEvent::CopySelectedBlockPrompt {
                terminal_id: props.menu.terminal_id,
            },
        ));
        buttons.push(menu_item(
            "Copy command",
            props.theme,
            TerminalEvent::CopySelectedBlockCommand {
                terminal_id: props.menu.terminal_id,
            },
        ));
    }

    buttons.push(menu_item(
        "Split horizontally",
        props.theme,
        TerminalEvent::SplitPane {
            pane: props.menu.pane,
            axis: iced::widget::pane_grid::Axis::Horizontal,
        },
    ));
    buttons.push(menu_item(
        "Split vertically",
        props.theme,
        TerminalEvent::SplitPane {
            pane: props.menu.pane,
            axis: iced::widget::pane_grid::Axis::Vertical,
        },
    ));
    buttons.push(menu_item(
        "Close",
        props.theme,
        TerminalEvent::ClosePane {
            pane: props.menu.pane,
        },
    ));

    let menu_height = menu_height_for_items(buttons.len());
    let menu_column = buttons
        .into_iter()
        .fold(Column::new(), |column, button| column.push(button));
    let menu_column = menu_column
        .spacing(0)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    let anchor =
        anchor_position(props.menu.cursor, props.menu.grid_size, menu_height);

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
    .on_press(TerminalEvent::CloseContextMenu)
    .on_right_press(TerminalEvent::CloseContextMenu);

    iced::widget::stack!(
        dismiss_layer,
        positioned_menu,
        focus_trap(props.menu.focus_target.clone())
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn menu_item<'a>(
    label: &'a str,
    theme: ThemeProps<'a>,
    event: TerminalEvent,
) -> Element<'a, TerminalEvent> {
    let props = MenuItemProps { label, theme };
    MenuItem::new(props)
        .view()
        .map(move |item_event| match item_event {
            MenuItemEvent::Pressed => event.clone(),
        })
}

fn focus_trap(id: Id) -> Element<'static, TerminalEvent> {
    text_input("", "")
        .on_input(|_| TerminalEvent::ContextMenuInput)
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

fn menu_panel_style(
    theme: ThemeProps<'_>,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Style + 'static {
    let palette = theme.theme.iced_palette().clone();
    move |_theme: &iced::Theme| iced::widget::container::Style {
        background: Some(palette.overlay.into()),
        text_color: Some(palette.foreground),
        border: iced::Border {
            width: 0.25,
            color: palette.overlay,
            radius: Radius::new(4.0),
        },

        ..Default::default()
    }
}

pub(crate) fn anchor_position(
    cursor: Point,
    grid_size: Size,
    menu_height: f32,
) -> Point {
    let clamped_cursor = Point::new(
        cursor.x.clamp(0.0, grid_size.width),
        cursor.y.clamp(0.0, grid_size.height),
    );

    let fits_right = clamped_cursor.x + MENU_CONTAINER_WIDTH + MENU_MARGIN
        <= grid_size.width;
    let x = if fits_right {
        (clamped_cursor.x + MENU_MARGIN)
            .min(grid_size.width - MENU_MARGIN - MENU_CONTAINER_WIDTH)
    } else {
        (clamped_cursor.x - MENU_CONTAINER_WIDTH - MENU_MARGIN).max(MENU_MARGIN)
    };

    let fits_down =
        clamped_cursor.y + menu_height + MENU_MARGIN <= grid_size.height;
    let y = if fits_down {
        (clamped_cursor.y + MENU_MARGIN)
            .min(grid_size.height - MENU_MARGIN - menu_height)
    } else {
        (clamped_cursor.y - menu_height - MENU_MARGIN).max(MENU_MARGIN)
    };

    let max_x =
        (grid_size.width - MENU_CONTAINER_WIDTH - MENU_MARGIN).max(MENU_MARGIN);
    let max_y = (grid_size.height - menu_height - MENU_MARGIN).max(MENU_MARGIN);

    Point::new(x.clamp(MENU_MARGIN, max_x), y.clamp(MENU_MARGIN, max_y))
}

pub(crate) fn menu_height_for_items(item_count: usize) -> f32 {
    MENU_VERTICAL_PADDING + MENU_ITEM_HEIGHT * item_count as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_position_clamps_inside_bounds() {
        let grid = Size::new(400.0, 300.0);
        let cursor = Point::new(390.0, 290.0);
        let menu_height = menu_height_for_items(5);
        let anchor = anchor_position(cursor, grid, menu_height);
        assert!(anchor.x >= MENU_MARGIN);
        assert!(anchor.y >= MENU_MARGIN);
        assert!(
            anchor.x + MENU_CONTAINER_WIDTH <= grid.width - MENU_MARGIN + 0.1
        );
        assert!(anchor.y + menu_height <= grid.height - MENU_MARGIN + 0.1);
    }

    #[test]
    fn anchor_position_stays_near_cursor_when_space_allows() {
        let grid = Size::new(800.0, 600.0);
        let cursor = Point::new(100.0, 120.0);
        let menu_height = menu_height_for_items(5);
        let anchor = anchor_position(cursor, grid, menu_height);
        assert!((anchor.x - (cursor.x + MENU_MARGIN)).abs() < 0.1);
        assert!((anchor.y - (cursor.y + MENU_MARGIN)).abs() < 0.1);
    }

    #[test]
    fn anchor_position_flips_when_near_right_edge() {
        let grid = Size::new(500.0, 400.0);
        let cursor = Point::new(490.0, 200.0);
        let menu_height = menu_height_for_items(5);
        let anchor = anchor_position(cursor, grid, menu_height);
        assert!(anchor.x < cursor.x);
        assert!(cursor.x - anchor.x >= MENU_CONTAINER_WIDTH - MENU_MARGIN);
    }
}
