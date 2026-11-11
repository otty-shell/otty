use super::state::SidebarState;
use super::types::OpenHeaderMenu;

/// Read-only view model for the side panels around the main area.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarViewModel {
    /// The left panel is shown (open and not hidden by focus mode).
    pub(crate) is_left_open: bool,
    /// The right panel is shown (open and not hidden by focus mode).
    pub(crate) is_right_open: bool,
    /// Focus mode hides both panels.
    pub(crate) is_focus_mode: bool,
    /// How visible the focus mode note is; `None` once it has faded out.
    pub(crate) focus_note_opacity: Option<f32>,
    /// The Sessions section of the left panel is open.
    pub(crate) is_sessions_open: bool,
    /// The Launches section of the left panel is open.
    pub(crate) is_launches_open: bool,
    /// The header menu that is open; `None` while both are closed.
    pub(crate) open_header_menu: Option<OpenHeaderMenu>,
}

impl SidebarViewModel {
    /// Build the view model from the sidebar state.
    pub(super) fn from_state(state: &SidebarState) -> Self {
        Self {
            is_left_open: state.is_left_open(),
            is_right_open: state.is_right_open(),
            is_focus_mode: state.is_focus_mode(),
            focus_note_opacity: state.focus_note_opacity(),
            is_sessions_open: state.is_sessions_open(),
            is_launches_open: state.is_launches_open(),
            open_header_menu: state.header_menu(),
        }
    }
}
