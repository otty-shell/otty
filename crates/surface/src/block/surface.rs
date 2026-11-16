//! The block surface: a terminal surface for each block, stitched into one
//! history, and the snapshots of that history.

use std::cmp::{max, min};
use std::sync::Arc;

use super::fold::BlockLines;
use super::{BlockKey, BlockKind, BlockMeta, BlockSnapshot};
use crate::cell::Cell;
use crate::escape::BlockPhase;
use crate::grid::{Grid, Scroll};
use crate::hyperlink::HyperlinkMap;
use crate::index::{Column, Line, Point};
use crate::selection::SelectionRange;
use crate::snapshot::{
    CursorSnapshot, SnapshotCell, SnapshotDamage, SnapshotOwned, SnapshotSize,
    SurfaceModel,
};
use crate::{
    Dimensions, Flags, Surface, SurfaceActor, SurfaceConfig, SurfaceMode,
};

/// In‑memory representation of a single block.
pub(super) struct Block {
    /// The key the block surface gave the block when it made it.
    pub(super) key: BlockKey,
    /// Metadata describing the block's identity and lifecycle.
    pub meta: BlockMeta,
    /// Embedded surface that records the block's terminal contents.
    pub surface: Surface,
    /// Cached textual contents for finished blocks.
    pub cached_text: Option<Arc<str>>,
    /// How many of the first lines of `cached_text` hold the command
    /// lines, counted when that text was made.
    cached_command_rows: usize,
    /// How many lines the block held when its command started (`preexec`):
    /// the prompt and the typed command. A line that wraps over several
    /// rows counts once, so the count stays right when a resize wraps the
    /// lines again. `None` for a block no `preexec` started; a block it
    /// started has a status line, see [`Block::has_status_line`].
    pub(super) command_lines: Option<usize>,
    /// Whether the block is folded: the stitched history shows only its
    /// status line and its command lines.
    pub(super) is_folded: bool,
}

impl Block {
    /// Whether a `preexec` event started the command of the block, so the
    /// stitched history shows an empty status line above its text.
    fn has_status_line(&self) -> bool {
        self.command_lines.is_some()
    }

    /// Lines the status line takes in the stitched history: one or none.
    fn status_line_count(&self) -> usize {
        usize::from(self.has_status_line())
    }

    /// How many of the first lines of the text a snapshot gives for the
    /// block hold its command lines: those of `cached_text` when the block
    /// has it, else those of the grid now, as `lines` tells them.
    fn text_command_rows(&self, lines: &BlockLines) -> usize {
        if self.cached_text.is_some() {
            return self.cached_command_rows;
        }

        lines.command_rows()
    }

    /// Construct a block with its own `Surface` configured for the provided
    /// dimensions so it can capture terminal output independently.
    fn new<D: Dimensions>(
        config: &SurfaceConfig,
        dimensions: &D,
        key: BlockKey,
        meta: BlockMeta,
    ) -> Self {
        Self {
            key,
            meta,
            surface: Surface::new(config.clone(), dimensions),
            cached_text: None,
            cached_command_rows: 0,
            command_lines: None,
            is_folded: false,
        }
    }

    fn update_cached_text(&mut self) {
        if self.meta.kind == BlockKind::Prompt || !self.meta.is_finished {
            return;
        }

        let grid = self.surface.grid();
        let (top_line, total_lines) = BlockSurface::block_visible_extent(self);
        if total_lines == 0 || self.surface.columns() == 0 {
            self.cached_text = None;
            return;
        }

        let columns = self.surface.columns();
        let start = top_line.0;
        let end = start + total_lines as i32;

        let mut lines = Vec::with_capacity(total_lines);
        for line_value in start..end {
            let line = Line(line_value);
            let mut buffer = String::with_capacity(columns);
            for col in 0..columns {
                let column = Column(col);
                let cell = &grid[line][column];
                if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    buffer.push(cell.c);
                }
            }
            lines.push(buffer.trim_end_matches(' ').to_string());
        }

        let text = lines.join("\n");
        let command_rows = self.lines().command_rows();

        if text.is_empty() {
            self.cached_text = None;
        } else {
            self.cached_text = Some(Arc::<str>::from(text));
            self.cached_command_rows = command_rows;
        }
    }
}

/// Aggregate geometry for a block within the concatenated history.
pub(super) struct BlockSliceInfo {
    /// Index into the `blocks` array.
    pub(super) index: usize,
    /// Global history line index marking the start of this block slice:
    /// its status line, when it has one.
    pub(super) start: usize,
    /// Global history line index of the first line of the block's text:
    /// `start`, or the line after it when the block has a status line.
    pub(super) text_start: usize,
    /// Global history line index marking the end of this block slice: just
    /// after the last line of its text that the history shows.
    pub(super) end: usize,
    /// First visible line inside the block's own coordinate space.
    top_line: Line,
}

pub(super) struct ViewportContext {
    /// Total number of lines contributed by all blocks.
    content_lines: usize,
    /// Global line where the viewport begins.
    pub(super) start: usize,
    /// Global line immediately after the viewport.
    viewport_end: usize,
    /// Extra blank rows appended below content when scrolled to bottom.
    bottom_padding: usize,
    /// Effective viewport start taking bottom padding into account.
    effective_start: isize,
}

#[derive(Clone, Copy)]
struct GlobalPoint {
    /// Global line index within the stitched history.
    line_index: usize,
    /// Column position relative to the viewport.
    column: Column,
}

#[derive(Clone, Copy)]
struct GlobalSelection {
    /// Anchor of the selection in global coordinates.
    start: GlobalPoint,
    /// Active end of the selection in global coordinates.
    end: GlobalPoint,
}

/// Helper dimensions type used when creating new block surfaces.
struct BlockDimensions {
    /// Column count shared by all block surfaces.
    columns: usize,
    /// Visible viewport line count shared by all block surfaces.
    screen_lines: usize,
}

impl Dimensions for BlockDimensions {
    /// Total available lines always matches the configured screen lines.
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    /// Screen lines (visible viewport) for the helper dimensions.
    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    /// Number of columns for the helper dimensions.
    fn columns(&self) -> usize {
        self.columns
    }
}

/// Multi‑grid surface composed of several blocks.
///
/// On this step `BlockSurface` behaves as a thin wrapper around a single
/// [`Surface`]: all operations are delegated to the active block and
/// snapshots are exported only for that block.
pub struct BlockSurface {
    /// Ordered list of terminal blocks and their backing surfaces.
    pub(super) blocks: Vec<Block>,
    /// Upper bound on how many blocks to retain in history.
    max_blocks: usize,
    /// Rendering configuration cloned into new blocks.
    config: SurfaceConfig,
    /// Global scroll offset expressed in lines from the bottom.
    pub(super) display_offset: usize,
    /// Index of the block currently owning the local selection, if any.
    selection_block: Option<usize>,
    /// Fixed anchor used when a selection extends beyond its original block.
    selection_anchor: Option<GlobalPoint>,
    /// Selection that spans multiple blocks in global coordinates.
    global_selection: Option<GlobalSelection>,
    /// The number of the key the next block gets. It only grows, so no two
    /// blocks of the terminal get the same key.
    next_key: u64,
}

impl Dimensions for BlockSurface {
    /// Reuse the active block's column count when reporting overall dimensions.
    fn columns(&self) -> usize {
        self.blocks
            .get(self.last_block_idx())
            .map(|block| block.surface.columns())
            .unwrap_or(0)
    }

    /// Reuse the active block's screen-line count for viewport height.
    fn screen_lines(&self) -> usize {
        self.blocks
            .get(self.last_block_idx())
            .map(|block| block.surface.screen_lines())
            .unwrap_or(0)
    }

    /// Combine history and viewport height to report total scrollable lines.
    fn total_lines(&self) -> usize {
        let viewport = self.screen_lines();
        let block_count = self.blocks.len();
        let history = self
            .blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                Self::block_history_line_count(block, index + 1 < block_count)
            })
            .sum::<usize>();

        max(viewport, history)
    }
}

impl BlockSurface {
    /// Default maximum number of blocks to keep.
    pub const DEFAULT_MAX_BLOCKS: usize = 1000;

    /// Create a new block surface with a single empty block: the start
    /// block, which takes what the shell prints before its first prompt.
    pub fn new<D: Dimensions>(config: SurfaceConfig, dimensions: &D) -> Self {
        let start = BlockMeta {
            kind: BlockKind::Start,
            ..BlockMeta::default()
        };
        let block = Block::new(&config, dimensions, BlockKey::new(0), start);

        Self {
            blocks: vec![block],
            max_blocks: Self::DEFAULT_MAX_BLOCKS,
            config,
            display_offset: 0,
            selection_anchor: None,
            selection_block: None,
            global_selection: None,
            next_key: 1,
        }
    }

    /// Return the index of the block with `key`, if it is still in the
    /// history.
    pub(super) fn block_index(&self, key: BlockKey) -> Option<usize> {
        self.blocks.iter().position(|block| block.key == key)
    }

    /// Return the index of the last block, clamping to zero if no blocks exist.
    fn last_block_idx(&self) -> usize {
        self.blocks.len().saturating_sub(1)
    }

    /// Return a mutable reference to the active block.
    fn active_block_mut(&mut self) -> &mut Block {
        let idx = self.last_block_idx();
        &mut self.blocks[idx]
    }

    /// Calculate and set display offset based on
    /// lines from all blocks and lines from active block
    fn calculate_display_offset(&mut self) {
        let viewport_lines = self.screen_lines();
        let max_offset = self.total_lines().saturating_sub(viewport_lines);
        self.display_offset = min(self.display_offset, max_offset);
    }

    /// Produce a list describing how each block maps into the concatenated
    /// history so snapshots can stitch them into a single viewport.
    pub(super) fn block_slices(&self) -> Vec<BlockSliceInfo> {
        if self.is_alt_screen_active()
            && let Some(block) = self.blocks.get(self.last_block_idx())
        {
            let (top_line, total_lines) = Self::block_visible_extent(block);
            // A full-screen program fills the screen: no status line.
            return vec![BlockSliceInfo {
                index: self.last_block_idx(),
                start: 0,
                text_start: 0,
                end: total_lines,
                top_line,
            }];
        }

        let mut start = 0;
        let mut result = Vec::with_capacity(self.blocks.len());

        for (index, block) in self.blocks.iter().enumerate() {
            let lines = block.lines();
            let has_next_block = index + 1 < self.blocks.len();
            let text_start = start + block.status_line_count();
            let shown_lines =
                block.shown_text_line_count(&lines, has_next_block);
            let slice = BlockSliceInfo {
                index,
                start,
                text_start,
                end: text_start + shown_lines,
                top_line: lines.top_line(),
            };
            start = slice.end;
            result.push(slice);
        }

        result
    }

    /// Determine whether the active block is currently in alt-screen mode.
    fn is_alt_screen_active(&self) -> bool {
        self.blocks
            .get(self.last_block_idx())
            .map(|block| block.surface.mode().contains(SurfaceMode::ALT_SCREEN))
            .unwrap_or(false)
    }

    /// Compute viewport geometry (start/end offsets and padding) for a set of
    /// block slices.
    pub(super) fn viewport_context(
        &self,
        slices: &[BlockSliceInfo],
    ) -> ViewportContext {
        let viewport_lines = self.screen_lines();
        let content_lines = slices.last().map(|slice| slice.end).unwrap_or(0);
        let total_lines = max(viewport_lines, content_lines);
        let start =
            total_lines.saturating_sub(viewport_lines + self.display_offset);
        let viewport_end = start + viewport_lines;
        let bottom_padding = if self.display_offset == 0 {
            viewport_lines.saturating_sub(content_lines)
        } else {
            0
        };
        let effective_start = start as isize - bottom_padding as isize;

        ViewportContext {
            content_lines,
            start,
            viewport_end,
            bottom_padding,
            effective_start,
        }
    }

