//! Where the rows of a terminal pane are drawn: on the line grid, with
//! space around the lines of every block.

use std::ops::Range;

use otty_libterm::surface::{BlockSnapshot, SnapshotView};

/// Space between the lines of a block and its edge, above its first line
/// and below its last, in pixels.
pub(crate) const BLOCK_GAP: f32 = 6.0;
/// Height of the line that separates a block from the next one, in pixels.
pub(crate) const BLOCK_LINE_HEIGHT: f32 = 1.0;
/// Space under the last line of a block: the gap, and the line that
/// separates the block from the next one at its bottom.
pub(crate) const BLOCK_GAP_BELOW: f32 = BLOCK_GAP + BLOCK_LINE_HEIGHT;

/// Where every row of a pane is drawn, in pixels from the top of the text
/// area, which is the top edge of the pane.
///
/// On the line grid row `n` starts `n` line heights down. Blocks add space
/// to it: [`BLOCK_GAP`] above the first row of every block and below its
/// last one, and the line between two blocks under that. The bottom row
/// stays where the grid puts it, so the space pushes the rows above it up,
/// and the rows it pushes over the top edge are cut. The program still has
/// as many rows as fit by the line height. A full-screen program has no
/// blocks, so its rows stay on the line grid.
///
/// At the top of the history (the model scrolled up as far as it goes, or
/// a history that fits in the pane by its lines and can not scroll) the
/// gaps may still cut the first block. The view can then go one step
/// further up, past the top: the user asks for it, and the engine of the
/// widget keeps it. Past the top the rows are laid out from the top edge
/// down instead: the first block starts at the top edge with its whole
/// gap, empty rows above it go over the top edge, and the space pushes the
/// rows below it down and cuts the ones it pushes over the bottom of the
/// line grid. Without that step the bottom row keeps to the bottom edge,
/// at the top of the history too, so the last step to the top moves the
/// text by one line like every other step.
///
/// Only the rows on screen are laid out. A line above them or below them
/// (and every line of a snapshot that has no rows) goes on in steps of
/// the line height.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RowLayout {
    cell_height: f32,
    display_offset: i32,
    /// Whether the rows are laid out from the top edge: the view is one
    /// step past the top of the history.
    is_past_the_top: bool,
    /// The top of every row on screen.
    row_tops: Vec<f32>,
    /// Whether a block starts at the row and the gap above it is on screen:
    /// not at the top row, unless the rows are laid out from the top edge.
    starts_block: Vec<bool>,
    /// Whether a block ends at the row and the gap below it is on screen:
    /// not at the bottom row, which keeps to the bottom edge.
    ends_block: Vec<bool>,
}

impl RowLayout {
    /// The height of one row.
    pub(crate) fn cell_height(&self) -> f32 {
        self.cell_height
    }

    /// How many lines the view is scrolled up from the bottom of the
    /// history.
    pub(crate) fn display_offset(&self) -> i32 {
        self.display_offset
    }

    /// Whether the rows are laid out from the top edge, one step past the
    /// top of the history.
    pub(crate) fn is_past_the_top(&self) -> bool {
        self.is_past_the_top
    }

    /// Lay out the rows of `view`, each `cell_height` high, around the
    /// blocks on screen. `is_past_the_top` asks for the step past the top
    /// of the history; it is taken only where there is one, see
    /// [`RowLayout::can_go_past_the_top`].
    pub(crate) fn of_view(
        view: &SnapshotView<'_>,
        cell_height: f32,
        is_past_the_top: bool,
    ) -> Self {
        let rows = view.size.screen_lines;
        let display_offset = i32::try_from(view.display_offset).unwrap_or(0);
        let is_full_screen = view.is_full_screen();
        let mut starts_block = vec![false; rows];
        let mut ends_block = vec![false; rows];

        if !is_full_screen {
            for block in view.blocks().iter().filter(|b| b.line_count > 0) {
                let first = block.start_line + display_offset;
                let last = first + count_as_i32(block.line_count) - 1;
                if let Some(starts) = row_flag(&mut starts_block, first)
                    && first > 0
                {
                    *starts = true;
                }
                if let Some(ends) = row_flag(&mut ends_block, last)
                    && last < count_as_i32(rows) - 1
                {
                    *ends = true;
                }
            }
        }

        let from_the_bottom =
            row_tops_from_the_bottom(&starts_block, &ends_block, cell_height);
        // At the top of the history its first block is on screen; the gaps
        // cut it when its own gap does not fit above it.
        let top_row =
            if is_past_the_top && !is_full_screen && is_at_the_top(view) {
                first_block_row(view, rows, display_offset)
                    .filter(|row| from_the_bottom[*row] < BLOCK_GAP)
            } else {
                None
            };

        let row_tops = match top_row {
            Some(row) => {
                starts_block[row] = true;
                row_tops_from_the_top(
                    &starts_block,
                    &ends_block,
                    cell_height,
                    row,
                )
            },
            None => from_the_bottom,
        };

        Self {
            cell_height,
            display_offset,
            is_past_the_top: top_row.is_some(),
            row_tops,
            starts_block,
            ends_block,
        }
    }

    /// Whether the view of `view`, with rows `cell_height` high, can go one
    /// step past the top of the history: it is at the top of the history,
    /// and the gaps push the first block, or the gap above it, over the top
    /// edge. A full-screen program has no gaps, so it can not.
    pub(crate) fn can_go_past_the_top(
        view: &SnapshotView<'_>,
        cell_height: f32,
    ) -> bool {
        Self::of_view(view, cell_height, true).is_past_the_top()
    }

