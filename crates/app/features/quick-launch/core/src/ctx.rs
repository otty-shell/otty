use iced::{Point, Size};

/// What quick launch reads from outside while it reduces: where the pointer
/// is, whether a panel edge is being dragged and how big the main area is.
/// The binary builds it from the frame before every intent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuickLaunchCtx {
    /// Last pointer position over the window content; a context menu opens
    /// there.
    pub cursor: Point,
    /// A panel edge was dragged a moment ago, so the tree keeps no hover
    /// until the drag settles.
    pub is_panel_resizing: bool,
    /// Size of the main area; a context menu stays inside it.
    pub screen_size: Size,
}