    /// Convert a point relative to a block slice into a global index within the
    /// concatenated history.
    fn global_index_for_point(
        slice: &BlockSliceInfo,
        point: Point,
    ) -> Option<usize> {
        let offset = point.line.0 - slice.top_line.0;
        if offset < 0 {
            return None;
        }

        let offset = offset as usize;
        let text_lines = slice.end.saturating_sub(slice.text_start);
        if offset >= text_lines {
            return None;
        }

        Some(slice.text_start + offset)
    }

    /// Translate a point from block coordinates into viewport coordinates
    /// taking the current scroll offset into account.
    fn convert_point_to_view(
        &self,
        slice: &BlockSliceInfo,
        point: Point,
        viewport_start: usize,
    ) -> Option<Point> {
        let global_index = Self::global_index_for_point(slice, point)?;
        let row_offset = global_index as isize - viewport_start as isize;
        let line_value = row_offset as i32 - self.display_offset as i32;

        Some(Point::new(Line(line_value), point.column))
    }

    /// Translate a selection range from block coordinates into viewport
    /// coordinates if the slice intersects the viewport.
    fn convert_selection_to_view(
        &self,
        range: SelectionRange,
        slice: &BlockSliceInfo,
        viewport_start: usize,
    ) -> Option<SelectionRange> {
        let start =
            self.convert_point_to_view(slice, range.start, viewport_start)?;
        let end =
            self.convert_point_to_view(slice, range.end, viewport_start)?;
        Some(SelectionRange::new(start, end, range.is_block))
    }

    /// Ensure the number of stored blocks does not exceed `max_blocks`.
    fn enforce_max_blocks(&mut self) {
        if self.blocks.len() <= self.max_blocks {
            return;
        }

        let mut index = 0;
        while self.blocks.len() > self.max_blocks
            && index != self.last_block_idx()
        {
            if self.blocks[index].meta.is_finished {
                self.remove_block_at(index);
            } else {
                index += 1;
            }
        }
    }

    /// Remove a block from the history and adjust selection state to match.
    fn remove_block_at(&mut self, index: usize) {
        if index >= self.blocks.len() {
            return;
        }

        self.blocks.remove(index);

        if let Some(selection_index) = self.selection_block {
            if selection_index == index {
                self.selection_block = None;
                self.selection_anchor = None;
                self.global_selection = None;
            } else if selection_index > index {
                self.selection_block = Some(selection_index - 1);
            }
        }

        self.calculate_display_offset();
    }

    /// Terminates the current block at `now_ms` (if it's still running),
    /// creates a new block with a new `Surface`, and makes it active.
    fn begin_block(&mut self, meta: BlockMeta, now_ms: i64) {
        // Mark the active block as complete if it is not already marked.
        let idx = self.last_block_idx();
        if let Some(active) = self.blocks.get_mut(idx) {
            if !active.meta.is_finished {
                active.meta.is_finished = true;
                active.meta.finished_at = Some(now_ms);
            }
            active.update_cached_text();
        }

        // A new block is created with the current surface dimensions.
        let size = {
            let surface = &self.blocks[self.last_block_idx()].surface;
            BlockDimensions {
                columns: surface.columns(),
                screen_lines: surface.screen_lines(),
            }
        };

        let key = BlockKey::new(self.next_key);
        self.next_key += 1;
        self.blocks.push(Block::new(&self.config, &size, key, meta));

        self.enforce_max_blocks();
        self.calculate_display_offset();
    }

    /// Return the index of the currently active unfinished prompt block, if any.
    fn active_prompt_index(&self) -> Option<usize> {
        let idx = self.last_block_idx();
        self.blocks.get(idx).and_then(|block| {
            if block.meta.kind == BlockKind::Prompt && !block.meta.is_finished {
                Some(idx)
            } else {
                None
            }
        })
    }

    /// Marks the running block with the given `id` as finished at `now_ms`
    /// with the given exit code. The block keeps everything else it knows:
    /// its command, folder and start time.
    ///
    /// Nothing changes when no running block has this id. This is the one
    /// place the id of the shell names a block: everywhere else a block is
    /// named by its [`BlockKey`]. Ids can repeat (a shell started inside
    /// another one counts from 1 again), so the latest running block with
    /// the id is taken and finished blocks are left alone. The shell
    /// integration puts the process id of the shell into its ids, so the
    /// `exit` of an outer shell never matches a block of an inner one.
    pub fn end_block_by_id(
        &mut self,
        id: &str,
        exit_code: Option<i32>,
        now_ms: i64,
    ) {
        let running = self
            .blocks
            .iter_mut()
            .rev()
            .find(|block| block.meta.id == id && !block.meta.is_finished);

        if let Some(block) = running {
            block.meta.exit_code = exit_code;
            block.meta.finished_at = Some(now_ms);
            block.meta.is_finished = true;
            block.update_cached_text();
        }

        self.enforce_max_blocks();
    }

    /// Generate a snapshot for the active block when it occupies the alt screen.
    fn snapshot_active_alt_screen_block(&mut self) -> SnapshotOwned {
        self.display_offset = 0;
        let idx = self.last_block_idx();
        let Some(block) = self.blocks.get_mut(idx) else {
            return SnapshotOwned::default();
        };

        let mut snapshot = SnapshotOwned::from_surface(&mut block.surface);
        let line_count = snapshot.view().size.screen_lines;
        snapshot.blocks = vec![BlockSnapshot {
            key: block.key,
            meta: block.meta.clone(),
            start_line: 0,
            line_count,
            cached_text: block.cached_text.clone(),
            // A full-screen program fills the screen: no status line, and
            // nothing to fold.
            has_status_line: false,
            is_status_line_visible: false,
            is_foldable: false,
            hidden_line_count: None,
            command_line_count: 0,
            text_lines_above: 0,
            // A full-screen program has no history: only its screen.
            history_line_count: line_count,
        }];

        snapshot
    }

    /// Return the number of lines a block takes in the stitched history:
    /// its status line, if any, and the lines of its text the history
    /// shows (see `Block::shown_text_line_count`). `has_next_block` tells
    /// whether another block follows it.
    pub(super) fn block_history_line_count(
        block: &Block,
        has_next_block: bool,
    ) -> usize {
        let lines = block.lines();

        block.status_line_count()
            + block.shown_text_line_count(&lines, has_next_block)
    }

    /// Clear the selection, in one block or across blocks.
    pub(super) fn clear_selection(&mut self) {
        if let Some(index) = self.selection_block {
            self.clear_block_selection(index);
        }

        self.selection_block = None;
        self.selection_anchor = None;
        self.global_selection = None;
    }

    /// Calculate the top-most visible line and total visible line count for a
    /// block by trimming empty viewport rows.
    pub(super) fn block_visible_extent(block: &Block) -> (Line, usize) {
        let grid = block.surface.grid();
        let history_lines = grid.history_size();
        let screen_lines = grid.screen_lines();
        let (viewport_head, viewport_tail) =
            Self::viewport_content_bounds(grid);
        let trim_head = if history_lines == 0 { viewport_head } else { 0 };
        let trim_tail = screen_lines.saturating_sub(viewport_tail);
        let visible_viewport =
            screen_lines.saturating_sub(trim_head + trim_tail);
        let total_lines = history_lines + visible_viewport;
        let top_line = grid.topmost_line() + trim_head;

        (top_line, total_lines)
    }

    /// Resolve a viewport-relative point into the owning block together with
    /// its local coordinates and global history index.
    pub(super) fn resolve_block_point_with(
        &self,
        slices: &[BlockSliceInfo],
        context: &ViewportContext,
        point: Point,
    ) -> Option<(usize, Point, usize)> {
        if slices.is_empty() {
            return None;
        }

        if context.content_lines == 0 {
            let idx = self.last_block_idx();
            let block = &self.blocks[idx];
            let column = min(
                point.column,
                Column(block.surface.columns().saturating_sub(1)),
            );
            return Some((idx, Point::new(Line(0), column), 0));
        }

        let viewport_line = point.line.0 + self.display_offset as i32;
        let mut global_index = context.effective_start + viewport_line as isize;
        global_index = global_index
            .clamp(0, (context.content_lines.saturating_sub(1)) as isize);
        let global_index = global_index as usize;

        let slice = slices.iter().find(|slice| {
            global_index >= slice.start && global_index < slice.end
        })?;
        let block = &self.blocks[slice.index];

        // The status line holds no text: a point on it belongs to the block
        // and stands for the start of its first line.
        if global_index < slice.text_start {
            return Some((
                slice.index,
                Point::new(slice.top_line, Column(0)),
                global_index,
            ));
        }

        let local_index = global_index - slice.text_start;
        let block_line = slice.top_line + local_index;
        let last_column = Column(block.surface.columns().saturating_sub(1));
        let column = min(point.column, last_column);

        Some((slice.index, Point::new(block_line, column), global_index))
    }

    /// Push a blank row of cells (offset by scroll) into a snapshot.
    fn push_blank_row(
        cells: &mut Vec<SnapshotCell>,
        columns: usize,
        row: usize,
        display_offset: usize,
    ) {
        let blank_cell = Cell::default();
        let line = Line(row as i32 - display_offset as i32);
        for col in 0..columns {
            let column = Column(col);
            cells.push(SnapshotCell {
                point: Point::new(line, column),
                cell: blank_cell.clone(),
            });
        }
    }

    /// Determine the first/last meaningful viewport rows (non-empty or cursor
    /// row) to trim empty top/bottom padding.
    fn viewport_content_bounds(grid: &Grid<Cell>) -> (usize, usize) {
        let screen_lines = grid.screen_lines();
        let mut first_non_empty = None;
        let mut last_non_empty = None;
        let cursor_line = grid.cursor.point.line;

        for row_idx in 0..screen_lines {
            let line = Line(row_idx as i32);
            let row = &grid[line];
            let is_cursor_row = cursor_line == line;
            if is_cursor_row || !row.is_clear() {
                if first_non_empty.is_none() {
                    first_non_empty = Some(row_idx);
                }
                last_non_empty = Some(row_idx + 1);
            }
        }

        match (first_non_empty, last_non_empty) {
            (Some(start), Some(end)) => (start, end),
            _ => (screen_lines, screen_lines),
        }
    }

    /// Convert a global history line + column into a viewport point while
    /// respecting scroll and bounds.
    fn global_to_view_point(
        &self,
        context: &ViewportContext,
        global_line: usize,
        column: Column,
        viewport_lines: usize,
    ) -> Option<Point> {
        if context.viewport_end <= context.start || viewport_lines == 0 {
            return None;
        }

        let viewport_span = context.viewport_end - context.start;
        if viewport_span == 0 {
            return None;
        }

        let mut relative_line = global_line as isize - context.start as isize;
        let mut clamped = false;

        if relative_line < 0 {
            relative_line = 0;
            clamped = true;
        } else if relative_line as usize >= viewport_span {
            relative_line = viewport_span as isize - 1;
            clamped = true;
        }

        if relative_line < 0 {
            return None;
        }

        let line = relative_line as i32 - self.display_offset as i32;
        let columns = self.columns();
        let max_col = columns.saturating_sub(1);
        let col = if clamped {
            if global_line < context.start {
                Column(0)
            } else {
                Column(max_col)
            }
        } else {
            column
        };

        Some(Point::new(Line(line), col))
    }

