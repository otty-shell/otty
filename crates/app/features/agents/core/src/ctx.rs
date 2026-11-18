use std::path::PathBuf;

use iced::{Point, Size};

/// What the agents feature reads from outside while it reduces: where the
/// pointer is and how big the main area is, so a menu opens at the pointer
/// and stays inside the main area; whether Terminal mode has a pane to
/// continue a session in a split; and the folder of that pane, where the
/// New session dialog opens. The binary builds it from the frame and the
/// terminal before every intent.
///
/// # Example
///
/// ```
/// use iced::{Point, Size};
/// use otty_app_agents::AgentsCtx;
///
/// let ctx = AgentsCtx {
///     cursor: Point::new(120.0, 300.0),
///     screen_size: Size::new(1200.0, 800.0),
///     has_local_pane: false,
///     local_folder: None,
/// };
///
/// assert_eq!(ctx.cursor.x, 120.0);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentsCtx {
    /// The last pointer position, measured from the top-left corner of the
    /// main area.
    pub cursor: Point,
    /// The size of the main area.
    pub screen_size: Size,
    /// Whether the tab in front of Terminal mode is a terminal tab on this
    /// computer with a focused pane: Continue in a split splits that pane.
    pub has_local_pane: bool,
    /// The folder of the focused pane of the tab in front, when that tab
    /// runs on this computer and the folder is known: the folder Files
    /// follows.
    pub local_folder: Option<PathBuf>,
}
