use std::cmp::{max, min};
use std::sync::Arc;

use crate::cell::Cell;
use crate::escape::{
    BlockKind as EscapeBlockKind, BlockMeta as EscapeBlockMeta, BlockPhase,
};
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

/// Kind of a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BlockKind {
    #[default]
    Command,
    Prompt,
    FullScreen,
}

impl From<EscapeBlockKind> for BlockKind {
    fn from(value: EscapeBlockKind) -> Self {
        match value {
            EscapeBlockKind::Command => Self::Command,
            EscapeBlockKind::FullScreen => Self::FullScreen,
            EscapeBlockKind::Prompt => Self::Prompt,
        }
    }
}

/// Minimal metadata associated with a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMeta {
    /// Unique identifier reported by the escape handler for this block.
    pub id: String,
    /// Semantic category (prompt, command, fullscreen) of the block.
    pub kind: BlockKind,
    /// Command line (if known) that generated the block.
    pub cmd: Option<String>,
    /// Working directory captured when the block started.
    pub cwd: Option<String>,
    /// Shell executable responsible for the block.
    pub shell: Option<String>,
    /// Exit status of the command, once finished.
    pub exit_code: Option<i32>,
    /// Timestamp marking when the block started executing.
    pub started_at: Option<i64>,
    /// Timestamp marking when the block finished executing.
    pub finished_at: Option<i64>,
    /// Whether the block ever entered alt-screen mode.
    pub is_alt_screen: bool,
    /// Whether the block has finished producing output.
    pub is_finished: bool,
}

impl From<EscapeBlockMeta> for BlockMeta {
    fn from(value: EscapeBlockMeta) -> Self {
        BlockMeta {
            id: value.id,
            kind: BlockKind::from(value.kind),
            cmd: value.cmd,
            cwd: value.cwd,
            shell: value.shell,
            started_at: value.started_at,
            finished_at: value.finished_at,
            exit_code: value.exit_code,
            is_alt_screen: value.is_alt_screen,
            is_finished: false,
        }
    }
}

/// Snapshot entry describing a block's extent within the viewport.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockSnapshot {
    /// Metadata describing the block captured in this snapshot entry.
    pub meta: BlockMeta,
    /// Viewport-relative line where this block begins.
    pub start_line: i32,
    /// Number of visible lines contributed by this block.
    pub line_count: usize,
    /// Cached full textual contents for finished blocks.
    ///
    /// This is intentionally detached from the viewport so UI actions like
    /// "copy content" can include scrollback lines that are currently off-screen.
    pub cached_text: Option<Arc<str>>,
    /// Whether this block snapshot corresponds to an alt screen.
    pub is_alt_screen: bool,
}

/// In‑memory representation of a single block.
struct Block {
    /// Metadata describing the block's identity and lifecycle.
    pub meta: BlockMeta,
    /// Embedded surface that records the block's terminal contents.
    pub surface: Surface,
    /// Cached textual contents for finished blocks.
    pub cached_text: Option<Arc<str>>,
}

impl Block {
    /// Construct a block with its own `Surface` configured for the provided
    /// dimensions so it can capture terminal output independently.
    fn new<D: Dimensions>(
        config: &SurfaceConfig,
        dimensions: &D,
        meta: BlockMeta,
    ) -> Self {
        Self {
            meta,
            surface: Surface::new(config.clone(), dimensions),
            cached_text: None,
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
        if text.is_empty() {
            self.cached_text = None;
        } else {
            self.cached_text = Some(Arc::<str>::from(text));
        }
    }
}

/// Aggregate geometry for a block within the concatenated history.
struct BlockSliceInfo {
    /// Index into the `blocks` array.
    index: usize,
    /// Global history line index marking the start of this block slice.
    start: usize,
    /// Global history line index marking the end of this block slice.
    end: usize,
    /// First visible line inside the block's own coordinate space.
    top_line: Line,
}

struct ViewportContext {
    /// Total number of lines contributed by all blocks.
    content_lines: usize,
    /// Global line where the viewport begins.
    start: usize,
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
    blocks: Vec<Block>,
    /// Upper bound on how many blocks to retain in history.
    max_blocks: usize,
    /// Rendering configuration cloned into new blocks.
    config: SurfaceConfig,
    /// Global scroll offset expressed in lines from the bottom.
    display_offset: usize,
    /// Index of the block currently owning the local selection, if any.
    selection_block: Option<usize>,
    /// Fixed anchor used when a selection extends beyond its original block.
    selection_anchor: Option<GlobalPoint>,
    /// Selection that spans multiple blocks in global coordinates.
    global_selection: Option<GlobalSelection>,
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
        let history = self
            .blocks
            .iter()
            .map(Self::block_visible_line_count)
            .sum::<usize>();