    /// Convert a global selection into a viewport-relative range so the UI can
    /// render drag selections spanning multiple blocks.
    fn global_selection_to_view(
        &self,
        selection: &GlobalSelection,
        context: &ViewportContext,
        viewport_lines: usize,
    ) -> Option<SelectionRange> {
        let (start, end) =
            if selection.start.line_index <= selection.end.line_index {
                (selection.start, selection.end)
            } else {
                (selection.end, selection.start)
            };

        let start_point = self.global_to_view_point(
            context,
            start.line_index,
            start.column,
            viewport_lines,
        )?;
        let end_point = self.global_to_view_point(
            context,
            end.line_index,
            end.column,
            viewport_lines,
        )?;
        let (start_point, end_point) = if start_point <= end_point {
            (start_point, end_point)
        } else {
            (end_point, start_point)
        };

        Some(SelectionRange::new(start_point, end_point, false))
    }

    /// Remove any existing selection from the block at the provided index.
    fn clear_block_selection(&mut self, index: usize) {
        if let Some(block) = self.blocks.get_mut(index) {
            block.surface.selection = None;
        }
    }

    /// Update the per-block selection if the drag remains inside a single block.
    ///
    /// Returns `true` when the selection update was handled locally, signalling
    /// that no global multi-block selection should be created.
    fn handle_local_selection(
        &mut self,
        resolved: Option<&(usize, Point, usize)>,
        side: crate::Side,
    ) -> bool {
        let Some(block_index) = self.selection_block else {
            return false;
        };

        let Some(&(index, local_point, global_index)) = resolved else {
            return true;
        };

        if self.global_selection.is_some() {
            return false;
        }

        if index == block_index {
            if let Some(block) = self.blocks.get_mut(index) {
                block.surface.update_selection(local_point, side);
            }
            return true;
        }

        self.clear_block_selection(block_index);
        self.selection_block = None;

        if let Some(anchor) = self.selection_anchor {
            self.global_selection = Some(GlobalSelection {
                start: anchor,
                end: GlobalPoint {
                    line_index: global_index,
                    column: local_point.column,
                },
            });
        }

        true
    }
}

impl SurfaceActor for BlockSurface {
    /// Write a printable character into the active block surface.
    fn print(&mut self, c: char) {
        self.active_block_mut().surface.print(c);
    }

    /// Resize every block surface so they stay aligned when the viewport changes.
    fn resize<S: Dimensions>(&mut self, size: S) {
        let columns = size.columns();
        let screen_lines = size.screen_lines();

        for block in &mut self.blocks {
            block.surface.resize(BlockDimensions {
                columns,
                screen_lines,
            });
        }

        self.calculate_display_offset();
    }

    /// Insert blank cells at the cursor within the active block.
    fn insert_blank(&mut self, count: usize) {
        self.active_block_mut().surface.insert_blank(count);
    }

    /// Insert blank lines into the active block, pushing existing content down.
    fn insert_blank_lines(&mut self, count: usize) {
        self.active_block_mut().surface.insert_blank_lines(count);
    }

    /// Delete lines from the active block starting at the cursor row.
    fn delete_lines(&mut self, count: usize) {
        self.active_block_mut().surface.delete_lines(count);
    }

    /// Delete characters from the active block starting at the cursor column.
    fn delete_chars(&mut self, count: usize) {
        self.active_block_mut().surface.delete_chars(count);
    }

    /// Erase characters (replacing with blanks) within the active block.
    fn erase_chars(&mut self, count: usize) {
        self.active_block_mut().surface.erase_chars(count);
    }

    /// Backspace within the active block surface.
    fn backspace(&mut self) {
        self.active_block_mut().surface.backspace();
    }

    /// Move the cursor to the beginning of the current line in the active block.
    fn carriage_return(&mut self) {
        self.active_block_mut().surface.carriage_return();
    }

    /// Perform a line feed within the active block surface.
    fn line_feed(&mut self) {
        self.active_block_mut().surface.line_feed();
    }

    /// Insert a newline sequence, combining carriage return and line feed.
    fn new_line(&mut self) {
        self.active_block_mut().surface.new_line();
    }

    /// Record a horizontal tab stop within the active block.
    fn set_horizontal_tab(&mut self) {
        self.active_block_mut().surface.set_horizontal_tab();
    }

    /// Scroll the active block up when the cursor moves past the top.
    fn reverse_index(&mut self) {
        self.active_block_mut().surface.reverse_index();
    }

    /// Reset the active block to its initial state.
    fn reset(&mut self) {
        self.active_block_mut().surface.reset();
    }

    /// Clear the active block screen according to the requested mode.
    fn clear_screen(&mut self, mode: crate::escape::ClearMode) {
        self.active_block_mut().surface.clear_screen(mode);
    }

    /// Clear part of the current line in the active block.
    fn clear_line(&mut self, mode: crate::escape::LineClearMode) {
        self.active_block_mut().surface.clear_line(mode);
    }

    /// Insert blank tab fields relative to the active cursor.
    fn insert_tabs(&mut self, count: usize) {
        self.active_block_mut().surface.insert_tabs(count);
    }

    /// Clear tab stops in the active block surface.
    fn clear_tabs(&mut self, mode: crate::escape::TabClearMode) {
        self.active_block_mut().surface.clear_tabs(mode);
    }

    /// Fill the active screen with test characters for alignment checks.
    fn screen_alignment_display(&mut self) {
        self.active_block_mut().surface.screen_alignment_display();
    }

    /// Move forward across tab stops within the active block.
    fn move_forward_tabs(&mut self, count: usize) {
        self.active_block_mut().surface.move_forward_tabs(count);
    }

    /// Move backward across tab stops within the active block.
    fn move_backward_tabs(&mut self, count: usize) {
        self.active_block_mut().surface.move_backward_tabs(count);
    }

    /// Select which charset slot subsequent escape codes target.
    fn set_active_charset_index(&mut self, index: crate::escape::CharsetIndex) {
        self.active_block_mut()
            .surface
            .set_active_charset_index(index);
    }

    /// Configure a charset mapping for the given slot on the active surface.
    fn configure_charset(
        &mut self,
        charset: crate::escape::Charset,
        index: crate::escape::CharsetIndex,
    ) {
        self.active_block_mut()
            .surface
            .configure_charset(charset, index);
    }

    /// Override one of the dynamic palette entries for the active block.
    fn set_color(&mut self, index: usize, color: crate::escape::Rgb) {
        self.active_block_mut().surface.set_color(index, color);
    }

    /// Query a palette entry, routing the response through the active surface.
    fn query_color(&mut self, index: usize) {
        self.active_block_mut().surface.query_color(index);
    }

    /// Reset a palette entry back to its default value.
    fn reset_color(&mut self, index: usize) {
        self.active_block_mut().surface.reset_color(index);
    }

    /// Set the DEC scrolling region on the active surface.
    fn set_scrolling_region(&mut self, top: usize, bottom: usize) {
        self.active_block_mut()
            .surface
            .set_scrolling_region(top, bottom);
    }

    /// Scroll the active surface content up by the given count.
    fn scroll_up(&mut self, count: usize) {
        self.active_block_mut().surface.scroll_up(count);
    }

    /// Scroll the active surface content down by the given count.
    fn scroll_down(&mut self, count: usize) {
        self.active_block_mut().surface.scroll_down(count);
    }

    /// Scroll the aggregated block history (rather than a single surface).
    fn scroll_display(&mut self, scroll: Scroll) {
        let viewport = self.screen_lines();
        if viewport == 0 {
            return;
        }

        let max_offset = self.total_lines().saturating_sub(viewport);

        self.display_offset = match scroll {
            Scroll::Delta(delta) => {
                let current = self.display_offset as i32;
                let next = (current + delta).clamp(0, max_offset as i32);
                next as usize
            },
            Scroll::PageUp => min(self.display_offset + viewport, max_offset),
            Scroll::PageDown => self.display_offset.saturating_sub(viewport),
            Scroll::Top => max_offset,
            Scroll::Bottom => 0,
            Scroll::Offset(offset) => min(offset, max_offset),
        };
    }

    /// Apply or clear the active hyperlink for subsequently printed cells.
    fn set_hyperlink(&mut self, link: Option<crate::escape::Hyperlink>) {
        self.active_block_mut().surface.set_hyperlink(link);
    }

    /// Apply an SGR attribute to the active surface.
    fn sgr(&mut self, attr: crate::escape::CharacterAttribute) {
        self.active_block_mut().surface.sgr(attr);
    }

    /// Change the cursor shape for the active block.
    fn set_cursor_shape(&mut self, shape: crate::escape::CursorShape) {
        self.active_block_mut().surface.set_cursor_shape(shape);
    }

    /// Change the cursor style (blinking/steady) for the active block.
    fn set_cursor_style(&mut self, style: Option<crate::escape::CursorStyle>) {
        self.active_block_mut().surface.set_cursor_style(style);
    }

    /// Save the current cursor state for later restoration.
    fn save_cursor(&mut self) {
        self.active_block_mut().surface.save_cursor();
    }

    /// Restore the previously saved cursor state.
    fn restore_cursor(&mut self) {
        self.active_block_mut().surface.restore_cursor();
    }

    /// Move the cursor up within the active block.
    fn move_up(&mut self, rows: usize, carriage_return: bool) {
        self.active_block_mut()
            .surface
            .move_up(rows, carriage_return);
    }

    /// Move the cursor down within the active block.
    fn move_down(&mut self, rows: usize, carriage_return: bool) {
        self.active_block_mut()
            .surface
            .move_down(rows, carriage_return);
    }

    /// Move the cursor forward horizontally.
    fn move_forward(&mut self, cols: usize) {
        self.active_block_mut().surface.move_forward(cols);
    }

    /// Move the cursor backward horizontally.
    fn move_backward(&mut self, cols: usize) {
        self.active_block_mut().surface.move_backward(cols);
    }

    /// Jump the cursor to an absolute row/column in the active block.
    fn goto(&mut self, row: i32, col: usize) {
        self.active_block_mut().surface.goto(row, col);
    }

    /// Jump the cursor to an absolute row.
    fn goto_row(&mut self, row: i32) {
        self.active_block_mut().surface.goto_row(row);
    }

    /// Jump the cursor to an absolute column.
    fn goto_column(&mut self, col: usize) {
        self.active_block_mut().surface.goto_column(col);
    }

    /// Toggle keypad application mode on the active surface.
    fn set_keypad_application_mode(&mut self, enabled: bool) {
        self.active_block_mut()
            .surface
            .set_keypad_application_mode(enabled);
    }

    /// Apply a keyboard mode change and optionally report it.
    fn set_keyboard_mode(
        &mut self,
        mode: crate::escape::KeyboardMode,
        behavior: crate::escape::KeyboardModeApplyBehavior,
    ) {
        self.active_block_mut()
            .surface
            .set_keyboard_mode(mode, behavior);
    }

    /// Push a keyboard mode onto the active surface stack.
    fn push_keyboard_mode(&mut self, mode: crate::escape::KeyboardMode) {
        self.active_block_mut().surface.push_keyboard_mode(mode);
    }

    /// Pop keyboard modes from the active surface stack.
    fn pop_keyboard_modes(&mut self, count: u16) {
        self.active_block_mut().surface.pop_keyboard_modes(count);
    }

    /// Report the current keyboard mode through the provided channel.
    fn report_keyboard_mode(
        &mut self,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_keyboard_mode(report_channel);
    }

    /// Save the current window title on a stack.
    fn push_window_title(&mut self) {
        self.active_block_mut().surface.push_window_title();
    }

