use iced::{Point, Rectangle, Size};
use otty_libterm::surface::{BlockKey, BlockKind, SnapshotView};

use crate::row_layout::RowLayout;

/// Rectangle describing the absolute geometry of a terminal block.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockRect {
    /// Key of the block the rectangle belongs to.
    pub block_key: BlockKey,
    /// Semantic kind of the block (prompt, command, fullscreen).
    pub kind: BlockKind,
    /// Absolute rectangle in layout coordinates: the rows of the block on
    /// screen with the space around them, see
    /// [`crate::Terminal::block_rects`].
    pub rect: Rectangle<f32>,
    /// Whether the command of the block ended with an exit code other
    /// than 0.
    pub(crate) is_failed: bool,
    /// Whether the gap and the line under the last row of the block are
    /// on screen: not when the block ends on the bottom row.
    pub(crate) has_gap_below: bool,
    /// Where the status line of the block lies, see
    /// [`BlockRect::status_line`].
    pub(crate) status_line: Option<Rectangle<f32>>,
}

impl BlockRect {
    /// Where the status line of the block lies, in layout coordinates: from
    /// the top of the block, the gap above its first line included, to the
    /// bottom of its status row, from edge to edge of the pane. A host puts
    /// what it shows on the status line there, so it is higher than a line
    /// of text by the gap. The gap is left out where it is above the top
    /// edge of the pane.
    ///
    /// `None` when the block shows no status line: it has none, its start
    /// is above the pane, or the gaps between the blocks pushed more than
    /// half of the status row over the top edge, or past the top of the
    /// history a part of it over the bottom of the line grid. While less
    /// than half of the row is over the top edge, the status line is what
    /// is left of the row in the pane.
    pub fn status_line(&self) -> Option<Rectangle<f32>> {
        self.status_line
    }
}