        max(viewport, history)
    }
}

impl BlockSurface {
    /// Default maximum number of blocks to keep.
    pub const DEFAULT_MAX_BLOCKS: usize = 1000;

    /// Create a new block surface with a single empty block.
    pub fn new<D: Dimensions>(config: SurfaceConfig, dimensions: &D) -> Self {
        let block = Block::new(&config, dimensions, BlockMeta::default());

        Self {
            blocks: vec![block],
            max_blocks: Self::DEFAULT_MAX_BLOCKS,
            config,
            display_offset: 0,
            selection_anchor: None,
            selection_block: None,
            global_selection: None,
        }
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
    fn block_slices(&self) -> Vec<BlockSliceInfo> {
        if self.is_alt_screen_active() {
            if let Some(block) = self.blocks.get(self.last_block_idx()) {
                let (top_line, total_lines) = Self::block_visible_extent(block);
                return vec![BlockSliceInfo {
                    index: self.last_block_idx(),
                    start: 0,
                    end: total_lines,
                    top_line,
                }];
            }
        }

        let mut start = 0;
        let mut result = Vec::with_capacity(self.blocks.len());

        for (index, block) in self.blocks.iter().enumerate() {
            let (top_line, total_lines) = Self::block_visible_extent(block);
            let slice = BlockSliceInfo {
                index,
                start,
                end: start + total_lines,
                top_line,
            };
            start += total_lines;
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
    fn viewport_context(&self, slices: &[BlockSliceInfo]) -> ViewportContext {
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
        let total_lines = slice.end.saturating_sub(slice.start);
        if offset >= total_lines {
            return None;
        }

        Some(slice.start + offset)
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

    /// Terminates the current block (if it's still running), creates a new block
    /// with a new `Surface`, and makes it active.
    fn begin_block(&mut self, meta: BlockMeta) {
        // Mark the active block as complete if it is not already marked.
        let idx = self.last_block_idx();
        if let Some(active) = self.blocks.get_mut(idx) {
            if !active.meta.is_finished {
                active.meta.is_finished = true;

                if active.meta.finished_at.is_none() {
                    active.meta.finished_at =
                        meta.started_at.or(meta.finished_at);
                }
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

        self.blocks.push(Block::new(&self.config, &size, meta));

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

    /// Updates the metadata and marks the block with the given `id` as complete.
    pub fn end_block_by_id(&mut self, meta: &BlockMeta) {
        if let Some(block) =
            self.blocks.iter_mut().find(|b| b.meta.id == meta.id)
        {
            block.meta = meta.clone();
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

        block.meta.is_alt_screen =
            block.surface.mode().contains(SurfaceMode::ALT_SCREEN);

        let mut snapshot = SnapshotOwned::from_surface(&mut block.surface);
        let line_count = snapshot.view().size.screen_lines;
        snapshot.blocks = vec![BlockSnapshot {
            meta: block.meta.clone(),
            start_line: 0,
            line_count,
            cached_text: block.cached_text.clone(),
            is_alt_screen: block.meta.is_alt_screen,
        }];

        snapshot
    }

    /// Return the number of visible lines from a block, including trimmed
    /// viewport content and scrollback.
    fn block_visible_line_count(block: &Block) -> usize {
        Self::block_visible_extent(block).1
    }

    /// Calculate the top-most visible line and total visible line count for a
    /// block by trimming empty viewport rows.
    fn block_visible_extent(block: &Block) -> (Line, usize) {
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
    fn resolve_block_point_with(
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
        let local_index = global_index.saturating_sub(slice.start);
        let block = &self.blocks[slice.index];
        let block_line = slice.top_line + local_index;
        let column = min(
            point.column,
            Column(block.surface.columns().saturating_sub(1)),
        );

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

    /// React to prompt/command lifecycle events emitted by the parser.
    fn handle_block_event(&mut self, event: crate::escape::BlockEvent) {
        let escape_meta = event.meta;
        let mut meta = BlockMeta::from(escape_meta);

        // BlockPhase mirrors the shell lifecycle:
        // - `Precmd` fires right before the prompt is drawn, so we ensure the
        //   last block is a prompt pinned to the bottom.
        // - `Preexec` signals that the user just submitted that prompt; we flip
        //   the same block into `Command` so stdout appears directly beneath the
        //   initiating prompt.
        // - `Exit` finalizes metadata once the running command finishes.
        match event.phase {
            // Command start hook
            BlockPhase::Preexec => {
                // Reuse the active prompt block so the pending command produces
                // output directly under the prompt instead of opening a new block.
                meta.kind = BlockKind::Command;
                if let Some(prompt_idx) = self.active_prompt_index() {
                    self.blocks[prompt_idx].meta = meta;
                    self.calculate_display_offset();
                } else {
                    self.begin_block(meta);
                }
            },
            BlockPhase::Exit => {
                self.end_block_by_id(&meta);
            },
            // Prompt block start hook
            BlockPhase::Precmd => {
                meta.kind = BlockKind::Prompt;

                if self.blocks.iter().any(|b| b.meta.id == meta.id) {
                    self.end_block_by_id(&meta);
                    return;
                }

                if let Some(index) = self.active_prompt_index() {
                    if self.blocks.len() > 1 {
                        self.remove_block_at(index);
                    }
                }

                self.begin_block(meta);
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

            if global_index < slice.start {
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
            let local_index = global_index - slice.start;
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
        if let Some(slice) = active_slice {
            if let Some(point) =
                self.convert_point_to_view(slice, cursor.point, start)
            {
                cursor.point = point;
            }
        }

        let mut selection = None;
        if let Some(index) =
            self.selection_block.filter(|&idx| idx < self.blocks.len())
        {
            if let Some(slice) =
                slices.iter().find(|slice| slice.index == index)
            {
                if let Some(range) = self.blocks[index]
                    .surface
                    .selection
                    .as_ref()
                    .and_then(|s| s.to_range(&self.blocks[index].surface))
                {
                    selection =
                        self.convert_selection_to_view(range, slice, start);
                }
            }
        }

        let size = SnapshotSize {
            columns,
            screen_lines: viewport_lines,
            total_lines: self.total_lines(),
        };
        let visible_cell_count = size.columns * size.screen_lines;

        let hyperlinks = HyperlinkMap::build_without_surface(
            &cells,
            size,
            self.display_offset,
        );

        if selection.is_none() {
            if let Some(global_selection) = &self.global_selection {
                selection = self.global_selection_to_view(
                    global_selection,
                    &context,
                    viewport_lines,
                );
            }
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

            block_snapshots.push(BlockSnapshot {
                meta: block.meta.clone(),
                start_line,
                line_count,
                cached_text: block.cached_text.clone(),
                is_alt_screen: block.meta.is_alt_screen,
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
    use crate::snapshot::SurfaceModel;
    use crate::{Dimensions, Line, SurfaceConfig};

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

        fn last_column(&self) -> crate::index::Column {
            crate::index::Column(self.columns - 1)
        }

        fn bottommost_line(&self) -> Line {
            Line(self.lines as i32 - 1)
        }
    }

    impl TestDimensions {
        fn new(columns: usize, lines: usize) -> Self {
            Self { columns, lines }
        }
    }

    #[test]
    fn block_surface_starts_with_single_block() {
        let dims = TestDimensions::new(4, 2);
        let surface = BlockSurface::new(SurfaceConfig::default(), &dims);

        assert_eq!(surface.blocks.len(), 1);
        assert_eq!(surface.blocks[0].meta.kind, BlockKind::Command);
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
            is_alt_screen: false,
            is_finished: false,
        };

        surface.begin_block(meta);
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
            is_alt_screen: false,
            is_finished: false,
        };

        surface.begin_block(start_meta);

        let meta = BlockMeta {
            id: String::from("cmd-1"),
            kind: BlockKind::Command,
            cmd: None,
            cwd: None,
            shell: None,
            started_at: Some(10),
            finished_at: Some(20),
            exit_code: Some(0),
            is_alt_screen: false,
            is_finished: false,
        };

        surface.end_block_by_id(&meta);

        let block = surface
            .blocks
            .iter()
            .find(|b| b.meta.id == "cmd-1")
            .expect("block exists");
        assert_eq!(block.meta.exit_code, Some(0));
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
                is_alt_screen: false,
                is_finished: false,
            };
            surface.begin_block(meta);
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
            is_alt_screen: false,
            is_finished: false,
        };
        surface.begin_block(meta);

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
            is_alt_screen: false,
            is_finished: false,
        };

        surface.begin_block(meta);
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
            is_alt_screen: false,
            is_finished: false,
        };

        surface.begin_block(meta);
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
            is_alt_screen: false,
            is_finished: false,
        };
        surface.begin_block(meta_1);
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
            is_alt_screen: false,
            is_finished: false,
        };
        surface.begin_block(meta_2);

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

        assert_eq!(snapshot.block_text("block-1"), Some(String::from("A\nB")));
    }
}