    /// Where the row of the grid line `line` (the line of a cell of the
    /// snapshot) starts. It may be above the top edge, where the gaps
    /// pushed it.
    pub(crate) fn line_top(&self, line: i32) -> f32 {
        self.row_top(line + self.display_offset)
    }

    /// The grid line of the row at `y`. A pixel in a gap is on no row of
    /// text; it gives the nearest row of the block the gap belongs to: the
    /// gap above a block belongs to its first row, the gap below a block
    /// and the line under it to its last row. So the block at that line is
    /// the nearest block, for the pointer and for clicks. Above the rows it
    /// gives the top row, below them the line grid goes on from the bottom
    /// row.
    pub(crate) fn line_at(&self, y: f32) -> i32 {
        let rows = self.row_tops.len();
        let last_top = self.row_tops.last().copied().unwrap_or(0.0);
        let row = if rows > 0 && y < last_top + self.cell_height {
            (0..rows)
                .rposition(|row| self.band_top(row) <= y)
                .map_or(0, count_as_i32)
        } else if rows > 0 && self.cell_height > 0.0 {
            let below = ((y - last_top) / self.cell_height).floor() as i32;
            count_as_i32(rows) - 1 + below
        } else if self.cell_height > 0.0 {
            (y / self.cell_height).floor().max(0.0) as i32
        } else {
            0
        };

        row - self.display_offset
    }

    /// Where `block` lies, from its top to its bottom: its rows on screen,
    /// with the gap above the first one and the gap and the line below the
    /// last one when they are on screen. `None` for a block with no row on
    /// screen.
    pub(crate) fn block_span(
        &self,
        block: &BlockSnapshot,
    ) -> Option<Range<f32>> {
        if block.line_count == 0 {
            return None;
        }

        let first = block.start_line + self.display_offset;
        let last = first + count_as_i32(block.line_count) - 1;
        let top = self.row_top(first) - self.gap_above(first);
        let bottom =
            self.row_top(last) + self.cell_height + self.gap_below(last);

        Some(top..bottom)
    }

    /// Whether the gap and the line under the last row of `block` are on
    /// screen. The bottom row has none under it, since it keeps to the
    /// bottom edge, and a block with no row on screen has none either.
    pub(crate) fn has_gap_below(&self, block: &BlockSnapshot) -> bool {
        if block.line_count == 0 {
            return false;
        }

        let last = block.start_line
            + self.display_offset
            + count_as_i32(block.line_count)
            - 1;

        self.gap_below(last) > 0.0
    }

    /// Where the status line of `block` lies: from the top of the block,
    /// the gap above it included, to the bottom of its status row. What is
    /// above the top edge is left out: the gap, and a part of the status
    /// row that is less than half of it. `None` when the block shows no
    /// status line, and when the gaps pushed more than half of the status
    /// row over the top edge or any of it over the bottom of the line grid
    /// (see [`RowLayout::shows_status_row`]).
    pub(crate) fn status_line_span(
        &self,
        block: &BlockSnapshot,
    ) -> Option<Range<f32>> {
        if !block.is_status_line_visible || block.line_count == 0 {
            return None;
        }

        let row = block.start_line + self.display_offset;
        if !self.shows_status_row(row) {
            return None;
        }

        let row_top = self.row_top(row);
        let top = (row_top - self.gap_above(row)).max(0.0);

        Some(top..row_top + self.cell_height)
    }

    /// How many top rows the gaps push over the top edge, even in part.
    pub(crate) fn rows_over_the_top(&self) -> usize {
        self.row_tops.iter().take_while(|top| **top < 0.0).count()
    }

    /// Whether the pane shows all of `block`: every row it has in the
    /// history, with the gap above its first row, between the top edge and
    /// the bottom of the line grid.
    pub(crate) fn shows_whole_block(&self, block: &BlockSnapshot) -> bool {
        if block.line_count == 0 || block.line_count != block.history_line_count
        {
            return false;
        }

        let last = block.start_line
            + self.display_offset
            + count_as_i32(block.line_count)
            - 1;
        let is_top_in_pane =
            self.block_span(block).is_some_and(|span| span.start >= 0.0);

        is_top_in_pane && !self.is_cut_at_the_bottom(last)
    }

    /// The blocks of `blocks` with a row in the pane, as the pane shows
    /// them. A row the gaps push wholly over the top edge is above the
    /// pane: a block loses it from its lines on screen. A row that still
    /// shows in part at the top edge stays on screen, so the block it
    /// belongs to is in the pane (the sticky header takes it), but it is
    /// cut: the status line is not shown when more than half of its row is
    /// cut (see [`RowLayout::shows_status_row`]), and a cut row of text
    /// counts among the lines above. Only the gap of a block left
    /// at the top edge does not keep the block in the pane. The rows the
    /// gaps push over the bottom of the line grid, even in part, count as
    /// below the pane: a block loses them from its lines on screen. A block
    /// left with no row, like a block above or below the pane, is left out.
    pub(crate) fn blocks_in_pane(
        &self,
        blocks: &[BlockSnapshot],
    ) -> Vec<BlockSnapshot> {
        let rows = count_as_i32(self.row_tops.len());
        let over_the_top = count_as_i32(self.rows_over_the_top());
        let above_the_pane = count_as_i32(self.rows_above_the_pane());
        let first_below = rows - count_as_i32(self.rows_under_the_bottom());

        blocks
            .iter()
            .filter_map(|block| {
                let first = block.start_line + self.display_offset;
                let end = first + count_as_i32(block.line_count);
                let above = usize::try_from(above_the_pane.min(end) - first)
                    .unwrap_or(0);
                let cut =
                    usize::try_from(over_the_top.min(end) - first).unwrap_or(0);
                let cut_below =
                    usize::try_from(end.min(rows) - first_below.max(first))
                        .unwrap_or(0);
                if block.line_count <= above + cut_below {
                    return None;
                }

                let mut shown = block.clone();
                shown.start_line += count_as_i32(above);
                shown.line_count -= above + cut_below;
                if cut > 0 {
                    let cut_status = usize::from(block.is_status_line_visible);
                    shown.is_status_line_visible = block.is_status_line_visible
                        && self.shows_status_row(first);
                    shown.text_lines_above += cut - cut_status;
                }

                Some(shown)
            })
            .collect()
    }