/// Where every block of `view` with a row on screen lies, for its rows
/// `rows`.
///
/// `layout_pos` and `layout_size` are the position and the size of the
/// whole terminal pane, so every rectangle runs from the left edge of the
/// pane to the right one. The text of the terminal starts further in, but
/// only to the left and the right.
///
/// A block takes its rows on screen and the space the row layout puts
/// around them: the gap above its first row and, under its last row, the
/// gap and the line that separates it from the next block. So a rectangle
/// may start above the top edge, and past the top of the history end below
/// the bottom edge. A full-screen program has no space around its rows.
pub(crate) fn block_rects(
    rows: &RowLayout,
    view: &SnapshotView<'_>,
    layout_pos: Point,
    layout_size: Size<f32>,
) -> Vec<BlockRect> {
    if layout_size.width <= 0.0 {
        return Vec::new();
    }

    let across_the_pane = |span: std::ops::Range<f32>| Rectangle {
        x: layout_pos.x,
        y: layout_pos.y + span.start,
        width: layout_size.width,
        height: span.end - span.start,
    };

    view.blocks()
        .iter()
        .filter_map(|block| {
            let span = rows.block_span(block)?;
            let status_line = rows.status_line_span(block).map(across_the_pane);

            Some(BlockRect {
                block_key: block.key,
                kind: block.meta.kind.clone(),
                rect: across_the_pane(span),
                is_failed: block.is_failed(),
                has_gap_below: rows.has_gap_below(block),
                status_line,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_libterm::surface::{BlockMeta, BlockSnapshot, SnapshotOwned};

    use super::*;
    use crate::testing::block_rects_of;

    fn block(
        key: u64,
        kind: BlockKind,
        start_line: i32,
        line_count: usize,
    ) -> BlockSnapshot {
        BlockSnapshot {
            key: BlockKey::new(key),
            meta: BlockMeta {
                kind,
                ..BlockMeta::default()
            },
            start_line,
            line_count,
            cached_text: None,
            has_status_line: false,
            is_status_line_visible: false,
            ..BlockSnapshot::default()
        }
    }

    /// A command block of one line whose command ended with `exit_code`.
    fn finished_block(key: u64, exit_code: Option<i32>) -> BlockSnapshot {
        let mut finished = block(key, BlockKind::Command, 0, 1);
        finished.meta.is_finished = true;
        finished.meta.exit_code = exit_code;

        finished
    }

    fn snapshot_with_blocks(blocks: Vec<BlockSnapshot>) -> SnapshotOwned {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = blocks;
        snapshot
    }

    #[test]
    fn skips_blocks_without_height() {
        let snapshot = snapshot_with_blocks(vec![
            block(1, BlockKind::Command, 0, 2),
            block(2, BlockKind::Command, 2, 0),
        ]);
        let view = snapshot.view();

        let rects = block_rects_of(
            &view,
            Point::new(10.0, 20.0),
            Size::new(120.0, 300.0),
            4.0,
        );

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].block_key, BlockKey::new(1));
        assert_eq!(rects[0].rect.height, 8.0);
    }

    #[test]
    fn accounts_for_display_offset_and_layout_origin() {
        let snapshot = snapshot_with_blocks(vec![
            block(1, BlockKind::Command, 0, 2),
            block(2, BlockKind::Prompt, 2, 1),
        ]);
        let mut view = snapshot.view();
        view.display_offset = 3;

        let rects = block_rects_of(
            &view,
            Point::new(5.0, 7.0),
            Size::new(80.0, 200.0),
            5.0,
        );

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].rect.y, 7.0 + ((0.0 + 3.0) * 5.0));
        assert_eq!(rects[0].rect.height, 10.0);
        assert_eq!(rects[0].rect.width, 80.0);
        assert_eq!(rects[1].rect.y, 7.0 + ((2.0 + 3.0) * 5.0));
    }

    #[test]
    fn a_block_failed_when_its_command_ended_with_a_code_other_than_zero() {
        let mut running = block(4, BlockKind::Command, 3, 1);
        running.meta.exit_code = Some(1);
        let snapshot = snapshot_with_blocks(vec![
            finished_block(1, Some(0)),
            finished_block(2, Some(101)),
            finished_block(3, None),
            running,
        ]);
        let view = snapshot.view();

        let rects =
            block_rects_of(&view, Point::ORIGIN, Size::new(80.0, 200.0), 5.0);

        let failed: Vec<bool> =
            rects.iter().map(|rect| rect.is_failed).collect();
        assert_eq!(failed, [false, true, false, false]);
    }

    /// The bottom row keeps to the bottom edge, so the prompt on it has no
    /// gap under it; the block above it has its gap and its line.
    #[test]
    fn only_a_block_that_ends_above_the_bottom_row_has_a_gap_below() {
        let snapshot = snapshot_with_blocks(vec![
            block(1, BlockKind::Command, 0, 2),
            block(2, BlockKind::Prompt, 2, 1),
        ]);
        let mut view = snapshot.view();
        view.size.screen_lines = 3;

        let rects =
            block_rects_of(&view, Point::ORIGIN, Size::new(80.0, 15.0), 5.0);

        assert!(rects[0].has_gap_below);
        assert!(!rects[1].has_gap_below);
    }

    /// Issue 07: a block of two rows over the prompt in a pane of 5 rows of
    /// 10 px. The prompt keeps to the bottom edge; the block is 6 px above
    /// it, with its own 6 px and its line under its rows and 6 px above.
    #[test]
    fn a_block_rect_covers_the_gaps_around_its_rows() {
        let snapshot = snapshot_with_blocks(vec![
            block(1, BlockKind::Command, 2, 2),
            block(2, BlockKind::Prompt, 4, 1),
        ]);
        let mut view = snapshot.view();
        view.size.screen_lines = 5;

        let rects = block_rects_of(
            &view,
            Point::new(0.0, 5.0),
            Size::new(80.0, 50.0),
            10.0,
        );

        // The prompt row: 40..50. Over it 6 px of its gap, then the line
        // and 6 px under the block, whose rows are 7..27.
        assert_eq!(rects[1].rect.y, 5.0 + 34.0);
        assert_eq!(rects[1].rect.height, 16.0);
        assert_eq!(rects[0].rect.y, 5.0 + 1.0);
        assert_eq!(rects[0].rect.height, 6.0 + 20.0 + 7.0);
        assert_eq!(rects[0].status_line(), None);
    }

    #[test]
    fn the_status_line_runs_from_the_top_of_the_block_to_the_bottom_of_its_status_row()
     {
        let mut ls = block(1, BlockKind::Command, 2, 2);
        ls.has_status_line = true;
        ls.is_status_line_visible = true;
        let snapshot =
            snapshot_with_blocks(vec![ls, block(2, BlockKind::Prompt, 4, 1)]);
        let mut view = snapshot.view();
        view.size.screen_lines = 5;

        let rects = block_rects_of(
            &view,
            Point::new(3.0, 5.0),
            Size::new(80.0, 50.0),
            10.0,
        );

        assert_eq!(
            rects[0].status_line(),
            Some(Rectangle {
                x: 3.0,
                y: 5.0 + 1.0,
                width: 80.0,
                height: 6.0 + 10.0,
            })
        );
    }
}
