use iced::{Point, Size};

/// What Plugins mode reads from outside while it reduces: where the
/// pointer is and how big the main area is, so the menu of a plugin in the
/// left panel opens at the pointer and stays inside the main area. The
/// binary builds it from the frame before every intent.
///
/// # Example
///
/// ```
/// use iced::{Point, Size};
/// use otty_app_plugins::PluginsCtx;
///
/// let ctx = PluginsCtx {
///     cursor: Point::new(120.0, 300.0),
///     screen_size: Size::new(1200.0, 800.0),
/// };
///
/// assert_eq!(ctx.cursor.x, 120.0);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PluginsCtx {
    /// The last pointer position over the window content.
    pub cursor: Point,
    /// The size of the main area.
    pub screen_size: Size,
}
