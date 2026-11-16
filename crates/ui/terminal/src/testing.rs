//! Test helpers of the terminal widget: snapshots of a history of blocks,
//! as the block surface makes them, and where their blocks lie.

use iced::{Point, Size};
use otty_libterm::TerminalSize;
use otty_libterm::surface::{
    BlockKey, BlockKind, BlockSnapshot, Scroll, SnapshotOwned, SnapshotView,
    Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
};

use crate::block_layout::{self, BlockRect};
use crate::engine::Engine;
use crate::program::MockTerminalProgram;
use crate::row_layout::RowLayout;

/// A block of a test history: its key, how many lines it has in the
/// history, whether the first of them is its status line and how many
/// lines its fold hides.
pub(crate) struct HistoryBlock {
    key: BlockKey,
    lines: usize,
    has_status_line: bool,
    hidden_lines: Option<usize>,
}

impl HistoryBlock {
    /// A command block of `lines` lines; the first is its status line.
    pub(crate) fn command(key: BlockKey, lines: usize) -> Self {
        Self {
            key,
            lines,
            has_status_line: true,
            hidden_lines: None,
        }
    }

    /// A folded command block that shows `lines` lines, the first its
    /// status line, and hides `hidden_lines` more.
    pub(crate) fn folded(
        key: BlockKey,
        lines: usize,
        hidden_lines: usize,
    ) -> Self {
        Self {
            hidden_lines: Some(hidden_lines),
            ..Self::command(key, lines)
        }
    }

    /// A block of `lines` lines without a status line, such as the prompt.
    pub(crate) fn plain(key: BlockKey, lines: usize) -> Self {
        Self {
            key,
            lines,
            has_status_line: false,
            hidden_lines: None,
        }
    }
}

/// The snapshot of a pane of `rows` rows (36 columns of 8 by 16 px cells)
/// that shows `blocks` one after the other, scrolled up by `offset` lines,
/// as the block surface makes it: every block with its lines on screen and
/// the lines it has in the history. A history shorter than the pane is at
/// its bottom, under empty rows.
pub(crate) fn history_snapshot(
    rows: usize,
    blocks: &[HistoryBlock],
    offset: usize,
) -> SnapshotOwned {
    let content: usize = blocks.iter().map(|block| block.lines).sum();
    let total = content.max(rows);
    let offset = offset.min(total - rows);
    let size = TerminalSize {
        cell_width: 8,
        cell_height: 16,
        cols: 36,
        rows: u16::try_from(rows).expect("a test pane has few rows"),
    };

    let mut surface = Surface::new(SurfaceConfig::default(), &size);
    for _ in 1..total {
        surface.line_feed();
    }
    surface.scroll_display(Scroll::Offset(offset));

    let view_start = total - rows - offset;
    let view_end = view_start + rows;
    let mut start = total - content;
    let mut snapshot = surface.snapshot_owned();
    snapshot.blocks = blocks
        .iter()
        .map(|lines| {
            let end = start + lines.lines;
            let visible_start = start.max(view_start);
            let visible_end = end.min(view_end);
            let row = visible_start.min(view_end) - view_start;
            let mut block = BlockSnapshot {
                key: lines.key,
                start_line: row as i32 - offset as i32,
                line_count: visible_end.saturating_sub(visible_start),
                history_line_count: lines.lines,
                has_status_line: lines.has_status_line,
                is_status_line_visible: lines.has_status_line
                    && start >= view_start
                    && start < view_end,
                hidden_line_count: lines.hidden_lines,
                ..BlockSnapshot::default()
            };
            block.meta.kind = if lines.has_status_line {
                BlockKind::Command
            } else {
                BlockKind::Prompt
            };
            start = end;

            block
        })
        .collect();

    snapshot
}

/// Where the blocks of `view` lie in a pane at `layout_pos` of
/// `layout_size` with rows `cell_height` high, as the widget draws them
/// without the step past the top of the history.
pub(crate) fn block_rects_of(
    view: &SnapshotView<'_>,
    layout_pos: Point,
    layout_size: Size,
    cell_height: f32,
) -> Vec<BlockRect> {
    let rows = RowLayout::of_view(view, cell_height, false);

    block_layout::block_rects(&rows, view, layout_pos, layout_size)
}

/// The engine of a terminal `size` big that shows `snapshot`: its program,
/// a mock, takes any request and has nothing newer.
pub(crate) fn engine_showing(
    snapshot: SnapshotOwned,
    size: TerminalSize,
) -> Engine {
    let mut waiting = Some(snapshot);
    let mut program = MockTerminalProgram::new();
    program.expect_send().return_const(());
    program
        .expect_take_snapshot()
        .returning(move || waiting.take());

    let mut engine = Engine::new(Box::new(program), size);
    engine.sync_newest_snapshot();

    engine
}
