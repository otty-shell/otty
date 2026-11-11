use iced::{Point, Size};

/// What the terminal reads from outside while it reduces. The binary builds
/// it before every intent from the window, the sidebar, the saved settings,
/// the modes and the overlays, so a decision reads them as they are at that
/// moment.
///
/// A plain input bag that crosses the crate boundary, so its fields are
/// public.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalCtx {
    /// Room the pane grid has: the main area without the shown side panels.
    pub pane_grid_size: Size,
    /// Size of the main area; a pane menu is kept inside it.
    pub screen_size: Size,
    /// Where the pointer is in the main area; a pane menu opens there.
    pub cursor: Point,
    /// Whether sibling panes are evened out on split and close: the saved
    /// setting, not an edit in the open Settings dialog.
    pub equalize_panes: bool,
    /// Whether Terminal mode is on screen. Outside it no pane is shown, so
    /// none takes the keyboard when an overlay closes.
    pub is_terminal_mode: bool,
    /// Whether a menu or a dialog lies over the window. It keeps the
    /// keyboard, so no terminal takes the focus from it.
    pub is_overlay_open: bool,
    /// Whether an overlay other than a terminal pane menu lies over the
    /// window, such as the SSH prompt over a pane menu. A pane that closes
    /// closes its own menu, but not this overlay.
    pub is_other_overlay_open: bool,
}
