use iced::{Point, Size};

/// What the agents feature reads from outside while it reduces: where the
/// pointer is and how big the main area is, so a menu of the left panel
/// opens at the pointer and stays inside the main area. The binary builds
/// it from the frame before every intent.
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
/// };
///
/// assert_eq!(ctx.cursor.x, 120.0);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AgentsCtx {
    /// The last pointer position, measured from the top-left corner of the
    /// main area.
    pub cursor: Point,
    /// The size of the main area.
    pub screen_size: Size,
}
