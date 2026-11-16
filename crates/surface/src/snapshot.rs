use crate::block::{BlockKey, BlockKind, BlockSnapshot};
use crate::cell::{Cell, Flags};
use crate::color::Colors;
use crate::damage::{LineDamageBounds, SurfaceDamage};
use crate::escape::CursorShape;
use crate::grid::Dimensions;
use crate::hyperlink::{HyperlinkMap, HyperlinkSpan};
use crate::index::{Line, Point};
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
    pub(crate) fn new(surface: &Surface) -> Self {
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
            cell: surface.grid().cursor.template.clone(),
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

impl SnapshotSize {
    /// How many lines the view can scroll up from the bottom of the
    /// history: the lines of the history that do not fit on screen. None
    /// for a history that fits.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_surface::SnapshotSize;
    ///
    /// let size = SnapshotSize {
    ///     columns: 80,
    ///     screen_lines: 24,
    ///     total_lines: 100,
    /// };
    ///
    /// assert_eq!(size.max_display_offset(), 76);
    /// ```
    pub fn max_display_offset(&self) -> usize {
        self.total_lines.saturating_sub(self.screen_lines)
    }
}

/// Owned view of damage accumulated on the surface.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum SnapshotDamage {
    #[default]
    Full,
    Partial(Vec<LineDamageBounds>),
}

impl From<SurfaceDamage<'_>> for SnapshotDamage {
    fn from(damage: SurfaceDamage<'_>) -> Self {
        match damage {
            SurfaceDamage::Full => Self::Full,
            SurfaceDamage::Partial(iter) => Self::Partial(iter.collect()),
        }
    }
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
    pub blocks: Vec<BlockSnapshot>,
    /// The lines on screen that OTTY adds to the blocks and that hold no
    /// terminal text: the status lines.
    added_lines: Vec<Line>,
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
            blocks: &self.blocks,
            added_lines: &self.added_lines,
        }
    }

    /// Report the whole surface as changed, whatever damage was captured.
    ///
    /// Use this when a consumer may have missed an earlier snapshot: the
    /// lines that only the missed snapshot reported would otherwise never
    /// be redrawn.
    ///
    /// ```
    /// use otty_surface::{SnapshotDamage, SnapshotOwned};
    ///
    /// let mut snapshot = SnapshotOwned::default();
    /// snapshot.mark_fully_damaged();
    ///
    /// assert_eq!(snapshot.view().damage, &SnapshotDamage::Full);
    /// ```
    pub fn mark_fully_damaged(&mut self) {
        self.damage = SnapshotDamage::Full;
    }

    pub(crate) fn from_parts(
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
        blocks: Vec<BlockSnapshot>,
        added_lines: Vec<Line>,
    ) -> Self {
        SnapshotOwned {
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
            blocks,
            added_lines,
        }
    }

    pub fn from_surface(surface: &mut Surface) -> SnapshotOwned {
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

        SnapshotOwned {
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
            blocks: Vec::new(),
            added_lines: Vec::new(),
        }
    }
}

impl From<&mut Surface> for SnapshotOwned {
    fn from(surface: &mut Surface) -> Self {
        SnapshotOwned::from_surface(surface)
    }
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
    /// Ordered list of block metadata captured alongside this snapshot.
    pub(crate) blocks: &'a [BlockSnapshot],
    /// The lines on screen OTTY adds to the blocks, which hold no terminal
    /// text.
    pub(crate) added_lines: &'a [Line],
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

    /// Get selected content into one string buffer. The status lines of
    /// the blocks are not text, so a selection across one leaves it out.
    #[inline]
    pub fn selectable_content(&self) -> String {
        let mut result = String::new();
        if let Some(range) = self.selection {
            for indexed in self.cells {
                if self.added_lines.contains(&indexed.point.line) {
                    continue;
                }

                if range.contains(indexed.point) {
                    result.push(indexed.cell.c);
                    if let Some(zerowidth) = indexed.cell.zerowidth() {
                        result.extend(zerowidth.iter());
                    }
                }
            }
        }
        result
    }

    /// Whether the cell at `point` is drawn selected: it is in the
    /// selection and on a line of terminal text. The lines OTTY adds to the
    /// blocks, the status lines, are never drawn selected, though a
    /// selection across blocks runs over them.
    pub fn is_selected(&self, point: Point) -> bool {
        self.selection.is_some_and(|range| range.contains(point))
            && !self.added_lines.contains(&point.line)
    }

    /// Return the list of block metadata captured in this snapshot.
    #[inline]
    pub fn blocks(&self) -> &[BlockSnapshot] {
        self.blocks
    }

