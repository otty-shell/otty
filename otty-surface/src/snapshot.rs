use crate::cell::{Cell, Flags};
use crate::color::Colors;
use crate::damage::{LineDamageBounds, SurfaceDamage};
use crate::escape::CursorShape;
use crate::grid::Dimensions;
use crate::hyperlink::{HyperlinkMap, HyperlinkSpan};
use crate::index::Point;
use crate::mode::SurfaceMode;
use crate::selection::SelectionRange;
use crate::surface::Surface;

/// Terminal cursor rendering information.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct CursorSnapshot {
    pub shape: CursorShape,
    pub cell: Cell,
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

        Self {
            shape,
            point,
            cell: surface
                .grid()
                .cursor
                .template
                .clone()
        }
    }
}

/// Visible terminal content.
///
/// This contains all content required to render the current terminal view.
/// Cell paired with its grid location in an owned snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotCell {
    pub point: Point,
    pub cell: Cell,
}

/// Geometry captured alongside an owned snapshot.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotSize {
    pub columns: usize,
    pub screen_lines: usize,
    pub total_lines: usize,
}

/// Owned view of damage accumulated on the surface.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum SnapshotDamage {
    #[default]
    Full,
    Partial(Vec<LineDamageBounds>),
}

/// Owned snapshot capturing all renderable surface state.
#[derive(Default, Clone)]
pub struct SnapshotOwned {
    cells: Vec<SnapshotCell>,
    selection: Option<SelectionRange>,
    hyperlinks: HyperlinkMap,
    cursor: CursorSnapshot,
    display_offset: usize,
    colors: Colors,
    mode: SurfaceMode,
    size: SnapshotSize,
    damage: SnapshotDamage,
    visible_cell_count: usize,
}

/// View over an owned snapshot suitable for rendering.
pub struct SnapshotView<'a> {
    /// Owned cells with grid positions.
    pub cells: &'a [SnapshotCell],
    /// Resolved selection range in grid coordinates, if any.
    pub selection: Option<&'a SelectionRange>,
    /// Hyperlink mapping for the visible viewport.
    pub(crate) hyperlinks: &'a HyperlinkMap,
    /// Cursor state suitable for rendering.
    pub cursor: &'a CursorSnapshot,
    /// Current scrollback display offset.
    pub display_offset: usize,
    /// Effective color palette.
    pub colors: &'a Colors,
    /// Active surface modes.
    pub mode: SurfaceMode,
    /// Grid geometry at capture time.
    pub size: SnapshotSize,
    /// Damage collected since last reset.
    pub damage: &'a SnapshotDamage,
    /// Total number of cells across visible viewport (cols × rows).
    pub visible_cell_count: usize,
}

impl SnapshotOwned {
    /// Borrow this owned snapshot as a lightweight view.
    pub fn view(&self) -> SnapshotView<'_> {
        SnapshotView {
            cells: &self.cells,
            selection: self.selection.as_ref(),
            hyperlinks: &self.hyperlinks,
            cursor: &self.cursor,
            display_offset: self.display_offset,
            colors: &self.colors,
            mode: self.mode,
            size: self.size,
            damage: &self.damage,
            visible_cell_count: self.visible_cell_count,
        }
    }
}

