use crate::cell::{Cell, Flags};
use crate::color::Colors;
use crate::damage::{LineDamageBounds, SurfaceDamage};
use crate::escape::CursorShape;
use crate::grid::{Dimensions, GridIterator};
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

/// Cell paired with its grid location in an owned frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCell {
    pub point: Point,
    pub cell: Cell,
}

/// Geometry captured alongside an owned frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSize {
    pub columns: usize,
    pub screen_lines: usize,
    pub total_lines: usize,
}

/// Owned view of damage accumulated on the surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameDamage {
    Full,
    Partial(Vec<LineDamageBounds>),
}

/// Owned frame capturing all renderable surface state.
#[derive(Clone)]
pub struct FrameOwned {
    cells: Vec<FrameCell>,
    selection: Option<SelectionRange>,
    cursor: CursorSnapshot,
    display_offset: usize,
    colors: Colors,
    mode: SurfaceMode,
    size: FrameSize,
    damage: FrameDamage,
    visible_cell_count: usize,
}

/// View over an owned frame suitable for rendering.
pub struct FrameView<'a> {
    /// Owned cells with grid positions.
    pub cells: &'a [FrameCell],
    /// Resolved selection range in grid coordinates, if any.
    pub selection: Option<&'a SelectionRange>,
    /// Cursor state suitable for rendering.
    pub cursor: CursorSnapshot,
    /// Current scrollback display offset.
    pub display_offset: usize,
    /// Effective color palette.
    pub colors: &'a Colors,
    /// Active surface modes.
    pub mode: SurfaceMode,
    /// Grid geometry at capture time.
    pub size: FrameSize,
    /// Damage collected since last reset.
    pub damage: &'a FrameDamage,
    /// Total number of cells across visible viewport (cols × rows).
    pub visible_cell_count: usize,
}

impl FrameOwned {
    /// Borrow this owned frame as a lightweight view.
    pub fn view(&self) -> FrameView<'_> {
        FrameView {
            cells: &self.cells,
            selection: self.selection.as_ref(),
            cursor: self.cursor,
            display_offset: self.display_offset,
            colors: &self.colors,
            mode: self.mode,
            size: self.size,
            damage: &self.damage,
            visible_cell_count: self.visible_cell_count,
        }
    }
}

impl From<&mut Surface> for FrameOwned {
    fn from(surface: &mut Surface) -> Self {
        let snapshot = surface.snapshot();
        let mut cells =
            Vec::with_capacity(snapshot.display_iter.clone().count());
        for indexed in snapshot.display_iter {
            cells.push(FrameCell {
                point: indexed.point,
                cell: indexed.cell.clone(),
            });
        }

        let selection = snapshot.selection;
        let cursor = snapshot.cursor;
        let display_offset = snapshot.display_offset;
        let colors = *snapshot.colors;
        let mode = snapshot.mode;
        let size = FrameSize {
            columns: surface.grid().columns(),
            screen_lines: surface.grid().screen_lines(),
            total_lines: surface.grid().total_lines(),
        };

        let damage = FrameDamage::from(surface.damage());
        let visible_cell_count = size.columns * size.screen_lines;

        Self {
            cells,
            selection,
            cursor,
            display_offset,
            colors,
            mode,
            size,
            damage,
            visible_cell_count,
        }
    }
}

impl From<SurfaceDamage<'_>> for FrameDamage {
    fn from(damage: SurfaceDamage<'_>) -> Self {
        match damage {
            SurfaceDamage::Full => Self::Full,
            SurfaceDamage::Partial(iter) => Self::Partial(iter.collect()),
        }
    }
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

/// Apply actions and export owned snapshots.
pub trait SurfaceModel {
    /// Export an owned frame capturing the current surface state.
    fn snapshot_owned(&mut self) -> FrameOwned;

    /// Reset any accumulated damage bookkeeping after a frame is consumed.
    fn reset_damage(&mut self) {}
}

impl SurfaceModel for Surface {
    fn snapshot_owned(&mut self) -> FrameOwned {
        FrameOwned::from(self)
    }

    fn reset_damage(&mut self) {
        Surface::reset_damage(self);
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::SurfaceActor;
    use crate::selection::SelectionType;
    use crate::{FrameDamage, FrameView, Surface, SurfaceConfig, SurfaceModel};

    use super::*;

    struct TestDimensions {
        columns: usize,
        lines: usize,
    }

    impl Dimensions for TestDimensions {
        fn total_lines(&self) -> usize {
            self.lines
        }

        fn screen_lines(&self) -> usize {
            self.lines
        }

        fn columns(&self) -> usize {
            self.columns
        }
    }

    impl TestDimensions {
        fn new(columns: usize, lines: usize) -> Self {
            Self { columns, lines }
        }
    }

    #[test]
    fn captures_owned_frame_with_damage() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = Surface::new(SurfaceConfig::default(), &dims);

        // Reset initial full damage, then mutate.
        surface.reset_damage();
        surface.print('X');
        let frame = surface.snapshot_owned();

        assert_eq!(frame.view().size.columns, 4);
        assert_eq!(frame.view().size.screen_lines, 2);
        assert_eq!(frame.view().visible_cell_count, 8);
        assert!(!frame.view().cells.is_empty());
        match frame.view().damage {
            FrameDamage::Partial(lines) => {
                assert!(!lines.is_empty());
            },
            FrameDamage::Full => {
                panic!("expected partial damage after single print")
            },
        }
    }

    #[test]
    fn view_exposes_selection_and_cursor() {
        let dims = TestDimensions::new(3, 2);
        let mut surface = Surface::new(SurfaceConfig::default(), &dims);
        surface.reset_damage();

        let start = Point::new(crate::index::Line(0), crate::index::Column(0));
        surface.start_selection(
            SelectionType::Simple,
            start,
            crate::index::Side::Left,
        );
        surface.update_selection(
            Point::new(crate::index::Line(0), crate::index::Column(1)),
            crate::index::Side::Right,
        );

        let frame = surface.snapshot_owned();
        let view: FrameView<'_> = frame.view();

        assert!(view.selection.is_some());
        assert_eq!(view.cursor.point, surface.grid().cursor.point);
    }
}