    /// Return the block with `key`, if it is in this snapshot. A key never
    /// repeats in a terminal, so there is at most one.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_surface::{BlockKey, BlockSnapshot, SnapshotOwned};
    ///
    /// let mut snapshot = SnapshotOwned::default();
    /// snapshot.blocks.push(BlockSnapshot {
    ///     key: BlockKey::new(7),
    ///     ..BlockSnapshot::default()
    /// });
    ///
    /// assert!(snapshot.view().block(BlockKey::new(7)).is_some());
    /// assert!(snapshot.view().block(BlockKey::new(8)).is_none());
    /// ```
    pub fn block(&self, key: BlockKey) -> Option<&'a BlockSnapshot> {
        self.blocks.iter().find(|block| block.key == key)
    }

    /// Whether a full-screen program, such as `vim` or `top`, is on now: it
    /// draws on a screen of its own (the alternate screen), where there is
    /// no history and no block to show.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_surface::{SnapshotOwned, SurfaceMode};
    ///
    /// let snapshot = SnapshotOwned::default();
    /// let mut view = snapshot.view();
    /// assert!(!view.is_full_screen());
    ///
    /// view.mode = SurfaceMode::ALT_SCREEN;
    /// assert!(view.is_full_screen());
    /// ```
    pub fn is_full_screen(&self) -> bool {
        self.mode.contains(SurfaceMode::ALT_SCREEN)
    }

    /// Return the command `Run again` types for the block `key`: the
    /// command of a command block, while the shell waits at the prompt (its
    /// latest block is the prompt). While a command runs, the typed
    /// command would go to that program, so there is none then, and none
    /// for a block without a command.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_surface::{BlockKey, SnapshotOwned};
    ///
    /// // No block, so no prompt to type at.
    /// let snapshot = SnapshotOwned::default();
    ///
    /// assert_eq!(snapshot.view().run_again_command(BlockKey::new(1)), None);
    /// ```
    pub fn run_again_command(&self, key: BlockKey) -> Option<&'a str> {
        let is_at_prompt = self
            .blocks
            .last()
            .is_some_and(|block| block.meta.kind == BlockKind::Prompt);
        if !is_at_prompt {
            return None;
        }

        self.block(key)
            .filter(|block| block.is_command())
            .and_then(|block| block.meta.cmd.as_deref())
            .filter(|command| !command.trim().is_empty())
    }

    /// Resolve the block intersecting the provided grid point, if any.
    pub fn block_at_point(&self, point: Point) -> Option<&BlockSnapshot> {
        self.blocks.iter().find(|block| {
            if block.line_count == 0 {
                return false;
            }

            let start = block.start_line;
            let end = start + block.line_count as i32;
            point.line.0 >= start && point.line.0 < end
        })
    }
}

/// Apply actions and export owned snapshots.
pub trait SurfaceModel {
    /// Export an owned snapshot capturing the current surface state.
    fn snapshot_owned(&mut self) -> SnapshotOwned;

    /// Reset any accumulated damage bookkeeping after a snapshot is consumed.
    fn reset_damage(&mut self) {}
}

impl SurfaceModel for Surface {
    fn snapshot_owned(&mut self) -> SnapshotOwned {
        SnapshotOwned::from_surface(self)
    }

