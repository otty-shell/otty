mod event;
mod model;
mod reducer;
mod state;
pub(crate) mod view;

pub(crate) use event::{SidebarEffect, SidebarEvent, SidebarIntent};
use iced::widget::pane_grid;
use iced::{Point, Task};
pub(crate) use model::{
    SIDEBAR_MENU_WIDTH, SidebarItem, SidebarPane, SidebarViewModel,
};
pub(crate) use reducer::SidebarCtx;
use state::SidebarState;

/// Sidebar widget owning its private state.
pub(crate) struct SidebarWidget {
    state: SidebarState,
}

impl SidebarWidget {
    /// Construct sidebar with default layout state.
    pub(crate) fn new() -> Self {
        Self {
            state: SidebarState::default(),
        }
    }

    /// Reduce an intent event into state updates and effect events.
    pub(crate) fn reduce(
        &mut self,
        event: SidebarIntent,
        ctx: &SidebarCtx,
    ) -> Task<SidebarEvent> {
        reducer::reduce(&mut self.state, event, ctx)
    }

    /// Build a read-only view model for the presentation layer.
    pub(crate) fn vm(&self) -> SidebarViewModel {
        SidebarViewModel {
            active_item: self.state.active_item(),
            is_hidden: self.state.is_hidden(),
            is_workspace_open: self.state.is_workspace_open(),
            has_add_menu_open: self.state.has_add_menu_open(),
            add_menu_cursor: self.state.add_menu_cursor(),
        }
    }

    /// Return whether the sidebar rail and workspace are hidden.
    pub(crate) fn is_hidden(&self) -> bool {
        self.state.is_hidden()
    }

    /// Return read-only access to pane-grid slots for rendering.
    pub(crate) fn panes(&self) -> &pane_grid::State<SidebarPane> {
        self.state.panes()
    }

    /// Return the current cursor position snapshot.
    pub(crate) fn cursor(&self) -> Point {
        self.state.cursor()
    }

    /// Return add-menu cursor anchor when the menu is open.
    pub(crate) fn add_menu_cursor(&self) -> Option<Point> {
        self.state.add_menu_cursor()
    }

    /// Return whether the add-menu overlay is currently open.
    pub(crate) fn has_add_menu_open(&self) -> bool {
        self.state.has_add_menu_open()
    }

    /// Return whether the user is actively resizing the sidebar split.
    pub(crate) fn is_resizing(&self) -> bool {
        self.state.is_resizing()
    }

    /// Return effective workspace ratio used for content sizing.
    pub(crate) fn effective_workspace_ratio(&self) -> f32 {
        self.state.effective_workspace_ratio()
    }
}