    /// Restore the most recently saved window title if available.
    fn pop_window_title(&mut self) -> Option<String> {
        self.active_block_mut().surface.pop_window_title()
    }

    /// Set the current window title.
    fn set_window_title(&mut self, title: Option<String>) {
        self.active_block_mut().surface.set_window_title(title);
    }

    /// Toggle 80/132-column mode on the active surface.
    fn deccolm(&mut self) {
        self.active_block_mut().surface.deccolm();
    }

    /// Enable a DEC private mode bit on the active surface.
    fn set_private_mode(&mut self, mode: crate::escape::PrivateMode) {
        self.active_block_mut().surface.set_private_mode(mode);
    }

    /// Disable a DEC private mode bit on the active surface.
    fn unset_private_mode(&mut self, mode: crate::escape::PrivateMode) {
        self.active_block_mut().surface.unset_private_mode(mode);
    }

    /// Report a DEC private mode through the response channel.
    fn report_private_mode(
        &mut self,
        mode: crate::escape::PrivateMode,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_private_mode(mode, report_channel);
    }

    /// Enable a standard terminal mode.
    fn set_mode(&mut self, mode: crate::escape::Mode) {
        self.active_block_mut().surface.set_mode(mode);
    }

    /// Disable a standard terminal mode.
    fn unset_mode(&mut self, mode: crate::escape::Mode) {
        self.active_block_mut().surface.unset_mode(mode);
    }

    /// Report a standard terminal mode to the host.
    fn report_mode(
        &mut self,
        mode: crate::escape::Mode,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_mode(mode, report_channel);
    }

    /// Respond to terminal identification queries.
    fn identify_terminal(
        &mut self,
        attr: Option<char>,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .identify_terminal(attr, report_channel);
    }

    /// Report device status queries via the active surface.
    fn report_device_status(
        &mut self,
        status: usize,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_device_status(status, report_channel);
    }

    /// Report the text area dimensions in pixels.
    fn request_text_area_by_pixels(
        &mut self,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .request_text_area_by_pixels(report_channel);
    }

    /// Report the text area dimensions in characters.
    fn request_text_area_by_chars(
        &mut self,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .request_text_area_by_chars(report_channel);
    }

    /// Begin a local selection tied to the block under the cursor.
    fn start_selection(
        &mut self,
        ty: crate::SelectionType,
        point: crate::index::Point,
        side: crate::Side,
    ) {
        self.global_selection = None;
        let slices = self.block_slices();
        if slices.is_empty() {
            self.selection_block = None;
            self.selection_anchor = None;
            return;
        }

        let context = self.viewport_context(&slices);
        if let Some((index, local_point, global_index)) =
            self.resolve_block_point_with(&slices, &context, point)
        {
            self.selection_block = Some(index);
            self.selection_anchor = Some(GlobalPoint {
                line_index: global_index,
                column: local_point.column,
            });
            if let Some(block) = self.blocks.get_mut(index) {
                block.surface.start_selection(ty, local_point, side);
            }
        } else {
            self.selection_block = None;
            self.selection_anchor = None;
        }
    }

    /// Extend or convert the active selection as the cursor moves.
    fn update_selection(
        &mut self,
        point: crate::index::Point,
        side: crate::Side,
    ) {
        let slices = self.block_slices();
        if slices.is_empty() {
            return;
        }

        let context = self.viewport_context(&slices);
        let resolved = self.resolve_block_point_with(&slices, &context, point);

        if self.handle_local_selection(resolved.as_ref(), side) {
            return;
        }

        if let (Some((_, local_point, global_index)), Some(anchor)) =
            (resolved, self.selection_anchor)
        {
            self.global_selection = Some(GlobalSelection {
                start: anchor,
                end: GlobalPoint {
                    line_index: global_index,
                    column: local_point.column,
                },
            });
        }
    }

    /// Fold the block with `key`, or unfold it when it is folded. Folding
    /// lives in the `fold` module next to this file.
    fn toggle_block_fold(&mut self, key: BlockKey) {
        self.toggle_fold(key);
    }

    /// React to prompt/command lifecycle events emitted by the parser.
    ///
    /// `now_ms` is the time of the event by the caller's clock; it becomes
    /// the start or the end time of the block the event is about.
    fn handle_block_event(
        &mut self,
        event: crate::escape::BlockEvent,
        now_ms: i64,
    ) {
        let escape_meta = event.meta;
        let mut meta = BlockMeta::from(escape_meta);

        // BlockPhase mirrors the shell lifecycle:
        // - `Precmd` fires right before the prompt is drawn, so we ensure the
        //   last block is a prompt pinned to the bottom.
        // - `Preexec` signals that the user just submitted that prompt; we flip
        //   the same block into `Command` so stdout appears directly beneath the
        //   initiating prompt.
        // - `Exit` adds the exit code and the end time to the running
        //   command and marks it finished.
        match event.phase {
            // Command start hook
            BlockPhase::Preexec => {
                // Reuse the active prompt block so the pending command produces
                // output directly under the prompt instead of opening a new block.
                // The command gets its status line above its text.
                meta.kind = BlockKind::Command;
                meta.started_at = Some(now_ms);
                if let Some(prompt_idx) = self.active_prompt_index() {
                    let block = &mut self.blocks[prompt_idx];
                    block.meta = meta;
                    block.remember_command_lines();
                    self.calculate_display_offset();
                } else {
                    self.begin_block(meta, now_ms);
                    self.active_block_mut().remember_command_lines();
                }
            },
            BlockPhase::Exit => {
                self.end_block_by_id(&meta.id, meta.exit_code, now_ms);
            },
            // Prompt block start hook
            BlockPhase::Precmd => {
                meta.kind = BlockKind::Prompt;

                if self.blocks.iter().any(|b| b.meta.id == meta.id) {
                    self.end_block_by_id(&meta.id, None, now_ms);
                    return;
                }

                if let Some(index) = self.active_prompt_index()
                    && self.blocks.len() > 1
                {
                    self.remove_block_at(index);
                }

                self.begin_block(meta, now_ms);
            },
        }
    }
}

impl SurfaceModel for BlockSurface {
    /// Build a full snapshot of all blocks merged into a single viewport.
    fn snapshot_owned(&mut self) -> SnapshotOwned {
        if self.blocks.is_empty() {
            return SnapshotOwned::default();
        }

        if self.is_alt_screen_active() {
            return self.snapshot_active_alt_screen_block();
        }

        self.calculate_display_offset();

        let columns = self.columns();
        let viewport_lines = self.screen_lines();
        let slices = self.block_slices();
        let context = self.viewport_context(&slices);
        let start = context.start;
        let viewport_end = context.viewport_end;

        let mut cells =
            Vec::with_capacity(columns.saturating_mul(viewport_lines));
        let mut added_lines = Vec::new();
        let bottom_padding = context.bottom_padding;
        let effective_start = context.effective_start;

        let mut slice_idx = 0;
        let mut current_slice = slices.get(slice_idx);
        for row in 0..viewport_lines {
            let global_index = effective_start + row as isize;
            if global_index < 0
                || global_index as usize >= context.content_lines
                || current_slice.is_none()
            {
                Self::push_blank_row(
                    &mut cells,
                    columns,
                    row,
                    self.display_offset,
                );
                continue;
            }

            let global_index = global_index as usize;
            while let Some(slice) = current_slice {
                if global_index < slice.end {
                    break;
                }
                slice_idx += 1;
                current_slice = slices.get(slice_idx);
            }

            let Some(slice) = current_slice else {
                Self::push_blank_row(
                    &mut cells,
                    columns,
                    row,
                    self.display_offset,
                );
                continue;
            };

            // The status line is a blank row of the block. It is no
            // terminal text.
            if global_index < slice.text_start {
                added_lines.push(Line(row as i32 - self.display_offset as i32));
                Self::push_blank_row(
                    &mut cells,
                    columns,
                    row,
                    self.display_offset,
                );
                continue;
            }

            let block_index = slice.index;
            let block = &self.blocks[block_index];
            let grid = block.surface.grid();
            let local_index = global_index - slice.text_start;
            let line = slice.top_line + local_index;

            for col in 0..columns {
                let column = Column(col);
                let cell = grid[line][column].clone();
                let point_line = row as i32 - self.display_offset as i32;
                let point = Point::new(Line(point_line), column);
                cells.push(SnapshotCell { point, cell });
            }
        }

        let idx = self.last_block_idx();
        let active_block = &self.blocks[idx].surface;
        let mut cursor = CursorSnapshot::new(active_block);
        let active_slice = slices.iter().find(|slice| slice.index == idx);
        if let Some(slice) = active_slice
            && let Some(point) =
                self.convert_point_to_view(slice, cursor.point, start)
        {
            cursor.point = point;
        }

        let mut selection = None;
        if let Some(index) =
            self.selection_block.filter(|&idx| idx < self.blocks.len())
            && let Some(slice) =
                slices.iter().find(|slice| slice.index == index)
            && let Some(range) = self.blocks[index]
                .surface
                .selection
                .as_ref()
                .and_then(|s| s.to_range(&self.blocks[index].surface))
        {
            selection = self.convert_selection_to_view(range, slice, start);
        }

        let size = SnapshotSize {
            columns,
            screen_lines: viewport_lines,
            total_lines: self.total_lines(),
        };
        let visible_cell_count = size.columns * size.screen_lines;

        let hyperlinks = HyperlinkMap::build(&cells, size, self.display_offset);

        if selection.is_none()
            && let Some(global_selection) = &self.global_selection
        {
            selection = self.global_selection_to_view(
                global_selection,
                &context,
                viewport_lines,
            );
        }

        let padding_adjustment = bottom_padding as i32;
        if padding_adjustment > 0 {
            cursor.point.line += padding_adjustment;
            if let Some(range) = selection.as_mut() {
                range.start.line += padding_adjustment;
                range.end.line += padding_adjustment;
            }
        }

        let mut block_snapshots = Vec::with_capacity(self.blocks.len());
        for slice in &slices {
            let block = &self.blocks[slice.index];
            let block_start = slice.start;
            let block_end = slice.end;
            let visible_start = max(block_start, start);
            let visible_end = min(block_end, viewport_end);
            let line_count = visible_end.saturating_sub(visible_start);
            let line_offset = visible_start.saturating_sub(start);
            let start_line = line_offset as i32 - self.display_offset as i32
                + padding_adjustment;
            let is_status_line_visible = block.has_status_line()
                && block_start >= start
                && block_start < viewport_end;
            let lines = block.lines();
            let hidden_line_count =
                block.is_folded.then(|| block.output_line_count(&lines));
            // The lines of text from the first one up to the top edge.
            let text_lines_above =
                min(slice.end, start).saturating_sub(slice.text_start);

            block_snapshots.push(BlockSnapshot {
                key: block.key,
                meta: block.meta.clone(),
                start_line,
                line_count,
                cached_text: block.cached_text.clone(),
                has_status_line: block.has_status_line(),
                is_status_line_visible,
                is_foldable: block.is_foldable(&lines),
                hidden_line_count,
                command_line_count: block.text_command_rows(&lines),
                text_lines_above,
                history_line_count: slice.end - slice.start,
            });
        }

        SnapshotOwned::from_parts(
            cells,
            selection,
            hyperlinks,
            cursor,
            self.display_offset,
            *active_block.colors(),
            *active_block.mode(),
            size,
            SnapshotDamage::Full,
            visible_cell_count,
            block_snapshots,
            added_lines,
        )
    }