    fn reset_damage(&mut self) {
        Surface::reset_damage(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::SurfaceActor;
    use crate::block::testing::block_of_kind;
    use crate::cell::Hyperlink;
    use crate::index::{Column, Line, Side};
    use crate::selection::SelectionType;
    use crate::{
        SnapshotDamage, SnapshotView, Surface, SurfaceConfig, SurfaceModel,
    };

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

    /// A block of `kind` with the key `key` that ran `cmd`.
    fn block(key: u64, kind: BlockKind, cmd: Option<&str>) -> BlockSnapshot {
        let mut block = block_of_kind(key, kind);
        block.meta.cmd = cmd.map(String::from);

        block
    }

    fn snapshot_of(blocks: Vec<BlockSnapshot>) -> SnapshotOwned {
        SnapshotOwned {
            blocks,
            ..SnapshotOwned::default()
        }
    }

    #[test]
    fn given_command_block_while_the_prompt_waits_when_run_again_is_asked_then_its_command_is_typed()
     {
        let snapshot = snapshot_of(vec![
            block(0, BlockKind::Start, None),
            block(1, BlockKind::Command, Some("ls -la")),
            block(2, BlockKind::Prompt, None),
        ]);

        let view = snapshot.view();

        assert_eq!(view.run_again_command(BlockKey::new(1)), Some("ls -la"));
        assert_eq!(view.run_again_command(BlockKey::new(0)), None);
        assert_eq!(view.run_again_command(BlockKey::new(2)), None);
        assert_eq!(view.run_again_command(BlockKey::new(9)), None);
    }

    #[test]
    fn given_command_running_when_run_again_is_asked_then_there_is_none() {
        let snapshot = snapshot_of(vec![
            block(1, BlockKind::Command, Some("ls")),
            block(2, BlockKind::Command, Some("sleep 30")),
        ]);

        assert_eq!(snapshot.view().run_again_command(BlockKey::new(1)), None);
    }

    #[test]
    fn given_block_with_a_blank_command_when_run_again_is_asked_then_there_is_none()
     {
        let snapshot = snapshot_of(vec![
            block(1, BlockKind::Command, Some("  ")),
            block(2, BlockKind::Prompt, None),
        ]);

        assert_eq!(snapshot.view().run_again_command(BlockKey::new(1)), None);
    }

    #[test]
    fn only_a_block_a_command_started_is_a_command_block() {
        let kinds = [
            (BlockKind::Command, true),
            (BlockKind::Prompt, false),
            (BlockKind::Start, false),
            (BlockKind::FullScreen, false),
        ];

        for (kind, is_command) in kinds {
            assert_eq!(block(1, kind, None).is_command(), is_command);
        }
    }

    #[test]
    fn only_a_finished_command_with_an_exit_code_other_than_0_failed() {
        let cases = [
            (true, Some(101), true),
            (true, Some(0), false),
            (true, None, false),
            (false, Some(1), false),
        ];

        for (is_finished, exit_code, is_failed) in cases {
            let mut block = block(1, BlockKind::Command, None);
            block.meta.is_finished = is_finished;
            block.meta.exit_code = exit_code;

            assert_eq!(block.is_failed(), is_failed);
        }
    }

    #[test]
    fn captures_owned_snapshot_with_damage() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = Surface::new(SurfaceConfig::default(), &dims);

        // Reset initial full damage, then mutate.
        surface.reset_damage();
        surface.print('X');
        let snapshot = surface.snapshot_owned();

        assert_eq!(snapshot.view().size.columns, 4);
        assert_eq!(snapshot.view().size.screen_lines, 2);
        let view = snapshot.view();
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
    fn mark_fully_damaged_replaces_partial_damage() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = Surface::new(SurfaceConfig::default(), &dims);
        surface.reset_damage();
        surface.print('X');
        let mut snapshot = surface.snapshot_owned();
        assert!(matches!(snapshot.view().damage, SnapshotDamage::Partial(_)));

        snapshot.mark_fully_damaged();

        assert_eq!(snapshot.view().damage, &SnapshotDamage::Full);
    }

    #[test]
    fn view_exposes_selection_and_cursor() {
        let dims = TestDimensions::new(3, 2);
        let mut surface = Surface::new(SurfaceConfig::default(), &dims);
        surface.reset_damage();

        let start = Point::new(crate::index::Line(0), crate::index::Column(0));
        surface.start_selection(SelectionType::Simple, start, Side::Left);
        surface.update_selection(
            Point::new(crate::index::Line(0), crate::index::Column(1)),
            Side::Right,
        );

        let snapshot = surface.snapshot_owned();
        let view: SnapshotView<'_> = snapshot.view();

        assert!(view.selection.is_some());
        assert_eq!(view.cursor.point, surface.grid().cursor.point);
    }

    #[test]
    fn selectable_content_preserves_zero_width_marks() {
        let mut surface =
            Surface::new(SurfaceConfig::default(), &TestDimensions::new(40, 2));
        let text = "ที่นี่, น้ำ, or กำลัง";
        for ch in text.chars() {
            surface.print(ch);
        }

        let has_zero_width_marks = surface.grid().display_iter().any(|cell| {
            cell.cell.zerowidth().is_some_and(|marks| !marks.is_empty())
        });
        assert!(has_zero_width_marks);

        surface.start_selection(
            SelectionType::Simple,
            Point::new(Line(0), Column(0)),
            Side::Left,
        );
        let end_column = surface.grid().cursor.point.column - 1;
        surface.update_selection(Point::new(Line(0), end_column), Side::Right);

        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();

        assert_eq!(view.selectable_content(), text);
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
        set_text(&mut surface, 0, "visit https://otty.run now");

        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();
        let span = view
            .hyperlink_span_at(Point::new(Line(0), Column(8)))
            .expect("url span");

        assert_eq!(span.link.uri(), "https://otty.run");
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
