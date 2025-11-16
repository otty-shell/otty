use crate::{TerminalSize, surface::SurfaceSnapshot};

/// Immutable snapshot of the terminal surface and its geometry.
pub struct TerminalSnapshot<'a> {
    /// Captured view of the underlying surface at a point in time.
    pub surface: SurfaceSnapshot<'a>,
    /// Terminal grid size associated with this snapshot.
    pub size: TerminalSize,
}

impl<'a> TerminalSnapshot<'a> {
    /// Construct a new snapshot from a surface view and terminal size.
    pub fn new(surface: SurfaceSnapshot<'a>, size: TerminalSize) -> Self {
        Self { surface, size }
    }
}
