use crate::cell::{Cell, Flags};
use crate::color::Colors;
use crate::escape::CursorShape;
use crate::grid::GridIterator;
use crate::index::Point;
use crate::mode::SurfaceMode;
use crate::selection::SelectionRange;
use crate::surface::Surface;

/// Terminal cursor rendering information.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct CursorSnapshot {
    pub shape: CursorShape,
    pub point: Point,
}

impl CursorSnapshot {
    /// Construct a renderable cursor description from the given surface.
    ///
    /// This accounts for wide characters and cursor visibility modes.
    fn new(surface: &Surface) -> Self {
        // Cursor position.
        let mut point = surface.grid().cursor.point;
        if surface.grid()[point]
            .flags
            .contains(Flags::WIDE_CHAR_SPACER)
        {
            point.column -= 1;
        }

        // Cursor shape.
        let shape = if !surface.mode().contains(SurfaceMode::SHOW_CURSOR) {
            CursorShape::Hidden
        } else {
            surface.cursor_style().shape
        };

        Self { shape, point }
    }
}

/// Visible terminal content.
///
/// This contains all content required to render the current terminal view.
pub struct SurfaceSnapshot<'a> {
    /// Iterator over all cells in the visible region.
    pub display_iter: GridIterator<'a, Cell>,
    /// Resolved selection range in grid coordinates, if any.
    pub selection: Option<SelectionRange>,
    /// Cursor state suitable for rendering.
    pub cursor: CursorSnapshot,
    /// Current scrollback display offset.
    pub display_offset: usize,
    /// Effective color palette.
    pub colors: &'a Colors,
    /// Active surface modes.
    pub mode: SurfaceMode,
}

impl<'a> SurfaceSnapshot<'a> {
    /// Capture a snapshot of all state needed to render the given surface.
    pub(crate) fn new(surface: &'a Surface) -> Self {
        Self {
            display_iter: surface.grid().display_iter(),
            display_offset: surface.grid().display_offset(),
            cursor: CursorSnapshot::new(surface),
            selection: surface
                .selection
                .as_ref()
                .and_then(|s| s.to_range(surface)),
            colors: surface.colors(),
            mode: *surface.mode(),
        }
    }
}

pub trait SurfaceSnapshotSource {
    /// Capture a read‑only snapshot of the surface suitable for rendering.
    fn capture_snapshot(&self) -> SurfaceSnapshot<'_>;
}

impl SurfaceSnapshotSource for Surface {
    /// Capture a [`SurfaceSnapshot`] from this surface.
    fn capture_snapshot(&self) -> SurfaceSnapshot<'_> {
        self.snapshot()
    }
}
