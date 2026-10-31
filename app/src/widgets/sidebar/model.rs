use iced::Point;

/// Read-only view model for the side panels around the main area.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarViewModel {
    /// The left panel is shown (open and not hidden by focus mode).
    pub(crate) is_left_open: bool,
    /// The right panel is shown (open and not hidden by focus mode).
    pub(crate) is_right_open: bool,
    /// Focus mode hides both panels.
    pub(crate) is_focus_mode: bool,
    /// The Sessions section of the left panel is open.
    pub(crate) is_sessions_open: bool,
    /// The Launches section of the left panel is open.
    pub(crate) is_launches_open: bool,
    pub(crate) has_add_menu_open: bool,
    pub(crate) add_menu_cursor: Option<Point>,
}