impl From<&mut Surface> for SnapshotOwned {
    fn from(surface: &mut Surface) -> Self {
        let mut cells =
            Vec::with_capacity(surface.grid().display_iter().count());
        for indexed in surface.grid().display_iter() {
            cells.push(SnapshotCell {
                point: indexed.point,
                cell: indexed.cell.clone(),
            });
        }

        let selection =
            surface.selection.as_ref().and_then(|s| s.to_range(surface));
        let cursor = CursorSnapshot::new(surface);
        let display_offset = surface.grid().display_offset();
        let colors = *surface.colors();
        let mode = *surface.mode();
        let size = SnapshotSize {
            columns: surface.grid().columns(),
            screen_lines: surface.grid().screen_lines(),
            total_lines: surface.grid().total_lines(),
        };
        let visible_cell_count = size.columns * size.screen_lines;
        let hyperlinks = HyperlinkMap::build(&cells, size, display_offset);

        let damage = SnapshotDamage::from(surface.damage());

        Self {
            cells,
            selection,
            hyperlinks,
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

impl From<SurfaceDamage<'_>> for SnapshotDamage {
    fn from(damage: SurfaceDamage<'_>) -> Self {
        match damage {
            SurfaceDamage::Full => Self::Full,
            SurfaceDamage::Partial(iter) => Self::Partial(iter.collect()),
        }
    }
}

/// Apply actions and export owned snapshots.
pub trait SurfaceModel {
    /// Export an owned frame capturing the current surface state.
    fn snapshot_owned(&mut self) -> SnapshotOwned;

    /// Reset any accumulated damage bookkeeping after a frame is consumed.
    fn reset_damage(&mut self) {}
}

impl SurfaceModel for Surface {
    fn snapshot_owned(&mut self) -> SnapshotOwned {
        SnapshotOwned::from(self)
    }

    fn reset_damage(&mut self) {
        Surface::reset_damage(self);
    }
}

impl<'a> SnapshotView<'a> {
    /// Get hyperlink span for the given grid point (visible viewport only).
    #[inline]
    pub fn hyperlink_span_at(&self, point: Point) -> Option<&HyperlinkSpan> {
        self.hyperlinks.span_for_point(self.display_offset, point)
    }

    /// Get hyperlink span id for the given grid point (visible viewport only).
    #[inline]
    pub fn hyperlink_span_id_at(&self, point: Point) -> Option<u32> {
        self.hyperlinks
            .span_id_for_point(self.display_offset, point)
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::SurfaceActor;
    use crate::cell::Hyperlink;
    use crate::index::{Column, Line};
    use crate::selection::SelectionType;
    use crate::{
        SnapshotDamage, SnapshotView, Surface, SurfaceConfig, SurfaceModel,
    };

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

    fn set_text(surface: &mut Surface, line: usize, text: &str) {
        for (idx, ch) in text.chars().enumerate() {
            let column = Column(idx);
            surface.grid_mut()[Line(line as i32)][column].c = ch;
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
        let view = frame.view();
        assert_eq!(view.visible_cell_count, 8);
        assert!(!view.cells.is_empty());
        match view.damage {
            SnapshotDamage::Partial(lines) => {
                assert!(!lines.is_empty());
            },
            SnapshotDamage::Full => {
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
        let view: SnapshotView<'_> = frame.view();

        assert!(view.selection.is_some());
        assert_eq!(view.cursor.point, surface.grid().cursor.point);
    }

    #[test]
    fn osc_hyperlink_is_exposed_in_snapshot() {
        let mut surface =
            Surface::new(SurfaceConfig::default(), &TestDimensions::new(10, 2));
        let link = Hyperlink::new(None::<String>, "https://example.com".into());
        surface.grid_mut()[Line(0)][Column(0)]
            .set_hyperlink(Some(link.clone()));
        surface.grid_mut()[Line(0)][Column(1)]
            .set_hyperlink(Some(link.clone()));
        set_text(&mut surface, 0, "hi");

        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();
        let span = view
            .hyperlink_span_at(Point::new(Line(0), Column(0)))
            .expect("span expected");

        assert_eq!(span.link, link);
        assert!(span.contains(Point::new(Line(0), Column(1))));
    }

    #[test]
    fn regex_detects_plain_url() {
        let mut surface =
            Surface::new(SurfaceConfig::default(), &TestDimensions::new(40, 2));
        set_text(&mut surface, 0, "visit https://otty.sh now");

        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();
        let span = view
            .hyperlink_span_at(Point::new(Line(0), Column(8)))
            .expect("url span");

        assert_eq!(span.link.uri(), "https://otty.sh");
        assert!(
            view.hyperlink_span_at(Point::new(Line(0), Column(22)))
                .is_none()
        );
    }

    #[test]
    fn unsupported_scheme_is_ignored() {
        let mut surface =
            Surface::new(SurfaceConfig::default(), &TestDimensions::new(40, 2));
        set_text(&mut surface, 0, "custom://example is not supported");

        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();

        assert!(
            view.hyperlink_span_at(Point::new(Line(0), Column(0)))
                .is_none()
        );
        assert!(
            view.hyperlink_span_at(Point::new(Line(0), Column(15)))
                .is_none()
        );
    }
}
