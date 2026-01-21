use iced::border::Radius;
use iced::widget::button::Status as ButtonStatus;
use iced::widget::pane_grid;
use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Id, button, container, mouse_area, text, text_input,
};
use iced::{Background, Element, Length, Point, Size, Task, Theme, alignment};

use crate::app::Event;
use crate::tab::TabBlockSelection;
use crate::theme::IcedColorPalette;

const MENU_CONTAINER_WIDTH: f32 = 250.0;
const MENU_ITEM_FONT_SIZE: f32 = 13.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_VERTICAL_PADDING: f32 = 20.0;
const MENU_MARGIN: f32 = 6.0;

#[derive(Debug, Clone)]
pub struct PaneContextMenu {
    pub pane: pane_grid::Pane,
    cursor: Point,
    grid_size: Size,
    pub terminal_id: u64,
    focus_target: Id,
    pub theme: IcedColorPalette,
}

impl PaneContextMenu {
    pub fn new(
        pane: pane_grid::Pane,
        cursor: Point,
        grid_size: Size,
        theme: &IcedColorPalette,
        terminal_id: u64,
    ) -> Self {
        Self {
            pane,
            cursor,
            grid_size,
            terminal_id,
            focus_target: Id::unique(),
            theme: theme.clone(),
        }
    }

    pub fn anchor_for_height(&self, menu_height: f32) -> Point {
        anchor_position(self.cursor, self.grid_size, menu_height)
    }

    pub fn focus_task<Message: 'static>(&self) -> Task<Message> {
        iced::widget::operation::focus(self.focus_target.clone())
    }
}

pub fn overlay(
    tab_id: u64,
    menu: &PaneContextMenu,
    selection: Option<TabBlockSelection>,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let pane = menu.pane;
    let has_block_selection = selection.is_some();

    let mut buttons: Vec<Element<'static, Event, Theme, iced::Renderer>> =
        Vec::new();
    buttons.push(context_menu_copy_selection_button(
        tab_id,
        menu.terminal_id,
        &menu.theme,
    ));
    buttons.push(context_menu_paste_button(
        tab_id,
        menu.terminal_id,
        &menu.theme,
    ));

    if has_block_selection {
        buttons.push(context_menu_copy_content_button(
            tab_id,
            menu.terminal_id,
            &menu.theme,
        ));
        buttons.push(context_menu_copy_prompt_button(
            tab_id,
            menu.terminal_id,
            &menu.theme,
        ));
        buttons.push(context_menu_copy_command_button(
            tab_id,
            menu.terminal_id,
            &menu.theme,
        ));
    }

    buttons.push(context_menu_button(
        "Split horizontally",
        tab_id,
        pane,
        pane_grid::Axis::Horizontal,
        &menu.theme,
    ));
    buttons.push(context_menu_button(
        "Split vertically",
        tab_id,
        pane,
        pane_grid::Axis::Vertical,
        &menu.theme,
    ));
    buttons.push(context_menu_close_button(tab_id, pane, &menu.theme));

    let menu_height = menu_height_for_items(buttons.len());
    let menu_column = buttons
        .into_iter()
        .fold(Column::new(), |column, button| column.push(button));
    let menu_column = menu_column
        .spacing(0)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    let anchor = menu.anchor_for_height(menu_height);

    let padding = iced::Padding {
        top: anchor.y,
        left: anchor.x,
        ..iced::Padding::ZERO
    };

    let menu_container = container(menu_column)
        .padding([10, 0])
        .width(MENU_CONTAINER_WIDTH)
        .style(menu_panel_style(menu.theme.clone()));

    let positioned_menu = container(menu_container)
        .padding(padding)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top);

    let dismiss_layer = mouse_area(
        container(text("")).width(Length::Fill).height(Length::Fill),
    )
    .on_press(Event::ClosePaneContextMenu { tab_id })
    .on_right_press(Event::ClosePaneContextMenu { tab_id });

    iced::widget::stack!(
        dismiss_layer,
        positioned_menu,
        focus_trap(menu.focus_target.clone())
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn context_menu_button(
    label: &'static str,
    tab_id: u64,
    pane: pane_grid::Pane,
    axis: pane_grid::Axis,
    theme: &IcedColorPalette,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let label = text(label)
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([1, 10])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(menu_button_style(theme.clone()))
        .on_press(Event::SplitPane { tab_id, pane, axis })
        .into()
}

fn context_menu_copy_selection_button(
    tab_id: u64,
    terminal_id: u64,
    theme: &IcedColorPalette,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let label = text("Copy selection")
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([1, 10])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(menu_button_style(theme.clone()))
        .on_press(Event::CopySelection {
            tab_id,
            terminal_id,
        })
        .into()
}

fn context_menu_paste_button(
    tab_id: u64,
    terminal_id: u64,
    theme: &IcedColorPalette,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let label = text("Paste")
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([1, 10])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(menu_button_style(theme.clone()))
        .on_press(Event::PasteIntoPrompt {
            tab_id,
            terminal_id,
        })
        .into()
}

fn context_menu_copy_content_button(
    tab_id: u64,
    terminal_id: u64,
    theme: &IcedColorPalette,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let label = text("Copy content")
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([1, 10])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(menu_button_style(theme.clone()))
        .on_press(Event::CopySelectedBlockContent {
            tab_id,
            terminal_id,
        })
        .into()
}

fn context_menu_copy_prompt_button(
    tab_id: u64,
    terminal_id: u64,
    theme: &IcedColorPalette,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let label = text("Copy prompt")
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([1, 10])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(menu_button_style(theme.clone()))
        .on_press(Event::CopySelectedBlockPrompt {
            tab_id,
            terminal_id,
        })
        .into()
}

fn context_menu_copy_command_button(
    tab_id: u64,
    terminal_id: u64,
    theme: &IcedColorPalette,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let label = text("Copy command")
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([1, 10])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(menu_button_style(theme.clone()))
        .on_press(Event::CopySelectedBlockCommand {
            tab_id,
            terminal_id,
        })
        .into()
}

fn context_menu_close_button(
    tab_id: u64,
    pane: pane_grid::Pane,
    theme: &IcedColorPalette,
) -> Element<'static, Event, Theme, iced::Renderer> {
    let label = text("Close")
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([1, 10])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(menu_button_style(theme.clone()))
        .on_press(Event::ClosePane { tab_id, pane })
        .into()
}

fn focus_trap(id: Id) -> Element<'static, Event, Theme, iced::Renderer> {
    text_input("", "")
        .on_input(|_| Event::PaneContextMenuInput)
        .padding(0)
        .size(1)
        .width(Length::Fixed(1.0))
        .id(id)
        .style(|theme: &Theme, _status| {
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
    theme: IcedColorPalette,
) -> impl Fn(&Theme) -> iced::widget::container::Style + 'static {
    move |_theme: &Theme| iced::widget::container::Style {
        background: Some(theme.overlay.into()),
        text_color: Some(theme.foreground),
        border: iced::Border {
            width: 0.25,
            color: theme.overlay,
            radius: Radius::new(4.0),
        },

        ..Default::default()
    }
}

fn menu_button_style(
    theme: IcedColorPalette,
) -> impl Fn(&Theme, ButtonStatus) -> button::Style + 'static {
    move |_theme: &Theme, status: ButtonStatus| {
        let background = match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => {
                Some(theme.dim_blue.into())
            },
            _ => Some(theme.overlay.into()),
        };

        let text_color = match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => theme.dim_black,
            _ => theme.foreground,
        };

        button::Style {
            background,
            text_color,
            border: iced::Border {
                width: 0.0,
                ..Default::default()
            },
            ..Default::default()
        }
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