    /// Whether the status row `row` shows its status line: at least half
    /// of the row is below the top edge, and none of it is over the bottom
    /// of the line grid. So a few pixels the gaps cut at the top leave the
    /// status line in the pane, and no sticky header lies over it.
    fn shows_status_row(&self, row: i32) -> bool {
        self.row_top(row) >= -self.cell_height / 2.0
            && !self.is_cut_at_the_bottom(row)
    }

    /// Where the row `row` (counted from the top row on screen) starts.
    fn row_top(&self, row: i32) -> f32 {
        let rows = count_as_i32(self.row_tops.len());

        match usize::try_from(row) {
            Ok(index) if row < rows => self.row_tops[index],
            Ok(_) => match self.row_tops.last() {
                Some(last_top) => {
                    last_top + (row - rows + 1) as f32 * self.cell_height
                },
                None => row as f32 * self.cell_height,
            },
            Err(_) => {
                let first_top = self.row_tops.first().copied().unwrap_or(0.0);
                first_top + row as f32 * self.cell_height
            },
        }
    }

    /// The gap on screen above the row `row`, when a block starts there.
    fn gap_above(&self, row: i32) -> f32 {
        let starts = usize::try_from(row)
            .ok()
            .and_then(|row| self.starts_block.get(row))
            .copied()
            .unwrap_or(false);

        if starts { BLOCK_GAP } else { 0.0 }
    }

    /// The gap and the line on screen below the row `row`, when a block
    /// ends there.
    fn gap_below(&self, row: i32) -> f32 {
        let ends = usize::try_from(row)
            .ok()
            .and_then(|row| self.ends_block.get(row))
            .copied()
            .unwrap_or(false);

        if ends { BLOCK_GAP_BELOW } else { 0.0 }
    }

    /// How many top rows the gaps push wholly over the top edge: no part of
    /// them is in the pane.
    fn rows_above_the_pane(&self) -> usize {
        self.row_tops
            .iter()
            .take_while(|top| **top + self.cell_height <= 0.0)
            .count()
    }

    /// How many bottom rows the gaps push over the bottom of the line
    /// grid, even in part. Only rows laid out from the top edge can be.
    fn rows_under_the_bottom(&self) -> usize {
        let rows = count_as_i32(self.row_tops.len());

        (0..rows)
            .rev()
            .take_while(|row| self.is_cut_at_the_bottom(*row))
            .count()
    }

    /// Whether the row `row` is on screen and the gaps pushed it over the
    /// bottom of the line grid, even in part.
    fn is_cut_at_the_bottom(&self, row: i32) -> bool {
        let grid_bottom = self.row_tops.len() as f32 * self.cell_height;

        usize::try_from(row)
            .ok()
            .and_then(|row| self.row_tops.get(row))
            .is_some_and(|top| top + self.cell_height > grid_bottom)
    }

    /// Where the part of the pane that belongs to the row `row` starts:
    /// the row, with the gap above it when a block starts there.
    fn band_top(&self, row: usize) -> f32 {
        let gap = if self.starts_block[row] {
            BLOCK_GAP
        } else {
            0.0
        };

        self.row_tops[row] - gap
    }
}

/// Whether `view` is at the top of the history: scrolled up as far as the
/// history goes, which is not at all for a history that fits in the pane
/// by its lines.
pub(crate) fn is_at_the_top(view: &SnapshotView<'_>) -> bool {
    view.display_offset >= view.size.max_display_offset()
}

/// The row of `rows` where the first block of `view` with a line on screen
/// starts, scrolled up by `display_offset` lines; `None` when it starts
/// above the rows or there is none.
fn first_block_row(
    view: &SnapshotView<'_>,
    rows: usize,
    display_offset: i32,
) -> Option<usize> {
    let block = view.blocks().iter().find(|block| block.line_count > 0)?;
    let row = usize::try_from(block.start_line + display_offset).ok()?;

    (row < rows).then_some(row)
}

/// The top of every row, from the bottom edge of the line grid up: every
/// row, and the space above it. A gap pushes the rows above it up.
fn row_tops_from_the_bottom(
    starts_block: &[bool],
    ends_block: &[bool],
    cell_height: f32,
) -> Vec<f32> {
    let rows = starts_block.len();
    let mut row_tops = vec![0.0; rows];
    let mut top = rows as f32 * cell_height;

    for row in (0..rows).rev() {
        top -= cell_height;
        row_tops[row] = top;

        if starts_block[row] {
            top -= BLOCK_GAP;
        }
        if row > 0 && ends_block[row - 1] {
            top -= BLOCK_GAP_BELOW;
        }
    }

    row_tops
}

