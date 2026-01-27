use iced::{Point, Size, Task, widget::Id};

/// State for a pane context menu.
#[derive(Debug, Clone)]
pub(crate) struct PaneContextMenuState {
    pub(crate) pane: iced::widget::pane_grid::Pane,
    pub(crate) cursor: Point,
    pub(crate) grid_size: Size,
    pub(crate) terminal_id: u64,
    pub(crate) focus_target: Id,
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

    pub fn focus_task<Message: 'static>(&self) -> Task<Message> {
        iced::widget::operation::focus(self.focus_target.clone())
    }
}
