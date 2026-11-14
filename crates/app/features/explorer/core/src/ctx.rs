use iced::{Point, Size};

/// What the explorer reads from outside while it reduces: where the pointer
/// is in the window and how big the main area is, so a context menu opens
/// at the pointer and stays inside the main area. The binary builds it from
/// the frame before every intent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplorerCtx {
    /// Last pointer position over the window content.
    pub cursor: Point,
    /// Size of the main area.
    pub screen_size: Size,
}