/// The top of every row, from the top edge down, where the row `first_row`
/// (with the space above it) starts at the top edge: the space above every
/// row, and the row. The rows above `first_row` hold no block and go over
/// the top edge. A gap pushes the rows below it down.
fn row_tops_from_the_top(
    starts_block: &[bool],
    ends_block: &[bool],
    cell_height: f32,
    first_row: usize,
) -> Vec<f32> {
    let mut row_tops = Vec::with_capacity(starts_block.len());
    let mut top = -(first_row as f32) * cell_height;

    for (starts, ends) in starts_block.iter().zip(ends_block) {
        if *starts {
            top += BLOCK_GAP;
        }
        row_tops.push(top);

        top += cell_height;
        if *ends {
            top += BLOCK_GAP_BELOW;
        }
    }

    row_tops
}

/// The flag of the row `row` in `flags`, when the row is on screen.
fn row_flag(flags: &mut [bool], row: i32) -> Option<&mut bool> {
    usize::try_from(row).ok().and_then(|row| flags.get_mut(row))
}

/// `count` as a number of rows that may be added to a grid line.
fn count_as_i32(count: usize) -> i32 {
    i32::try_from(count).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use otty_libterm::surface::{
        BlockKey, BlockKind, BlockMeta, Column, Line, Point, SnapshotOwned,
        SurfaceMode,
    };

    use super::*;

    const CELL_HEIGHT: f32 = 16.0;

    /// A command block of `line_count` rows from the grid line
    /// `start_line`.
    fn block(key: u64, start_line: i32, line_count: usize) -> BlockSnapshot {
        BlockSnapshot {
            key: BlockKey::new(key),
            meta: BlockMeta {
                kind: BlockKind::Command,
                ..BlockMeta::default()
            },
            start_line,
            line_count,
            history_line_count: line_count,
            ..BlockSnapshot::default()
        }
    }

    /// `block` with its status line on screen, as its first row.
    fn with_status_line(block: BlockSnapshot) -> BlockSnapshot {
        BlockSnapshot {
            has_status_line: true,
            is_status_line_visible: true,
            ..block
        }
    }

    fn snapshot_of(blocks: Vec<BlockSnapshot>) -> SnapshotOwned {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = blocks;

        snapshot
    }

    /// The layout of `snapshot` in a pane of `rows` rows, scrolled up by
    /// `display_offset` lines.
    fn layout_of(
        snapshot: &SnapshotOwned,
        rows: usize,
        display_offset: usize,
    ) -> RowLayout {
        let mut view = snapshot.view();
        view.size.screen_lines = rows;
        view.display_offset = display_offset;

        RowLayout::of_view(&view, CELL_HEIGHT, false)
    }

    /// The layout of `snapshot` in a pane of `rows` rows, in a history of
    /// `history_lines` lines scrolled up by `display_offset` lines, and one
    /// step past the top of the history when `is_past_the_top`.
    fn layout_in_history(
        snapshot: &SnapshotOwned,
        rows: usize,
        history_lines: usize,
        display_offset: usize,
        is_past_the_top: bool,
    ) -> RowLayout {
        let mut view = snapshot.view();
        view.size.screen_lines = rows;
        view.size.total_lines = history_lines;
        view.display_offset = display_offset;

        RowLayout::of_view(&view, CELL_HEIGHT, is_past_the_top)
    }

    /// Whether the view of `snapshot` in a pane of `rows` rows, in a
    /// history of `history_lines` lines scrolled up by `display_offset`
    /// lines, can go one step past the top of the history.
    fn can_go_past_the_top_in(
        snapshot: &SnapshotOwned,
        rows: usize,
        history_lines: usize,
        display_offset: usize,
    ) -> bool {
        let mut view = snapshot.view();
        view.size.screen_lines = rows;
        view.size.total_lines = history_lines;
        view.display_offset = display_offset;

        RowLayout::can_go_past_the_top(&view, CELL_HEIGHT)
    }

    /// Three blocks of one row each at the bottom of a pane of 5 rows: the
    /// two rows above them hold no block.
    fn three_one_row_blocks() -> SnapshotOwned {
        snapshot_of(vec![block(1, 2, 1), block(2, 3, 1), block(3, 4, 1)])
    }

    #[test]
    fn rows_of_one_row_blocks_are_12_px_and_the_line_apart_and_the_last_ends_at_the_bottom()
     {
        let snapshot = three_one_row_blocks();

        let layout = layout_of(&snapshot, 5, 0);

        let apart = BLOCK_GAP * 2.0 + BLOCK_LINE_HEIGHT;
        assert_eq!(layout.line_top(4) + CELL_HEIGHT, 5.0 * CELL_HEIGHT);
        assert_eq!(
            layout.line_top(4) - (layout.line_top(3) + CELL_HEIGHT),
            apart
        );
        assert_eq!(
            layout.line_top(3) - (layout.line_top(2) + CELL_HEIGHT),
            apart
        );
        // The first block has the gap above it too; the rows over it hold
        // no block and touch.
        assert_eq!(
            layout.line_top(2) - (layout.line_top(1) + CELL_HEIGHT),
            BLOCK_GAP
        );
        assert_eq!(
            layout.line_top(1) - (layout.line_top(0) + CELL_HEIGHT),
            0.0
        );
    }

    #[test]
    fn a_block_taller_than_the_pane_leaves_every_row_on_the_line_grid() {
        let snapshot = snapshot_of(vec![block(1, -3, 5), block(2, 2, 3)]);

        let layout = layout_of(&snapshot, 5, 3);

        for line in -3..2 {
            let row = (line + 3) as f32;
            assert_eq!(layout.line_top(line), row * CELL_HEIGHT);
        }
        let span = layout.block_span(&snapshot.blocks[0]);
        assert_eq!(span, Some(0.0..5.0 * CELL_HEIGHT));
    }

    #[test]
    fn a_pixel_in_a_gap_is_on_no_row_of_text_but_belongs_to_the_nearest_block()
    {
        let snapshot = three_one_row_blocks();
        let layout = layout_of(&snapshot, 5, 0);
        let first_bottom = layout.line_top(2) + CELL_HEIGHT;
        let second_top = layout.line_top(3);
        // Below the first block, on its line, and above the second block.
        let below_first = first_bottom + 1.0;
        let on_the_line = second_top - BLOCK_GAP - 0.5;
        let above_second = second_top - 1.0;

        let lines = [
            layout.line_at(below_first),
            layout.line_at(on_the_line),
            layout.line_at(above_second),
        ];

        assert_eq!(lines, [2, 2, 3]);
        for y in [below_first, on_the_line, above_second] {
            let line = layout.line_at(y);
            let top = layout.line_top(line);
            assert!(y < top || y >= top + CELL_HEIGHT, "{y} is on line {line}");
        }
        let view = snapshot.view();
        let block_at = |line: i32| {
            view.block_at_point(Point::new(Line(line), Column(0)))
                .map(|block| block.key)
        };
        assert_eq!(block_at(lines[0]), Some(BlockKey::new(1)));
        assert_eq!(block_at(lines[1]), Some(BlockKey::new(1)));
        assert_eq!(block_at(lines[2]), Some(BlockKey::new(2)));
    }

    #[test]
    fn the_line_at_a_pixel_and_the_pixel_of_that_line_agree_for_every_row() {
        // Scrolled up by 2 lines: the third block runs on below the pane.
        let snapshot = snapshot_of(vec![
            with_status_line(block(1, -2, 3)),
            with_status_line(block(2, 1, 2)),
            with_status_line(block(3, 3, 4)),
        ]);

        let layout = layout_of(&snapshot, 6, 2);

        for line in -2..4 {
            let top = layout.line_top(line);
            assert_eq!(layout.line_at(top), line, "the top of {line}");
            assert_eq!(
                layout.line_at(top + CELL_HEIGHT - 0.5),
                line,
                "the bottom of {line}"
            );
        }
    }

    #[test]
    fn a_full_screen_program_has_its_rows_on_the_line_grid() {
        let snapshot = three_one_row_blocks();
        let mut view = snapshot.view();
        view.size.screen_lines = 5;
        view.mode = SurfaceMode::ALT_SCREEN;

        let layout = RowLayout::of_view(&view, CELL_HEIGHT, true);

        for line in 0..5 {
            assert_eq!(layout.line_top(line), line as f32 * CELL_HEIGHT);
            assert_eq!(layout.line_at(line as f32 * CELL_HEIGHT), line);
        }
        // It has no gaps to cut its top row, so no step past the top.
        assert!(!layout.is_past_the_top());
        assert!(!RowLayout::can_go_past_the_top(&view, CELL_HEIGHT));
    }

    #[test]
    fn below_the_rows_and_without_rows_the_line_grid_goes_on() {
        let snapshot = three_one_row_blocks();

        let laid_out = layout_of(&snapshot, 5, 0);
        let without_rows = layout_of(&snapshot, 0, 0);

        assert_eq!(laid_out.line_at(5.0 * CELL_HEIGHT + 1.0), 5);
        assert_eq!(laid_out.line_at(-100.0), 0);
        assert_eq!(without_rows.line_top(3), 3.0 * CELL_HEIGHT);
        assert_eq!(without_rows.line_at(3.0 * CELL_HEIGHT), 3);
    }

    #[test]
    fn a_block_spans_its_rows_with_the_gap_above_and_the_gap_and_the_line_below()
     {
        let snapshot = three_one_row_blocks();
        let layout = layout_of(&snapshot, 5, 0);

        let middle = layout.block_span(&snapshot.blocks[1]);
        let last = layout.block_span(&snapshot.blocks[2]);

        let top = layout.line_top(3);
        let below = BLOCK_GAP_BELOW;
        assert_eq!(middle, Some(top - BLOCK_GAP..top + CELL_HEIGHT + below));
        // The bottom row stays at the bottom edge: no gap under it.
        let last_top = layout.line_top(4);
        assert_eq!(last, Some(last_top - BLOCK_GAP..5.0 * CELL_HEIGHT));
        assert_eq!(layout.block_span(&block(4, 5, 0)), None);
    }

    #[test]
    fn a_status_line_takes_the_gap_above_its_block() {
        let snapshot =
            snapshot_of(vec![with_status_line(block(1, 2, 2)), block(2, 4, 1)]);
        let layout = layout_of(&snapshot, 5, 0);

        let span = layout.status_line_span(&snapshot.blocks[0]);

        let top = layout.line_top(2);
        assert_eq!(span, Some(top - BLOCK_GAP..top + CELL_HEIGHT));
        assert_eq!(layout.status_line_span(&snapshot.blocks[1]), None);
    }

    /// The status row is the top row and nothing pushes it up: the gap
    /// above it is over the top edge, so the status line keeps to the row.
    #[test]
    fn a_status_line_at_the_top_edge_keeps_to_its_row() {
        let snapshot = snapshot_of(vec![with_status_line(block(1, 0, 4))]);
        let layout = layout_of(&snapshot, 4, 0);

        let span = layout.status_line_span(&snapshot.blocks[0]);

        assert_eq!(span, Some(0.0..CELL_HEIGHT));
    }

    /// `ls` holds the first two rows, its status line and its command,
    /// and two one-row blocks follow. The gaps push the status row of `ls`
    /// wholly and its command row in part over the top edge.
    fn ls_pushed_over_the_top() -> SnapshotOwned {
        snapshot_of(vec![
            with_status_line(block(1, 0, 2)),
            block(2, 2, 1),
            block(3, 3, 1),
        ])
    }

    /// The sticky header takes the first block in the pane: a block whose
    /// last row still shows in part at the top edge is that block.
    #[test]
    fn a_block_whose_last_row_is_partly_over_the_top_edge_stays_in_the_pane_with_that_row_counted_above()
     {
        let snapshot = ls_pushed_over_the_top();
        let layout = layout_of(&snapshot, 4, 0);
        assert!(layout.line_top(0) + CELL_HEIGHT <= 0.0);
        assert!(layout.line_top(1) < 0.0);
        assert!(layout.line_top(1) + CELL_HEIGHT > 0.0);
        assert_eq!(layout.rows_over_the_top(), 2);

        let blocks = layout.blocks_in_pane(snapshot.view().blocks());

        let ls = &blocks[0];
        assert_eq!(ls.key, BlockKey::new(1));
        assert_eq!(ls.start_line, 1);
        assert_eq!(ls.line_count, 1);
        assert!(!ls.is_status_line_visible);
        assert_eq!(ls.text_lines_above, 1);
        assert_eq!(blocks[1..], snapshot.blocks[1..]);
        assert_eq!(layout.status_line_span(&snapshot.blocks[0]), None);
    }

    /// `ls` holds the first three rows and four one-row blocks follow in a
    /// pane of 7 rows. The gaps push every row of `ls` over the top edge;
    /// only the gap and the line under it are left in the pane.
    #[test]
    fn a_block_of_which_only_the_gap_is_left_at_the_top_is_not_in_the_pane() {
        let snapshot = snapshot_of(vec![
            with_status_line(block(1, 0, 3)),
            block(2, 3, 1),
            block(3, 4, 1),
            block(4, 5, 1),
            block(5, 6, 1),
        ]);
        let layout = layout_of(&snapshot, 7, 0);
        let ls_bottom = layout.line_top(2) + CELL_HEIGHT;
        assert!(ls_bottom <= 0.0);
        assert!(ls_bottom + BLOCK_GAP_BELOW > 0.0);

        let keys: Vec<BlockKey> = layout
            .blocks_in_pane(snapshot.view().blocks())
            .iter()
            .map(|block| block.key)
            .collect();

        assert_eq!(
            keys,
            [
                BlockKey::new(2),
                BlockKey::new(3),
                BlockKey::new(4),
                BlockKey::new(5)
            ]
        );
    }

    #[test]
    fn blocks_with_no_row_on_screen_are_not_in_the_pane() {
        let snapshot =
            snapshot_of(vec![block(1, -5, 0), block(2, 0, 3), block(3, 3, 1)]);
        let layout = layout_of(&snapshot, 4, 0);

        let keys: Vec<BlockKey> = layout
            .blocks_in_pane(snapshot.view().blocks())
            .iter()
            .map(|block| block.key)
            .collect();

        assert_eq!(keys, [BlockKey::new(2), BlockKey::new(3)]);
    }

    /// Issue 18: the gap and the line between the blocks push 13 px of the
    /// 16 px status row over the top edge, more than half of it. The block
    /// keeps the row, but its status line is not shown, so the sticky
    /// header takes it.
    #[test]
    fn a_block_whose_status_row_is_more_than_half_over_the_top_edge_keeps_that_row_but_not_its_status_line()
     {
        let snapshot =
            snapshot_of(vec![with_status_line(block(1, 0, 4)), block(2, 4, 1)]);
        let layout = layout_of(&snapshot, 5, 0);
        assert_eq!(layout.line_top(0), -BLOCK_GAP_BELOW - BLOCK_GAP);
        assert!(layout.line_top(1) >= 0.0);

        let blocks = layout.blocks_in_pane(snapshot.view().blocks());

        let first = &blocks[0];
        assert_eq!(first.start_line, 0);
        assert_eq!(first.line_count, 4);
        assert!(!first.is_status_line_visible);
        assert_eq!(first.text_lines_above, 0);
        assert_eq!(layout.status_line_span(&snapshot.blocks[0]), None);
    }

    /// Issue 18: the blocks of the test above in rows of 30 px. The gap
    /// and the line push 13 px of the status row over the top edge, less
    /// than half of it: the status line is still shown, on what is left of
    /// its row, so no sticky header lies over it.
    #[test]
    fn a_block_whose_status_row_is_less_than_half_over_the_top_edge_keeps_its_status_line()
     {
        let snapshot =
            snapshot_of(vec![with_status_line(block(1, 0, 4)), block(2, 4, 1)]);
        let mut view = snapshot.view();
        view.size.screen_lines = 5;
        let layout = RowLayout::of_view(&view, 30.0, false);
        assert_eq!(layout.line_top(0), -13.0);

        let blocks = layout.blocks_in_pane(snapshot.view().blocks());

        let first = &blocks[0];
        assert_eq!((first.start_line, first.line_count), (0, 4));
        assert!(first.is_status_line_visible);
        assert_eq!(first.text_lines_above, 0);
        assert_eq!(
            layout.status_line_span(&snapshot.blocks[0]),
            Some(0.0..17.0)
        );
    }

    /// The top of a history of 10 lines in a pane of 4 rows (scrolled up
    /// by 6 lines): `ls` holds the two top rows, its status line and its
    /// command, and `seq` the next two of its three lines.
    fn top_of_the_history() -> SnapshotOwned {
        let mut seq = with_status_line(block(2, -4, 2));
        seq.history_line_count = 3;

        snapshot_of(vec![with_status_line(block(1, -6, 2)), seq])
    }

    /// The same history one line below its top (scrolled up by 5 lines):
    /// the second line of `ls` is the top row, and `seq` holds the three
    /// rows under it.
    fn a_line_below_the_top_of_the_history() -> SnapshotOwned {
        let mut ls = block(1, -5, 1);
        ls.has_status_line = true;
        ls.history_line_count = 2;

        snapshot_of(vec![ls, with_status_line(block(2, -4, 3))])
    }

    /// The model is at its top, but the user did not ask to go further up:
    /// the rows keep to the bottom edge and the gaps cut the top of `ls`.
    #[test]
    fn at_the_top_of_the_history_the_bottom_row_stays_at_the_bottom_edge_until_the_step_past_the_top()
     {
        let snapshot = top_of_the_history();

        let layout = layout_in_history(&snapshot, 4, 10, 6, false);

        assert_eq!(layout.line_top(-3) + CELL_HEIGHT, 4.0 * CELL_HEIGHT);
        assert!(layout.line_top(-6) < 0.0);
        assert!(!layout.is_past_the_top());
        assert!(can_go_past_the_top_in(&snapshot, 4, 10, 6));
    }

    /// The last wheel step to the top of the history moves the text by
    /// one line, as every other step does; only the step after it lays
    /// the rows out from the top edge.
    #[test]
    fn the_last_step_to_the_top_of_the_history_moves_the_rows_by_one_line() {
        let below = a_line_below_the_top_of_the_history();
        let top = top_of_the_history();

        let one_line_down = layout_in_history(&below, 4, 10, 5, false);
        let at_the_top = layout_in_history(&top, 4, 10, 6, false);
        let past_the_top = layout_in_history(&top, 4, 10, 6, true);

        // The status line of `seq`, the line -4 in both.
        assert_eq!(
            at_the_top.line_top(-4) - one_line_down.line_top(-4),
            CELL_HEIGHT
        );
        assert!(!can_go_past_the_top_in(&below, 4, 10, 5));
        assert_eq!(past_the_top.line_top(-6), BLOCK_GAP);
    }

    /// Asked to be past the top while the view is not at the top of the
    /// history, the rows still keep to the bottom edge.
    #[test]
    fn away_from_the_top_of_the_history_the_step_past_the_top_does_nothing() {
        let snapshot = a_line_below_the_top_of_the_history();

        let asked = layout_in_history(&snapshot, 4, 10, 5, true);
        let not_asked = layout_in_history(&snapshot, 4, 10, 5, false);

        assert_eq!(asked, not_asked);
        assert!(!asked.is_past_the_top());
    }

    #[test]
    fn past_the_top_of_the_history_the_first_block_starts_at_the_top_edge_with_its_gap()
     {
        let snapshot = top_of_the_history();

        let layout = layout_in_history(&snapshot, 4, 10, 6, true);

        assert!(layout.is_past_the_top());
        assert_eq!(layout.line_top(-6), BLOCK_GAP);
        assert_eq!(layout.line_top(-5), BLOCK_GAP + CELL_HEIGHT);
        // The gap and the line below `ls`, and the gap above `seq`.
        let seq_top = BLOCK_GAP + 2.0 * CELL_HEIGHT + 2.0 * BLOCK_GAP + 1.0;
        assert_eq!(layout.line_top(-4), seq_top);
        assert_eq!(layout.line_top(-3), seq_top + CELL_HEIGHT);
        assert_eq!(layout.rows_over_the_top(), 0);
    }

    #[test]
    fn past_the_top_of_the_history_the_first_status_line_is_whole() {
        let snapshot = top_of_the_history();

        let layout = layout_in_history(&snapshot, 4, 10, 6, true);

        let span = layout.status_line_span(&snapshot.blocks[0]);
        assert_eq!(span, Some(0.0..BLOCK_GAP + CELL_HEIGHT));
        // The status row of `seq` does not fit above the bottom edge.
        assert_eq!(layout.status_line_span(&snapshot.blocks[1]), None);
    }

    #[test]
    fn past_the_top_of_the_history_a_pixel_on_a_row_cut_at_the_bottom_is_on_that_row()
     {
        let snapshot = top_of_the_history();
        let layout = layout_in_history(&snapshot, 4, 10, 6, true);
        let seq_top = layout.line_top(-4);
        assert!(seq_top + CELL_HEIGHT > 4.0 * CELL_HEIGHT);

        let in_the_pane = layout.line_at(4.0 * CELL_HEIGHT - 1.0);
        let under_the_grid = layout.line_at(4.0 * CELL_HEIGHT + 1.0);
        let in_the_gap_above_seq = layout.line_at(seq_top - 1.0);

        assert_eq!(in_the_pane, -4);
        assert_eq!(under_the_grid, -4);
        assert_eq!(in_the_gap_above_seq, -4);
        assert_eq!(layout.line_at(0.0), -6);
    }

    /// The sticky header and the status lines take the blocks the pane
    /// shows: rows cut at the bottom edge, even in part, are below it.
    #[test]
    fn past_the_top_of_the_history_rows_cut_at_the_bottom_edge_are_not_in_the_pane()
     {
        let snapshot = top_of_the_history();
        let layout = layout_in_history(&snapshot, 4, 10, 6, true);

        let blocks = layout.blocks_in_pane(snapshot.view().blocks());

        assert_eq!(blocks, snapshot.blocks[..1]);
    }

    /// Many tests build snapshots without rows: nothing is laid out, so
    /// nothing is cut.
    #[test]
    fn without_rows_every_block_is_in_the_pane() {
        let snapshot = three_one_row_blocks();
        let layout = layout_of(&snapshot, 0, 0);

        let blocks = layout.blocks_in_pane(snapshot.view().blocks());

        assert_eq!(blocks, snapshot.blocks);
    }

    #[test]
    fn a_line_below_the_top_of_the_history_the_bottom_row_is_at_the_bottom_edge()
     {
        let mut snapshot = top_of_the_history();
        for block in &mut snapshot.blocks {
            block.start_line += 1;
        }

        let layout = layout_in_history(&snapshot, 4, 10, 5, false);

        assert_eq!(layout.line_top(-2) + CELL_HEIGHT, 4.0 * CELL_HEIGHT);
        assert!(layout.rows_over_the_top() > 0);
    }

    /// A new terminal: the whole history fits in the pane, so the prompt
    /// stays at the bottom edge. The gaps push only the two empty rows
    /// over the top edge, so nothing is cut and there is no step past the
    /// top.
    #[test]
    fn a_history_that_fits_in_the_pane_keeps_the_bottom_row_at_the_bottom_edge()
    {
        let snapshot = three_one_row_blocks();

        let layout = layout_in_history(&snapshot, 5, 5, 0, false);
        let asked_past_the_top = layout_in_history(&snapshot, 5, 5, 0, true);

        assert_eq!(layout.line_top(4) + CELL_HEIGHT, 5.0 * CELL_HEIGHT);
        assert_eq!(layout.rows_over_the_top(), 2);
        assert!(!can_go_past_the_top_in(&snapshot, 5, 5, 0));
        assert_eq!(asked_past_the_top, layout);
    }

    /// A new terminal after `ls` and `pwd`: 4 lines, as many as the pane
    /// has rows, so the history can not scroll. The gaps push the status
    /// line of `ls` over the top edge.
    fn short_history_cut_at_the_top() -> SnapshotOwned {
        snapshot_of(vec![
            with_status_line(block(1, 0, 2)),
            with_status_line(block(2, 2, 1)),
            block(3, 3, 1),
        ])
    }

    #[test]
    fn a_short_history_cut_at_the_top_shows_its_first_block_whole_past_the_top()
    {
        let snapshot = short_history_cut_at_the_top();
        let at_the_bottom = layout_in_history(&snapshot, 4, 4, 0, false);
        assert!(at_the_bottom.line_top(0) < 0.0);
        assert!(can_go_past_the_top_in(&snapshot, 4, 4, 0));

        let layout = layout_in_history(&snapshot, 4, 4, 0, true);

        assert!(layout.is_past_the_top());
        assert_eq!(layout.line_top(0), BLOCK_GAP);
        let span = layout.status_line_span(&snapshot.blocks[0]);
        assert_eq!(span, Some(0.0..BLOCK_GAP + CELL_HEIGHT));
        let blocks = layout.blocks_in_pane(snapshot.view().blocks());
        assert_eq!(blocks[0], snapshot.blocks[0]);
    }

    /// A short history with an empty row above it: past the top, the
    /// first block starts at the top edge, and the empty row goes over it.
    #[test]
    fn past_the_top_the_empty_rows_above_a_short_history_go_over_the_top_edge()
    {
        let snapshot = snapshot_of(vec![
            with_status_line(block(1, 1, 2)),
            with_status_line(block(2, 3, 1)),
            block(3, 4, 1),
        ]);
        assert!(can_go_past_the_top_in(&snapshot, 5, 5, 0));

        let layout = layout_in_history(&snapshot, 5, 5, 0, true);

        assert_eq!(layout.line_top(1), BLOCK_GAP);
        assert_eq!(layout.line_top(0), -CELL_HEIGHT);
        assert_eq!(layout.line_at(0.0), 1);
        let blocks = layout.blocks_in_pane(snapshot.view().blocks());
        assert_eq!(blocks[0], snapshot.blocks[0]);
    }

    #[test]
    fn a_block_with_all_its_rows_and_its_gap_in_the_pane_is_shown_whole() {
        let snapshot = three_one_row_blocks();
        let layout = layout_of(&snapshot, 5, 0);

        let whole = layout.shows_whole_block(&snapshot.blocks[1]);

        assert!(whole);
    }

    #[test]
    fn a_block_the_gaps_push_over_the_top_edge_is_not_shown_whole() {
        let snapshot = ls_pushed_over_the_top();
        let layout = layout_of(&snapshot, 4, 0);

        let whole = layout.shows_whole_block(&snapshot.blocks[0]);

        assert!(!whole);
    }

    #[test]
    fn a_block_with_lines_off_screen_is_not_shown_whole() {
        let mut taller = block(1, 0, 3);
        taller.history_line_count = 5;
        let snapshot = snapshot_of(vec![taller]);
        let layout = layout_of(&snapshot, 3, 0);

        let whole = layout.shows_whole_block(&snapshot.blocks[0]);

        assert!(!whole);
    }

    #[test]
    fn past_the_top_of_the_history_a_block_cut_at_the_bottom_edge_is_not_shown_whole()
     {
        let mut snapshot = top_of_the_history();
        snapshot.blocks[1].history_line_count = 2;
        let layout = layout_in_history(&snapshot, 4, 10, 6, true);

        let first = layout.shows_whole_block(&snapshot.blocks[0]);
        let second = layout.shows_whole_block(&snapshot.blocks[1]);

        assert!(first);
        assert!(!second);
    }
}
