mod event;
mod model;
mod reducer;
mod side_panel;
mod state;
pub(crate) mod types;

pub(crate) use event::{SidebarEffect, SidebarEvent, SidebarIntent};
use iced::widget::pane_grid;
use iced::{Point, Task};
pub(crate) use model::SidebarViewModel;
use state::SidebarState;

/// Sidebar widget owning the left and right panels around the main area,
/// focus mode and the New session menu.
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
    ) -> Task<SidebarEvent> {
        reducer::reduce(&mut self.state, event)
    }

    /// Build a read-only view model for the presentation layer.
    pub(crate) fn vm(&self) -> SidebarViewModel {
        SidebarViewModel {
            is_left_open: self.state.is_left_open(),
            is_right_open: self.state.is_right_open(),
            is_focus_mode: self.state.is_focus_mode(),
            is_sessions_open: self.state.is_sessions_open(),
            is_launches_open: self.state.is_launches_open(),
            has_new_session_menu_open: self.state.has_new_session_menu_open(),
            new_session_menu_cursor: self.state.new_session_menu_cursor(),
        }
    }

    /// Return read-only access to pane-grid slots for rendering.
    pub(crate) fn panes(&self) -> &pane_grid::State<types::SidebarPane> {
        self.state.panes()
    }

    /// Return the current cursor position snapshot.
    pub(crate) fn cursor(&self) -> Point {
        self.state.cursor()
    }

    /// Return the New session menu anchor when the menu is open.
    pub(crate) fn new_session_menu_cursor(&self) -> Option<Point> {
        self.state.new_session_menu_cursor()
    }

    /// Return whether the New session menu is currently open.
    pub(crate) fn has_new_session_menu_open(&self) -> bool {
        self.state.has_new_session_menu_open()
    }

    /// Return whether the user is actively resizing a side panel.
    pub(crate) fn is_resizing(&self) -> bool {
        self.state.is_resizing()
    }

    /// Share of the window width both shown side panels take together; the
    /// main area gets the rest.
    pub(crate) fn side_panels_ratio(&self) -> f32 {
        self.state.left_ratio() + self.state.right_ratio()
    }
}

#[cfg(test)]
impl SidebarWidget {
    /// Share of the window width the left panel takes now.
    pub(crate) fn left_ratio(&self) -> f32 {
        self.state.left_ratio()
    }

    /// Share of the window width the right panel takes now.
    pub(crate) fn right_ratio(&self) -> f32 {
        self.state.right_ratio()
    }
}