    /// Propagate damage reset to the active block surface.
    fn reset_damage(&mut self) {
        self.active_block_mut().surface.reset_damage();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::testing::*;
    use crate::snapshot::SurfaceModel;
    use crate::{Line, SurfaceConfig};

    #[test]
    fn block_surface_starts_with_single_block() {
        let dims = TestDimensions::new(4, 2);
        let surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        assert_eq!(surface.blocks.len(), 1);
        assert_eq!(surface.blocks[0].meta.kind, BlockKind::Start);
    }

    #[test]
    fn block_surface_delegates_to_inner_surface() {
        let dims = TestDimensions::new(4, 2);
        let mut block_surface =
            BlockSurface::new(SurfaceConfig::default(), &dims);

        block_surface.print('X');
        let snapshot = block_surface.snapshot_owned();
        let view = snapshot.view();

        assert_eq!(view.size.columns, 4);
        assert_eq!(view.size.screen_lines, 2);
        assert!(view.cells.iter().any(|cell| cell.cell.c == 'X'));
    }

    #[test]
    fn block_surface_begin_block_creates_new_active_block() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        surface.print('a');
        let first_snapshot = surface.snapshot_owned();
        let first_view = first_snapshot.view();
        assert!(first_view.cells.iter().any(|cell| cell.cell.c == 'a'));

        let meta = BlockMeta {
            id: String::from("1"),
            kind: BlockKind::Command,
            cmd: None,
            cwd: None,
            shell: None,
            started_at: Some(1),
            finished_at: None,
            exit_code: None,
            is_finished: false,
        };

        surface.begin_block(meta, 0);
        surface.print('b');

        assert_eq!(surface.blocks.len(), 2);
        assert_eq!(surface.last_block_idx(), 1);

        let second_snapshot = surface.snapshot_owned();
        let second_view = second_snapshot.view();
        assert!(second_view.cells.iter().any(|cell| cell.cell.c == 'b'));
        let blocks = second_view.blocks();
        assert_eq!(blocks.len(), 2);
        assert!(
            blocks.last().is_some_and(|block| block.line_count > 0),
            "new block should contribute its own surface lines",
        );
    }

    #[test]
    fn block_surface_end_block_by_id_marks_block_finished() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        let start_meta = BlockMeta {
            id: String::from("cmd-1"),
            kind: BlockKind::Command,
            cmd: None,
            cwd: None,
            shell: None,
            started_at: Some(10),
            finished_at: None,
            exit_code: None,
            is_finished: false,
        };

        surface.begin_block(start_meta, 10);

        surface.end_block_by_id("cmd-1", Some(0), 20);

