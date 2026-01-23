use iced::border::Radius;
use iced::widget::text_input;
use iced::widget::{Column, Id, container, mouse_area};
use iced::{Background, Element, Length, Point, Size, Task, alignment};

use crate::app::theme::ThemeProps;
use crate::components::menu_item::{MenuItem, MenuItemEvent, MenuItemProps};
use crate::widgets::pane_context_menu::PaneContextMenuEvent::*;

const MENU_CONTAINER_WIDTH: f32 = 250.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_VERTICAL_PADDING: f32 = 20.0;
const MENU_MARGIN: f32 = 6.0;

/// UI events emitted by the pane context menu.
#[derive(Debug, Clone)]
pub(crate) enum PaneContextMenuEvent {
    SplitPane(iced::widget::pane_grid::Axis),
    ClosePane,
    CopySelection,
    PasteIntoPrompt,
    CopyBlockContent,
    CopyBlockPrompt,
    CopyBlockCommand,
    Dismiss,
    FocusInput,
}

/// State for a pane context menu.
#[derive(Debug, Clone)]
pub(crate) struct PaneContextMenuState {
    pub(crate) pane: iced::widget::pane_grid::Pane,
    cursor: Point,
    grid_size: Size,
    pub(crate) terminal_id: u64,
    focus_target: Id,
}

impl PaneContextMenuState {
    pub fn new(
        pane: iced::widget::pane_grid::Pane,
        cursor: Point,
        grid_size: Size,
        terminal_id: u64,
    ) -> Self {
        Self {
            pane,
            cursor,
            grid_size,
            terminal_id,
            focus_target: Id::unique(),
        }
    }

    pub fn anchor_for_height(&self, menu_height: f32) -> Point {
        anchor_position(self.cursor, self.grid_size, menu_height)
    }

    pub fn focus_task<Message: 'static>(&self) -> Task<Message> {
        iced::widget::operation::focus(self.focus_target.clone())
    }
}

/// Props for rendering a pane context menu.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaneContextMenuProps<'a> {
    pub(crate) menu: &'a PaneContextMenuState,
    pub(crate) has_block_selection: bool,
    pub(crate) theme: ThemeProps<'a>,
}

/// The overlay menu rendered on top of a terminal pane.
pub(crate) struct PaneContextMenu<'a> {
    props: PaneContextMenuProps<'a>,
}

impl<'a> PaneContextMenu<'a> {
    pub fn new(props: PaneContextMenuProps<'a>) -> Self {
        Self { props }
    }

    pub fn view(&self) -> Element<'a, PaneContextMenuEvent> {
        let mut buttons: Vec<Element<'a, PaneContextMenuEvent>> = Vec::new();

        buttons.push(menu_item(
            "Copy selection",
            self.props.theme,
            CopySelection,
        ));
        buttons.push(menu_item("Paste", self.props.theme, PasteIntoPrompt));

        if self.props.has_block_selection {
            buttons.push(menu_item(
                "Copy content",
                self.props.theme,
                CopyBlockContent,
            ));
            buttons.push(menu_item(
                "Copy prompt",
                self.props.theme,
                CopyBlockPrompt,
            ));
            buttons.push(menu_item(
                "Copy command",
                self.props.theme,
                CopyBlockCommand,
            ));
        }

        buttons.push(menu_item(
            "Split horizontally",
            self.props.theme,
            SplitPane(iced::widget::pane_grid::Axis::Horizontal),
        ));
        buttons.push(menu_item(
            "Split vertically",
            self.props.theme,
            SplitPane(iced::widget::pane_grid::Axis::Vertical),
        ));
        buttons.push(menu_item("Close", self.props.theme, ClosePane));

        let menu_height = menu_height_for_items(buttons.len());
        let menu_column = buttons
            .into_iter()
            .fold(Column::new(), |column, button| column.push(button));
        let menu_column = menu_column
            .spacing(0)
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Left);

        let anchor = self.props.menu.anchor_for_height(menu_height);

        let padding = iced::Padding {
            top: anchor.y,
            left: anchor.x,
            ..iced::Padding::ZERO
        };

        let menu_container = container(menu_column)
            .padding([10, 0])
            .width(MENU_CONTAINER_WIDTH)
            .style(menu_panel_style(self.props.theme));

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
        .on_press(Dismiss)
        .on_right_press(Dismiss);

        iced::widget::stack!(
            dismiss_layer,
            positioned_menu,
            focus_trap(self.props.menu.focus_target.clone())
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn menu_item<'a>(
    label: &'a str,
    theme: ThemeProps<'a>,
    event: PaneContextMenuEvent,
) -> Element<'a, PaneContextMenuEvent> {
    let props = MenuItemProps { label, theme };
    MenuItem::new(props)
        .view()
        .map(move |item_event| match item_event {
            MenuItemEvent::Pressed => event.clone(),
        })
}

fn focus_trap(id: Id) -> Element<'static, PaneContextMenuEvent> {
    text_input("", "")
        .on_input(|_| FocusInput)
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