        let block = surface
            .blocks
            .iter()
            .find(|b| b.meta.id == "cmd-1")
            .expect("block exists");
        assert_eq!(block.meta.exit_code, Some(0));
        assert_eq!(block.meta.started_at, Some(10));
        assert_eq!(block.meta.finished_at, Some(20));
        assert!(block.meta.is_finished);
    }

    #[test]
    fn block_surface_respects_max_blocks_and_keeps_unfinished() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        surface.max_blocks = 3;

        for i in 0..5 {
            let meta = BlockMeta {
                id: format!("{i}"),
                kind: BlockKind::Command,
                cmd: None,
                cwd: None,
                shell: None,
                started_at: Some(i),
                finished_at: Some(i),
                exit_code: Some(0),
                is_finished: false,
            };
            surface.begin_block(meta, 0);
        }

        assert!(
            surface.blocks.len() <= 3,
            "block history should be bounded by max_blocks",
        );
        assert!(
            surface.blocks.iter().any(|b| !b.meta.is_finished),
            "unfinished blocks should not be deleted when applying max_blocks",
        );
    }

    #[test]
    fn snapshot_includes_multiple_blocks() {
        use crate::grid::Scroll;

        let dims = TestDimensions::new(2, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        for _ in 0..4 {
            surface.print('A');
            surface.new_line();
        }

        let meta = BlockMeta {
            id: String::from("cmd-1"),
            kind: BlockKind::Command,
            cmd: Some(String::from("ls")),
            cwd: Some(String::from("/tmp")),
            shell: Some(String::from("bash")),
            started_at: Some(1),
            finished_at: None,
            exit_code: None,
            is_finished: false,
        };
        surface.begin_block(meta, 0);

        for _ in 0..4 {
            surface.print('B');
            surface.new_line();
        }

        let bottom_snapshot = surface.snapshot_owned();
        let bottom_view = bottom_snapshot.view();
        assert!(
            bottom_view.cells.iter().any(|cell| cell.cell.c == 'B'),
            "bottom view should display active block output",
        );

        let blocks = bottom_view.blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].line_count, 0);
        assert_eq!(blocks[1].line_count, bottom_view.size.screen_lines);

        surface.scroll_display(Scroll::Top);
        let top_snapshot = surface.snapshot_owned();
        let top_view = top_snapshot.view();
        assert!(
            top_view.cells.iter().any(|cell| cell.cell.c == 'A'),
            "scrolling to top should reveal the first block",
        );

        let top_blocks = top_view.blocks();
        assert_eq!(top_blocks[0].line_count, top_view.size.screen_lines);
        assert_eq!(top_blocks[1].line_count, 0);
    }

    #[test]
    fn block_snapshot_trims_empty_viewport_rows() {
        use crate::grid::Scroll;

        let dims = TestDimensions::new(2, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        surface.print('A');
        surface.new_line();
        surface.print('A');

        let meta = BlockMeta {
            id: String::from("cmd-1"),
            kind: BlockKind::Command,
            cmd: Some(String::from("echo A")),
            cwd: None,
            shell: None,
            started_at: Some(1),
            finished_at: None,
            exit_code: None,
            is_finished: false,
        };

        surface.begin_block(meta, 0);
        surface.print('B');

        surface.scroll_display(Scroll::Top);
        let top_snapshot = surface.snapshot_owned();
        let top_view = top_snapshot.view();
        let top_blocks = top_view.blocks();
        assert_eq!(top_blocks.len(), 2);
        assert_eq!(top_blocks[0].line_count, 2);
        let first_block_extent = top_blocks[0].line_count;

        surface.scroll_display(Scroll::Bottom);
        let bottom_snapshot = surface.snapshot_owned();
        let bottom_view = bottom_snapshot.view();
        let bottom_blocks = bottom_view.blocks();
        assert_eq!(bottom_blocks.len(), 2);
        assert!(
            bottom_blocks[0].line_count <= first_block_extent,
            "first block should never expand when scrolled to the bottom",
        );
        assert_eq!(bottom_blocks[1].line_count, 1);
        assert_eq!(
            bottom_blocks[1].start_line,
            bottom_view.size.screen_lines as i32 - 1,
        );
    }

    #[test]
    fn prompt_is_pinned_to_bottom_when_not_scrolled() {
        let dims = TestDimensions::new(4, 6);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        surface.print('>');

        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();
        let blocks = view.blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].line_count, 1);
        assert_eq!(blocks[0].start_line, view.size.screen_lines as i32 - 1);
    }

    #[test]
    fn resizing_updates_all_blocks() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        surface.print('A');

        let meta = BlockMeta {
            id: String::from("cmd-1"),
            kind: BlockKind::Command,
            cmd: None,
            cwd: None,
            shell: None,
            started_at: Some(1),
            finished_at: None,
            exit_code: None,
            is_finished: false,
        };

        surface.begin_block(meta, 0);
        surface.print('B');

        surface.resize(TestDimensions::new(6, 4));

        for block in &surface.blocks {
            let grid = block.surface.grid();
            assert_eq!(grid.columns(), 6);
            assert_eq!(grid.screen_lines(), 4);
        }
    }

    #[test]
    fn block_text_includes_offscreen_history_for_finished_blocks() {
        let dims = TestDimensions::new(4, 2);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        let meta_1 = BlockMeta {
            id: String::from("block-1"),
            kind: BlockKind::Command,
            cmd: None,
            cwd: None,
            shell: None,
            started_at: Some(1),
            finished_at: None,
            exit_code: None,
            is_finished: false,
        };
        surface.begin_block(meta_1, 1);
        surface.print('A');
        surface.carriage_return();
        surface.line_feed();
        surface.print('B');

        let meta_2 = BlockMeta {
            id: String::from("block-2"),
            kind: BlockKind::Command,
            cmd: None,
            cwd: None,
            shell: None,
            started_at: Some(2),
            finished_at: None,
            exit_code: None,
            is_finished: false,
        };
        surface.begin_block(meta_2, 2);

        surface.print('C');
        surface.carriage_return();
        surface.line_feed();
        surface.print('D');

        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();
        let blocks = view.blocks();
        let first = blocks
            .iter()
            .find(|b| b.meta.id == "block-1")
            .expect("block-1 snapshot");
        assert_eq!(first.line_count, 0);

        assert_eq!(
            block_text(&snapshot, first.key),
            Some(String::from("A\nB"))
        );
    }

    fn block_metas(surface: &BlockSurface) -> Vec<BlockMeta> {
        surface
            .blocks
            .iter()
            .map(|block| block.meta.clone())
            .collect()
    }

    fn block_meta<'a>(surface: &'a BlockSurface, id: &str) -> &'a BlockMeta {
        surface
            .blocks
            .iter()
            .map(|block| &block.meta)
            .find(|meta| meta.id == id)
            .expect("block with this id exists")
    }

    #[test]
    fn given_running_command_when_exit_arrives_then_block_keeps_command_and_gets_code_and_duration()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        surface.handle_block_event(
            preexec_event("cmd-1", "cargo test", "/home/me/otty"),
            5_000,
        );

        surface.handle_block_event(exit_event("cmd-1", 101), 6_900);

        let meta = block_meta(&surface, "cmd-1");
        assert!(meta.is_finished);
        assert_eq!(meta.kind, BlockKind::Command);
        assert_eq!(meta.cmd.as_deref(), Some("cargo test"));
        assert_eq!(meta.cwd.as_deref(), Some("/home/me/otty"));
        assert_eq!(meta.exit_code, Some(101));
        assert_eq!(meta.started_at, Some(5_000));
        assert_eq!(meta.finished_at, Some(6_900));
    }

    #[test]
    fn given_running_command_when_prompt_returns_without_exit_then_block_is_finished_with_no_code()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        surface.handle_block_event(preexec_event("cmd-1", "ls", "/tmp"), 5_000);

        surface.handle_block_event(precmd_event("prompt-2"), 5_300);

        let meta = block_meta(&surface, "cmd-1");
        assert!(meta.is_finished);
        assert_eq!(meta.cmd.as_deref(), Some("ls"));
        assert_eq!(meta.exit_code, None);
        assert_eq!(meta.started_at, Some(5_000));
        assert_eq!(meta.finished_at, Some(5_300));
    }

    #[test]
    fn given_exit_for_unknown_id_when_handled_then_nothing_changes() {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        surface.handle_block_event(preexec_event("cmd-1", "ls", "/tmp"), 5_000);
        let before = block_metas(&surface);

        surface.handle_block_event(exit_event("cmd-9", 1), 6_000);

        assert_eq!(block_metas(&surface), before);
    }

    #[test]
    fn given_exit_then_prompt_when_handled_then_end_time_of_exit_stays() {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        surface
            .handle_block_event(preexec_event("cmd-1", "false", "/tmp"), 5_000);
        surface.handle_block_event(exit_event("cmd-1", 1), 5_200);

        surface.handle_block_event(precmd_event("prompt-2"), 5_900);

        let meta = block_meta(&surface, "cmd-1");
        assert_eq!(meta.exit_code, Some(1));
        assert_eq!(meta.finished_at, Some(5_200));
        let prompt = block_meta(&surface, "prompt-2");
        assert_eq!(prompt.kind, BlockKind::Prompt);
        assert!(!prompt.is_finished);
    }

    #[test]
    fn given_command_ended_by_exit_when_copied_then_its_text_and_command_are_whole()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        "$ ls".chars().for_each(|c| surface.print(c));
        surface.handle_block_event(preexec_event("cmd-1", "ls", "/tmp"), 5_000);
        surface.carriage_return();
        surface.line_feed();
        "a b".chars().for_each(|c| surface.print(c));

        surface.handle_block_event(exit_event("cmd-1", 0), 5_100);
        surface.handle_block_event(precmd_event("prompt-2"), 5_100);

        let snapshot = surface.snapshot_owned();
        assert_eq!(
            block_text(&snapshot, block_snapshot(&snapshot, "cmd-1").key),
            Some(String::from("$ ls\na b"))
        );
        assert_eq!(
            snapshot
                .view()
                .block_output_text(block_snapshot(&snapshot, "cmd-1").key),
            Some(String::from("a b"))
        );
    }

    #[test]
    fn given_shell_times_in_events_when_handled_then_only_the_passed_time_is_kept()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        surface.handle_block_event(precmd_event("prompt-1"), 1_000);

        let prompt = block_meta(&surface, "prompt-1");
        assert_eq!(prompt.started_at, None);
        assert_eq!(prompt.finished_at, None);

        surface.handle_block_event(preexec_event("cmd-1", "ls", "/tmp"), 5_000);

        let command = block_meta(&surface, "cmd-1");
        assert_eq!(command.started_at, Some(5_000));
        assert_eq!(command.finished_at, None);
    }

    #[test]
    fn given_empty_enter_when_prompt_returns_then_finished_command_is_untouched()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        surface
            .handle_block_event(preexec_event("cmd-1", "true", "/tmp"), 5_000);
        surface.handle_block_event(exit_event("cmd-1", 0), 5_100);
        surface.handle_block_event(precmd_event("prompt-2"), 5_100);
        let before = block_meta(&surface, "cmd-1").clone();

        surface.handle_block_event(precmd_event("prompt-3"), 9_000);

        assert_eq!(block_meta(&surface, "cmd-1"), &before);
        assert_eq!(surface.blocks.len(), 3);
    }

    #[test]
    fn given_two_blocks_with_one_id_when_exit_arrives_then_only_the_running_one_ends()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        surface
            .handle_block_event(preexec_event("cmd-1", "true", "/tmp"), 2_000);
        surface.handle_block_event(exit_event("cmd-1", 0), 2_100);
        // A shell started inside the first one counts its commands from 1
        // again.
        surface.handle_block_event(precmd_event("prompt-2"), 3_000);
        surface
            .handle_block_event(preexec_event("cmd-1", "false", "/tmp"), 4_000);

        surface.handle_block_event(exit_event("cmd-1", 1), 4_500);

        let metas = block_metas(&surface);
        let first = metas
            .iter()
            .find(|meta| meta.cmd.as_deref() == Some("true"))
            .expect("first command block");
        let second = metas
            .iter()
            .find(|meta| meta.cmd.as_deref() == Some("false"))
            .expect("second command block");
        assert_eq!(first.exit_code, Some(0));
        assert_eq!(first.finished_at, Some(2_100));
        assert_eq!(second.exit_code, Some(1));
        assert_eq!(second.finished_at, Some(4_500));
        assert!(second.is_finished);
    }

    #[test]
    fn given_command_started_by_preexec_when_shown_then_an_empty_status_line_is_above_its_text()
     {
        let mut surface = surface_running_ls();

        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert!(block.has_status_line);
        assert!(block.is_status_line_visible);
        assert_eq!(block.line_count, 2);
        assert_eq!(line_text(&snapshot, block.start_line), "");
        assert_eq!(line_text(&snapshot, block.start_line + 1), "$ ls");
        assert_eq!(snapshot.view().cursor.point.line.0, block.start_line + 1);
    }

    #[test]
    fn given_prompt_block_and_block_without_preexec_when_shown_then_they_have_no_status_line()
     {
        let dims = TestDimensions::new(8, 6);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        print_text(&mut surface, "boot");
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ ");

        let snapshot = surface.snapshot_owned();

        let first = block_snapshot(&snapshot, "");
        let prompt = block_snapshot(&snapshot, "prompt-1");
        assert!(!first.has_status_line);
        assert!(!first.is_status_line_visible);
        assert_eq!(line_text(&snapshot, first.start_line), "boot");
        assert!(!prompt.has_status_line);
        assert!(!prompt.is_status_line_visible);
        assert_eq!(prompt.line_count, 1);
        assert_eq!(line_text(&snapshot, prompt.start_line), "$");
    }

    #[test]
    fn given_prompt_block_when_its_command_starts_then_the_history_grows_by_one_line()
     {
        let dims = TestDimensions::new(8, 6);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ ls");
        let before = history_line_count(&surface);

        surface.handle_block_event(preexec_event("cmd-1", "ls", "/tmp"), 5_000);

        assert_eq!(history_line_count(&surface), before + 1);
    }

    #[test]
    fn given_command_with_status_line_when_its_text_is_copied_then_no_empty_line_is_in_it()
     {
        let mut surface = surface_running_ls();
        next_line(&mut surface);
        print_text(&mut surface, "a b");

        let running = surface.snapshot_owned();
        surface.handle_block_event(exit_event("cmd-1", 0), 5_100);
        surface.handle_block_event(precmd_event("prompt-2"), 5_100);
        let finished = surface.snapshot_owned();

        assert_eq!(
            block_text(&running, block_snapshot(&running, "cmd-1").key),
            Some(String::from("$ ls\na b"))
        );
        assert_eq!(
            running
                .view()
                .block_output_text(block_snapshot(&running, "cmd-1").key),
            Some(String::from("a b"))
        );
        assert_eq!(
            block_text(&finished, block_snapshot(&finished, "cmd-1").key),
            Some(String::from("$ ls\na b"))
        );
        assert_eq!(
            finished
                .view()
                .block_output_text(block_snapshot(&finished, "cmd-1").key),
            Some(String::from("a b"))
        );
    }

    #[test]
    fn given_finished_command_with_output_when_the_next_prompt_came_then_its_block_ends_with_its_last_line_of_output()
     {
        let mut surface = surface_with_seq_20(30);

        let snapshot = surface.snapshot_owned();

        // The status line, 2 command lines and 20 lines of output: the
        // empty line the cursor waited on is not shown.
        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.line_count, 1 + 2 + 20);
        assert_eq!(block.history_line_count, 1 + 2 + 20);
        assert_eq!(block_line_text(&snapshot, "cmd-1", 22), "20");
        let prompt = block_snapshot(&snapshot, "prompt-2");
        assert_eq!(prompt.start_line, block.start_line + 23);
        assert_eq!(line_text(&snapshot, prompt.start_line), "$ ls");
    }

    #[test]
    fn given_command_without_output_when_the_next_prompt_came_then_its_block_ends_with_its_command_line()
     {
        let dims = TestDimensions::new(12, 10);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ cd /tmp");
        start_command(&mut surface, "cmd-1", "cd /tmp");
        end_command(&mut surface, "cmd-1", "prompt-2");
        print_text(&mut surface, "$");

        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.line_count, 2);
        assert_eq!(block_line_text(&snapshot, "cmd-1", 1), "$ cd /tmp");
        let prompt = block_snapshot(&snapshot, "prompt-2");
        assert_eq!(prompt.start_line, block.start_line + 2);
    }

    #[test]
    fn given_output_that_ends_with_blank_lines_when_the_next_prompt_came_then_the_blank_lines_stay()
     {
        let dims = TestDimensions::new(8, 10);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ ls");
        start_command(&mut surface, "cmd-1", "ls");
        print_lines(&mut surface, &["a", "", ""]);
        end_command(&mut surface, "cmd-1", "prompt-2");
        print_text(&mut surface, "$");

        let snapshot = surface.snapshot_owned();

        // The status line, `$ ls`, `a` and the two blank lines it printed;
        // only the line the cursor waited on under them goes.
        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.line_count, 1 + 1 + 3);
        assert_eq!(block_line_text(&snapshot, "cmd-1", 2), "a");
        assert_eq!(block_line_text(&snapshot, "cmd-1", 3), "");
        assert_eq!(block_line_text(&snapshot, "cmd-1", 4), "");
        let prompt = block_snapshot(&snapshot, "prompt-2");
        assert_eq!(prompt.start_line, block.start_line + 5);
        assert_eq!(line_text(&snapshot, prompt.start_line), "$");
    }

    #[test]
    fn given_command_whose_next_prompt_has_not_come_when_shown_then_the_line_of_the_cursor_stays()
     {
        let mut surface = surface_running_ls();
        next_line(&mut surface);
        print_lines(&mut surface, &["a"]);

        let running = surface.snapshot_owned();
        surface.handle_block_event(exit_event("cmd-1", 0), 5_100);
        let ended = surface.snapshot_owned();

        for snapshot in [&running, &ended] {
            // The status line, `$ ls`, `a` and the line of the cursor.
            let block = block_snapshot(snapshot, "cmd-1");
            assert_eq!(block.line_count, 4);
            assert_eq!(
                snapshot.view().cursor.point.line.0,
                block.start_line + 3
            );
        }
    }

    #[test]
    fn given_finished_command_when_its_text_is_copied_then_it_is_the_whole_text_of_the_block()
     {
        let mut surface = surface_with_seq_20(30);

        let snapshot = surface.snapshot_owned();

        let key = block_snapshot(&snapshot, "cmd-1").key;
        let output: Vec<String> =
            (1..=20).map(|number| format!("{number}")).collect();
        let output = output.join("\n");
        assert_eq!(
            block_text(&snapshot, key),
            Some(format!("~/otty\n$ seq 20\n{output}\n"))
        );
        assert_eq!(
            snapshot.view().block_output_text(key),
            Some(format!("{output}\n"))
        );
    }

    #[test]
    fn given_selection_from_the_last_line_of_output_into_the_prompt_when_copied_then_no_empty_line_is_between()
     {
        let mut surface = surface_with_seq_20(30);
        let before = surface.snapshot_owned();
        let last_output_line = block_snapshot(&before, "cmd-1").start_line + 22;
        let prompt_line = block_snapshot(&before, "prompt-2").start_line;

        surface.start_selection(
            crate::SelectionType::Simple,
            Point::new(Line(last_output_line), Column(0)),
            crate::Side::Left,
        );
        surface.update_selection(
            Point::new(Line(prompt_line), Column(3)),
            crate::Side::Right,
        );
        let snapshot = surface.snapshot_owned();

        // The cells on screen, without line breaks: `20` and the rest of
        // its line, then the prompt.
        let rest_of_the_line = " ".repeat(10);
        assert_eq!(
            snapshot.view().selectable_content(),
            format!("20{rest_of_the_line}$ ls")
        );
    }

    #[test]
    fn given_block_whose_start_is_above_the_view_when_shown_then_its_status_line_is_hidden()
     {
        let dims = TestDimensions::new(8, 3);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ seq");
        surface
            .handle_block_event(preexec_event("cmd-1", "seq", "/tmp"), 5_000);
        for line in ["1", "2", "3", "4"] {
            next_line(&mut surface);
            print_text(&mut surface, line);
        }

        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert!(block.has_status_line);
        assert!(!block.is_status_line_visible);
        assert_eq!(block.line_count, 3);
        assert_eq!(line_text(&snapshot, block.start_line), "2");
        assert_eq!(
            block_text(&snapshot, block_snapshot(&snapshot, "cmd-1").key),
            Some(String::from("2\n3\n4"))
        );
    }

    /// Two finished commands, `$ a` printing `out` and `$ b`, and the
    /// prompt `$ c`.
    fn surface_with_two_commands() -> BlockSurface {
        let dims = TestDimensions::new(6, 10);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ a");
        surface.handle_block_event(preexec_event("cmd-1", "a", "/tmp"), 2_000);
        next_line(&mut surface);
        print_text(&mut surface, "out");
        surface.handle_block_event(exit_event("cmd-1", 0), 2_100);
        surface.handle_block_event(precmd_event("prompt-2"), 2_100);
        print_text(&mut surface, "$ b");
        surface.handle_block_event(preexec_event("cmd-2", "b", "/tmp"), 3_000);
        surface.handle_block_event(exit_event("cmd-2", 0), 3_100);
        surface.handle_block_event(precmd_event("prompt-3"), 3_100);
        print_text(&mut surface, "$ c");

        surface
    }

    #[test]
    fn given_selection_dragged_into_the_next_block_when_copied_then_its_status_line_is_skipped()
     {
        let mut surface = surface_with_two_commands();
        let before = surface.snapshot_owned();
        let output_line = block_snapshot(&before, "cmd-1").start_line + 2;
        let next_command_line = block_snapshot(&before, "cmd-2").start_line + 1;
        assert_eq!(line_text(&before, output_line), "out");
        assert_eq!(line_text(&before, next_command_line), "$ b");

        surface.start_selection(
            crate::SelectionType::Simple,
            Point::new(Line(output_line), Column(0)),
            crate::Side::Left,
        );
        surface.update_selection(
            Point::new(Line(next_command_line), Column(2)),
            crate::Side::Right,
        );
        let snapshot = surface.snapshot_owned();

        assert_eq!(snapshot.view().selectable_content(), "out   $ b");
    }

    #[test]
    fn given_point_on_a_status_line_when_resolved_then_it_belongs_to_its_block()
    {
        let mut surface = surface_with_two_commands();
        let snapshot = surface.snapshot_owned();
        let status_line = block_snapshot(&snapshot, "cmd-2").start_line;
        let point = Point::new(Line(status_line), Column(1));

        let slices = surface.block_slices();
        let context = surface.viewport_context(&slices);
        let resolved =
            surface.resolve_block_point_with(&slices, &context, point);

        let (index, _, _) = resolved.expect("the point is in the history");
        assert_eq!(surface.blocks[index].meta.id, "cmd-2");
        assert_eq!(
            snapshot
                .view()
                .block_at_point(point)
                .map(|block| block.meta.id.as_str()),
            Some("cmd-2")
        );
    }

    #[test]
    fn given_full_screen_program_when_shown_then_there_is_no_status_line() {
        use crate::escape::{NamedPrivateMode, PrivateMode};

        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ vim");
        surface
            .handle_block_event(preexec_event("cmd-1", "vim", "/tmp"), 5_000);
        surface.set_private_mode(PrivateMode::Named(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor,
        ));
        surface.goto(0, 0);
        print_text(&mut surface, "~");

        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert!(!block.is_status_line_visible);
        assert_eq!(block.start_line, 0);
        assert_eq!(line_text(&snapshot, 0), "~");
    }

    /// `seq 1 500` typed at the prompt `$ seq 1 500`: a block with one
    /// command line and 500 lines of output in a pane of 20 lines. `ls`,
    /// which prints `a` and `b`, and the prompt `$` come after it.
    fn surface_with_seq_500() -> BlockSurface {
        let dims = TestDimensions::new(12, 20);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ seq 1 500");
        start_command(&mut surface, "cmd-1", "seq 1 500");
        let output: Vec<String> =
            (1..=500).map(|number| format!("{number}")).collect();
        let output: Vec<&str> = output.iter().map(String::as_str).collect();
        print_lines(&mut surface, &output);
        end_command(&mut surface, "cmd-1", "prompt-2");
        print_text(&mut surface, "$ ls");
        start_command(&mut surface, "cmd-2", "ls");
        print_lines(&mut surface, &["a", "b"]);
        end_command(&mut surface, "cmd-2", "prompt-3");
        print_text(&mut surface, "$");

        surface
    }

    /// Scroll so that the line `line` of the text of `block_id`, counted
    /// from its first command line, is the top line of the pane.
    fn scroll_text_line_to_top(
        surface: &mut BlockSurface,
        block_id: &str,
        line: usize,
    ) {
        let text_start = surface
            .block_slices()
            .iter()
            .find(|slice| surface.blocks[slice.index].meta.id == block_id)
            .map(|slice| slice.text_start)
            .expect("the block is in the history");
        let max_offset =
            surface.total_lines().saturating_sub(surface.screen_lines());

        surface.display_offset = max_offset - (text_start + line);
    }

    /// The line of the snapshot at the top of the pane: lines on screen are
    /// counted from the bottom of the history, so it is minus the scroll
    /// offset.
    fn top_line(snapshot: &SnapshotOwned) -> i32 {
        -(snapshot.view().display_offset as i32)
    }

    #[test]
    fn given_view_412_lines_into_a_block_of_500_lines_when_shown_then_412_of_its_lines_are_above_the_pane()
     {
        let mut surface = surface_with_seq_500();
        scroll_text_line_to_top(&mut surface, "cmd-1", 412);

        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.text_lines_above, 412);
        assert!(!block.is_status_line_visible);
        assert_eq!(block.start_line, top_line(&snapshot));
        assert_eq!(line_text(&snapshot, top_line(&snapshot)), "412");
    }

    #[test]
    fn given_block_whose_first_line_is_on_screen_when_shown_then_none_of_its_lines_are_above_the_pane()
     {
        let mut surface = surface_with_seq_500();

        let snapshot = surface.snapshot_owned();

        let block = block_snapshot(&snapshot, "cmd-2");
        assert!(block.is_status_line_visible);
        assert_eq!(block.text_lines_above, 0);
        assert_eq!(block_snapshot(&snapshot, "prompt-3").text_lines_above, 0);
    }

    #[test]
    fn given_block_whose_status_line_only_is_above_the_pane_when_shown_then_none_of_its_text_is_above()
     {
        let mut surface = surface_with_seq_500();
        scroll_text_line_to_top(&mut surface, "cmd-1", 0);

        let snapshot = surface.snapshot_owned();

        // The status line is not terminal text, so it is not counted.
        let block = block_snapshot(&snapshot, "cmd-1");
        assert!(!block.is_status_line_visible);
        assert_eq!(block.text_lines_above, 0);
        assert_eq!(line_text(&snapshot, top_line(&snapshot)), "$ seq 1 500");
    }

    /// The terminal widget works out how far to scroll to bring a block
    /// into view, so a block tells how many lines it has in the whole
    /// history, also the lines off screen.
    #[test]
    fn given_block_mostly_above_the_pane_when_shown_then_it_counts_all_its_lines_in_the_history()
     {
        let mut surface = surface_with_seq_500();

        let snapshot = surface.snapshot_owned();

        // The status line, `$ seq 1 500` and 500 lines of output.
        let seq = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(seq.history_line_count, 502);
        assert!(seq.line_count < seq.history_line_count);
        let ls = block_snapshot(&snapshot, "cmd-2");
        assert_eq!(ls.history_line_count, 4);
        assert_eq!(ls.line_count, 4);
        let in_history: usize =
            snapshot.blocks.iter().map(|b| b.history_line_count).sum();
        assert_eq!(in_history, snapshot.view().size.total_lines);
    }

    #[test]
    fn given_folded_block_when_shown_then_it_counts_only_the_lines_the_history_shows()
     {
        let mut surface = surface_with_seq_500();
        surface.toggle_block_fold(key_of(&surface, "cmd-1"));

        let snapshot = surface.snapshot_owned();

        // The status line and `$ seq 1 500`.
        let seq = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(seq.history_line_count, 2);
    }

    #[test]
    fn given_block_wholly_above_the_pane_when_shown_then_it_still_counts_its_lines()
     {
        let mut surface = surface_with_seq_500();
        surface.scroll_display(Scroll::Top);

        let snapshot = surface.snapshot_owned();

        let ls = block_snapshot(&snapshot, "cmd-2");
        assert_eq!(ls.line_count, 0);
        assert_eq!(ls.history_line_count, 4);
    }

    #[test]
    fn given_folded_block_whose_first_command_line_is_above_the_pane_when_shown_then_only_the_lines_it_shows_are_counted()
     {
        let mut surface = surface_with_seq_20(6);
        start_command(&mut surface, "cmd-2", "ls");
        print_lines(&mut surface, &["1", "2", "3", "4", "5", "6", "7", "8"]);
        end_command(&mut surface, "cmd-2", "prompt-3");
        surface.toggle_block_fold(key_of(&surface, "cmd-1"));
        scroll_text_line_to_top(&mut surface, "cmd-1", 1);

        let snapshot = surface.snapshot_owned();

        // `~/otty` is above the pane; the 20 hidden lines are not.
        let block = block_snapshot(&snapshot, "cmd-1");
        assert_eq!(block.text_lines_above, 1);
        assert_eq!(line_text(&snapshot, top_line(&snapshot)), "$ seq 20");
    }

    #[test]
    fn given_full_screen_program_after_a_long_output_when_shown_then_nothing_is_above_the_pane()
     {
        use crate::escape::{NamedPrivateMode, PrivateMode};

        let mut surface = surface_with_seq_20(6);
        start_command(&mut surface, "cmd-2", "vim");
        surface.set_private_mode(PrivateMode::Named(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor,
        ));
        surface.goto(0, 0);
        print_text(&mut surface, "~");

        let snapshot = surface.snapshot_owned();

        assert_eq!(snapshot.blocks.len(), 1);
        assert_eq!(snapshot.blocks[0].text_lines_above, 0);
    }

    /// Issue 17: a script runs a full-screen program, which quits, and then
    /// prints 20 lines. The snapshot is not in the mode of a full-screen
    /// program any more, and it shows the script as any other command:
    /// its status line, now above the pane, and the lines of its text
    /// above the top edge, so the sticky header can show for it.
    #[test]
    fn given_command_whose_full_screen_program_quit_when_it_prints_more_then_it_is_shown_as_any_command()
     {
        use crate::escape::{NamedPrivateMode, PrivateMode};

        let full_screen =
            PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor);
        let mut surface = surface_with_seq_20(6);
        start_command(&mut surface, "cmd-2", "./deploy.sh");
        surface.set_private_mode(full_screen);
        let on_full_screen = surface.snapshot_owned();
        surface.unset_private_mode(full_screen);
        let output: Vec<String> =
            (1..=20).map(|number| format!("{number}")).collect();
        let output: Vec<&str> = output.iter().map(String::as_str).collect();
        print_lines(&mut surface, &output);

        let snapshot = surface.snapshot_owned();

        assert!(on_full_screen.view().mode.contains(SurfaceMode::ALT_SCREEN));
        assert!(!snapshot.view().mode.contains(SurfaceMode::ALT_SCREEN));
        let script = block_snapshot(&snapshot, "cmd-2");
        assert!(script.has_status_line);
        assert!(!script.is_status_line_visible);
        assert_eq!(script.line_count, 6);
        assert!(script.text_lines_above > 0);
    }

    #[test]
    fn given_new_terminal_when_the_first_prompt_comes_then_the_empty_start_block_takes_no_lines_and_the_prompt_is_at_the_bottom()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ ");
        let snapshot = surface.snapshot_owned();

        let start = &snapshot.blocks[0];
        assert_eq!(start.meta.kind, BlockKind::Start);
        assert_eq!(start.line_count, 0);
        let prompt = block_snapshot(&snapshot, "prompt-1");
        assert_eq!(prompt.start_line, 3);
        assert_eq!(prompt.line_count, 1);
        assert_eq!(history_line_count(&surface), 1);
        assert_eq!(snapshot.view().size.total_lines, 4);
    }

    #[test]
    fn given_new_terminal_before_the_first_prompt_when_shown_then_the_cursor_line_is_at_the_bottom()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        let snapshot = surface.snapshot_owned();

        let start = &snapshot.blocks[0];
        assert_eq!(start.meta.kind, BlockKind::Start);
        assert_eq!(start.start_line, 3);
        assert_eq!(start.line_count, 1);
        assert_eq!(snapshot.view().cursor.point.line, Line(3));
    }

    #[test]
    fn given_commands_after_an_empty_start_when_scrolled_to_the_top_then_the_first_command_block_is_the_top_of_the_history()
     {
        let dims = TestDimensions::new(8, 3);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ ls");
        start_command(&mut surface, "cmd-1", "ls");
        print_lines(&mut surface, &["a", "b"]);
        end_command(&mut surface, "cmd-1", "prompt-2");
        print_text(&mut surface, "$ ");

        surface.scroll_display(Scroll::Top);
        let snapshot = surface.snapshot_owned();

        let first = snapshot
            .blocks
            .iter()
            .find(|block| block.line_count > 0)
            .expect("a block is on screen");
        assert_eq!(first.meta.id, "cmd-1");
        assert_eq!(first.start_line, top_line(&snapshot));
        assert!(first.is_status_line_visible);
        // The status line, `$ ls`, `a`, `b` and the prompt.
        assert_eq!(history_line_count(&surface), 5);
    }

    #[test]
    fn given_text_before_the_first_prompt_when_shown_then_it_stays_and_can_be_selected_as_text()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        print_text(&mut surface, "Welcome");
        next_line(&mut surface);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ ");

        let before = surface.snapshot_owned();
        let start = &before.blocks[0];
        assert_eq!(start.meta.kind, BlockKind::Start);
        // `Welcome` and the empty line the cursor was on when the prompt
        // came: only a command block leaves that line out.
        assert_eq!(start.line_count, 2);
        assert_eq!(line_text(&before, start.start_line), "Welcome");
        let welcome_line = start.start_line;

        surface.start_selection(
            crate::SelectionType::Simple,
            Point::new(Line(welcome_line), Column(0)),
            crate::Side::Left,
        );
        surface.update_selection(
            Point::new(Line(welcome_line), Column(6)),
            crate::Side::Right,
        );
        let snapshot = surface.snapshot_owned();

        assert_eq!(snapshot.view().selectable_content(), "Welcome");
    }

    #[test]
    fn given_shell_without_the_integration_when_it_prints_then_its_only_block_is_the_start_block()
     {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        print_text(&mut surface, "$ ls");
        next_line(&mut surface);
        print_text(&mut surface, "a");
        let snapshot = surface.snapshot_owned();

        assert_eq!(snapshot.blocks.len(), 1);
        assert_eq!(snapshot.blocks[0].meta.kind, BlockKind::Start);
        assert!(!snapshot.blocks[0].has_status_line);
        assert_eq!(snapshot.blocks[0].line_count, 2);
    }

    #[test]
    fn given_long_history_when_scrolled_to_an_offset_then_the_view_is_that_many_lines_up()
     {
        let mut surface = surface_with_seq_500();
        let max_offset =
            surface.total_lines().saturating_sub(surface.screen_lines());

        surface.scroll_display(Scroll::Offset(120));
        assert_eq!(surface.snapshot_owned().view().display_offset, 120);

        surface.scroll_display(Scroll::Offset(0));
        assert_eq!(surface.snapshot_owned().view().display_offset, 0);

        surface.scroll_display(Scroll::Offset(max_offset + 50));
        assert_eq!(surface.snapshot_owned().view().display_offset, max_offset);
    }

    /// The key of the block whose command is `cmd`.
    fn command_key(surface: &BlockSurface, cmd: &str) -> BlockKey {
        surface
            .blocks
            .iter()
            .find(|block| block.meta.cmd.as_deref() == Some(cmd))
            .map(|block| block.key)
            .expect("a block runs this command")
    }

    /// The snapshot of the block with `key`.
    fn keyed_block(snapshot: &SnapshotOwned, key: BlockKey) -> &BlockSnapshot {
        snapshot
            .view()
            .block(key)
            .expect("the block with this key is in the snapshot")
    }

    /// `ls` printing `a`, then `zsh`, which starts a shell inside this one,
    /// and in the inner shell `pwd` printing `/tmp` and `date` printing
    /// `today`, then the inner prompt `%`. The inner shell counts from 1
    /// again, so `pwd` gets the id of `ls` and `date` the id of `zsh`.
    fn surface_with_inner_shell(screen_lines: usize) -> BlockSurface {
        let dims = TestDimensions::new(12, screen_lines);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        print_text(&mut surface, "$ ls");
        start_command(&mut surface, "cmd-1", "ls");
        print_lines(&mut surface, &["a"]);
        end_command(&mut surface, "cmd-1", "prompt-2");
        print_text(&mut surface, "$ zsh");
        start_command(&mut surface, "cmd-2", "zsh");
        // The first prompt of the inner shell.
        surface.handle_block_event(precmd_event("prompt-1"), 7_000);
        print_text(&mut surface, "% pwd");
        start_command(&mut surface, "cmd-1", "pwd");
        print_lines(&mut surface, &["/tmp"]);
        end_command(&mut surface, "cmd-1", "prompt-2");
        print_text(&mut surface, "% date");
        start_command(&mut surface, "cmd-2", "date");
        print_lines(&mut surface, &["today"]);
        end_command(&mut surface, "cmd-2", "prompt-3");
        print_text(&mut surface, "%");

        surface
    }

    #[test]
    fn given_inner_shell_counting_from_1_again_when_its_blocks_are_made_then_every_block_has_a_key_of_its_own()
     {
        let mut surface = surface_with_inner_shell(30);

        let snapshot = surface.snapshot_owned();

        let ls = keyed_block(&snapshot, command_key(&surface, "ls"));
        let pwd = keyed_block(&snapshot, command_key(&surface, "pwd"));
        assert_eq!(ls.meta.id, pwd.meta.id);
        assert_ne!(ls.key, pwd.key);
        let keys: std::collections::HashSet<BlockKey> =
            snapshot.blocks.iter().map(|block| block.key).collect();
        assert_eq!(keys.len(), snapshot.blocks.len());
    }

    #[test]
    fn given_prompt_block_when_its_command_starts_then_the_block_keeps_its_key()
    {
        let dims = TestDimensions::new(8, 4);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-1"), 1_000);
        let prompt = surface.snapshot_owned().blocks[1].key;

        surface.handle_block_event(preexec_event("cmd-1", "ls", "/tmp"), 5_000);

        assert_eq!(command_key(&surface, "ls"), prompt);
    }

    #[test]
    fn given_two_blocks_with_one_id_when_the_older_one_is_folded_by_its_key_then_only_it_folds()
     {
        let mut surface = surface_with_inner_shell(30);
        let ls = command_key(&surface, "ls");
        let pwd = command_key(&surface, "pwd");

        surface.toggle_block_fold(ls);
        let snapshot = surface.snapshot_owned();

        assert!(keyed_block(&snapshot, ls).is_folded());
        assert!(!keyed_block(&snapshot, pwd).is_folded());
    }

    #[test]
    fn given_two_blocks_with_one_id_when_each_is_copied_by_its_key_then_each_gives_its_own_text()
     {
        let mut surface = surface_with_inner_shell(30);
        let ls = command_key(&surface, "ls");
        let pwd = command_key(&surface, "pwd");

        let snapshot = surface.snapshot_owned();

        let ls_text = block_text(&snapshot, ls).expect("ls has text");
        let pwd_text = block_text(&snapshot, pwd).expect("pwd has text");
        assert_eq!(ls_text.lines().collect::<Vec<_>>(), ["$ ls", "a"]);
        assert_eq!(pwd_text.lines().collect::<Vec<_>>(), ["% pwd", "/tmp"]);
        assert_eq!(
            snapshot.view().block_output_text(pwd).as_deref(),
            Some("/tmp\n")
        );
    }

    /// Owner, issue 13: the shell sends no `exit` for the command that
    /// starts an inner shell before the inner shell shows its first prompt,
    /// so that prompt closes the block of the command: it is finished, has
    /// no exit code and took the time up to that prompt.
    #[test]
    fn given_command_that_starts_an_inner_shell_when_the_inner_prompt_comes_then_its_block_is_finished_without_a_code()
     {
        let surface = surface_with_inner_shell(30);

        let zsh = surface
            .blocks
            .iter()
            .find(|block| block.meta.cmd.as_deref() == Some("zsh"))
            .map(|block| &block.meta)
            .expect("the block of zsh");

        assert!(zsh.is_finished);
        assert_eq!(zsh.exit_code, None);
        assert_eq!(zsh.started_at, Some(5_000));
        assert_eq!(zsh.finished_at, Some(7_000));
    }

    /// Owner, issue 13: the shell integration puts the process id of the
    /// shell into its ids, so an inner shell never sends the id of a block
    /// of the outer one. When the inner shell ends with `exit`, the outer
    /// shell sends the `exit` of its `zsh`; it must not end the block of
    /// that `exit`, which belongs to the inner shell.
    #[test]
    fn given_inner_shell_ended_by_exit_when_the_outer_exit_arrives_then_no_block_of_the_inner_shell_ends()
     {
        let dims = TestDimensions::new(12, 20);
        let mut surface = BlockSurface::new(SurfaceConfig::default(), &dims);
        surface.handle_block_event(precmd_event("prompt-100-1"), 1_000);
        print_text(&mut surface, "$ zsh");
        start_command(&mut surface, "cmd-100-1", "zsh");
        surface.handle_block_event(precmd_event("prompt-200-1"), 2_000);
        print_text(&mut surface, "% exit");
        start_command(&mut surface, "cmd-200-1", "exit");

        surface.handle_block_event(exit_event("cmd-100-1", 0), 3_000);

        let exit = surface
            .blocks
            .iter()
            .find(|block| block.meta.cmd.as_deref() == Some("exit"))
            .map(|block| &block.meta)
            .expect("the block of exit");
        assert!(!exit.is_finished);
        assert_eq!(exit.exit_code, None);
    }

    #[test]
    fn given_selection_from_one_block_into_the_next_when_shown_then_the_status_line_of_the_next_is_not_drawn_selected()
     {
        let mut surface = surface_with_two_commands();
        let before = surface.snapshot_owned();
        let output_line = block_snapshot(&before, "cmd-1").start_line + 2;
        let status_line = block_snapshot(&before, "cmd-2").start_line;
        let next_command_line = status_line + 1;

        surface.start_selection(
            crate::SelectionType::Simple,
            Point::new(Line(output_line), Column(0)),
            crate::Side::Left,
        );
        surface.update_selection(
            Point::new(Line(next_command_line), Column(2)),
            crate::Side::Right,
        );
        let snapshot = surface.snapshot_owned();
        let view = snapshot.view();

        assert!(view.is_selected(Point::new(Line(output_line), Column(0))));
        assert!(
            view.is_selected(Point::new(Line(next_command_line), Column(0)))
        );
        for column in 0..6 {
            let point = Point::new(Line(status_line), Column(column));
            assert!(view.selection.is_some_and(|range| range.contains(point)));
            assert!(!view.is_selected(point), "column {column}");
        }
    }
}
