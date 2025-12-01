use std::cmp::{max, min};

use crate::grid::Scroll;
use crate::hyperlink::HyperlinkMap;
use crate::index::{Column, Line, Point};
use crate::selection::SelectionRange;
use crate::snapshot::{
    CursorSnapshot, SnapshotCell, SnapshotDamage, SnapshotOwned, SnapshotSize,
    SurfaceModel,
};
use crate::{Dimensions, Surface, SurfaceActor, SurfaceConfig};

/// Kind of a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BlockKind {
    #[default]
    Command,
    Prompt,
    FullScreen,
}

/// Minimal metadata associated with a terminal block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMeta {
    pub id: String,
    pub kind: BlockKind,
    pub cmd: Option<String>,
    pub cwd: Option<String>,
    pub shell: Option<String>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub exit_code: Option<i32>,
    pub is_alt_screen: bool,
    pub is_finished: bool,
}

/// Public subset of block metadata exposed via snapshots/UI.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMetaPublic {
    pub id: String,
    pub kind: BlockKind,
    pub cmd: Option<String>,
    pub cwd: Option<String>,
    pub shell: Option<String>,
    pub exit_code: Option<i32>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

impl From<&BlockMeta> for BlockMetaPublic {
    fn from(meta: &BlockMeta) -> Self {
        Self {
            id: meta.id.clone(),
            kind: meta.kind.clone(),
            cmd: meta.cmd.clone(),
            cwd: meta.cwd.clone(),
            shell: meta.shell.clone(),
            exit_code: meta.exit_code,
            started_at: meta.started_at,
            finished_at: meta.finished_at,
        }
    }
}

/// Snapshot entry describing a block's extent within the viewport.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockSnapshot {
    pub id: String,
    pub meta: BlockMetaPublic,
    pub start_line: i32,
    pub line_count: usize,
    pub is_alt_screen: bool,
}

/// In‑memory representation of a single block.
pub struct Block {
    pub meta: BlockMeta,
    pub surface: Surface,
}

/// Aggregate geometry for a block within the concatenated history.
struct BlockSliceInfo {
    index: usize,
    start: usize,
    end: usize,
    top_line: Line,
}

/// Helper dimensions type used when creating new block surfaces.
struct BlockDimensions {
    columns: usize,
    screen_lines: usize,
}

impl Dimensions for BlockDimensions {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

impl Block {
    fn new<D: Dimensions>(
        config: &SurfaceConfig,
        dimensions: &D,
        meta: BlockMeta,
    ) -> Self {
        Self {
            meta,
            surface: Surface::new(config.clone(), dimensions),
        }
    }
}

/// Multi‑grid surface composed of several blocks.
///
/// On this step `BlockSurface` behaves as a thin wrapper around a single
/// [`Surface`]: all operations are delegated to the active block and
/// snapshots are exported only for that block.
pub struct BlockSurface {
    blocks: Vec<Block>,
    active: usize,
    max_blocks: usize,
    config: SurfaceConfig,
    display_offset: usize,
}

impl BlockSurface {
    /// Default maximum number of blocks to keep.
    pub const DEFAULT_MAX_BLOCKS: usize = 1000;

    /// Create a new block surface with a single empty block.
    pub fn new<D: Dimensions>(config: SurfaceConfig, dimensions: &D) -> Self {
        let meta = BlockMeta {
            id: String::from("0"),
            kind: BlockKind::Command,
            cmd: None,
            cwd: None,
            shell: None,
            started_at: None,
            finished_at: None,
            exit_code: None,
            is_alt_screen: false,
            is_finished: false,
        };

        let block = Block::new(&config, dimensions, meta);

        Self {
            blocks: vec![block],
            active: 0,
            max_blocks: Self::DEFAULT_MAX_BLOCKS,
            config,
            display_offset: 0,
        }
    }

    /// Return a mutable reference to the active block.
    fn active_block_mut(&mut self) -> &mut Block {
        &mut self.blocks[self.active]
    }

    fn columns(&self) -> usize {
        self.blocks
            .get(self.active)
            .map(|block| block.surface.columns())
            .unwrap_or(0)
    }

    fn screen_lines(&self) -> usize {
        self.blocks
            .get(self.active)
            .map(|block| block.surface.screen_lines())
            .unwrap_or(0)
    }

    fn total_lines(&self) -> usize {
        let viewport = self.screen_lines();
        let history = self
            .blocks
            .iter()
            .map(|block| {
                let grid = block.surface.grid();
                grid.history_size() + grid.screen_lines()
            })
            .sum::<usize>();

        max(viewport, history)
    }

    fn clamp_display_offset(&mut self) {
        let viewport_lines = self.screen_lines();
        let max_offset = self.total_lines().saturating_sub(viewport_lines);
        self.display_offset = min(self.display_offset, max_offset);
    }

    fn block_slices(&self) -> Vec<BlockSliceInfo> {
        let mut start = 0;
        let mut result = Vec::with_capacity(self.blocks.len());

        for (index, block) in self.blocks.iter().enumerate() {
            let grid = block.surface.grid();
            let total_lines = grid.history_size() + grid.screen_lines();
            let slice = BlockSliceInfo {
                index,
                start,
                end: start + total_lines,
                top_line: grid.topmost_line(),
            };
            start += total_lines;
            result.push(slice);
        }

        result
    }

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
    ///
    /// Oldest завершённые блоки удаляются первыми; активный и другие
    /// незавершённые блоки не удаляются.
    fn enforce_max_blocks(&mut self) {
        if self.blocks.len() <= self.max_blocks {
            return;
        }

        let mut index = 0;
        while self.blocks.len() > self.max_blocks && index < self.blocks.len() {
            if self.blocks[index].meta.is_finished && index != self.active {
                self.blocks.remove(index);

                if self.active > index {
                    self.active -= 1;
                } else if self.active >= self.blocks.len() {
                    self.active = self.blocks.len().saturating_sub(1);
                }
            } else {
                index += 1;
            }
        }

        self.clamp_display_offset();
    }

    /// Завершает текущий блок (если он ещё running), создаёт новый блок
    /// с новым `Surface` и делает его активным.
    pub fn begin_block(&mut self, mut meta: BlockMeta) -> &mut Surface {
        // Пометить активный блок завершённым, если он ещё не помечен.
        if let Some(active) = self.blocks.get_mut(self.active) {
            if !active.meta.is_finished {
                active.meta.is_finished = true;

                if active.meta.finished_at.is_none() {
                    active.meta.finished_at =
                        meta.started_at.or(meta.finished_at);
                }
            }
        }

        // Новый блок создаётся с текущими размерами поверхности.
        let size = {
            let surface = &self.blocks[self.active].surface;
            BlockDimensions {
                columns: surface.columns(),
                screen_lines: surface.screen_lines(),
            }
        };

        meta.is_finished = false;

        let block = Block::new(&self.config, &size, meta);
        self.blocks.push(block);
        self.active = self.blocks.len() - 1;
        self.enforce_max_blocks();
        self.display_offset = 0;
        self.clamp_display_offset();

        &mut self.blocks[self.active].surface
    }

    /// Обновляет метаданные и помечает блок с данным `id` как завершённый.
    pub fn end_block_by_id(&mut self, meta: &BlockMeta) {
        if let Some(block) =
            self.blocks.iter_mut().find(|b| b.meta.id == meta.id)
        {
            if let Some(cmd) = &meta.cmd {
                block.meta.cmd = Some(cmd.clone());
            }

            if let Some(cwd) = &meta.cwd {
                block.meta.cwd = Some(cwd.clone());
            }

            if let Some(shell) = &meta.shell {
                block.meta.shell = Some(shell.clone());
            }

            if let Some(finished_at) = meta.finished_at {
                block.meta.finished_at = Some(finished_at);
            }

            if let Some(exit_code) = meta.exit_code {
                block.meta.exit_code = Some(exit_code);
            }

            block.meta.kind = meta.kind.clone();
            block.meta.is_alt_screen = meta.is_alt_screen;
            block.meta.is_finished = true;
        }

        self.enforce_max_blocks();
    }
}

impl SurfaceActor for BlockSurface {
    fn print(&mut self, c: char) {
        self.active_block_mut().surface.print(c);
    }

    fn resize<S: Dimensions>(&mut self, size: S) {
        self.active_block_mut().surface.resize(size);
    }

    fn insert_blank(&mut self, count: usize) {
        self.active_block_mut().surface.insert_blank(count);
    }

    fn insert_blank_lines(&mut self, count: usize) {
        self.active_block_mut().surface.insert_blank_lines(count);
    }

    fn delete_lines(&mut self, count: usize) {
        self.active_block_mut().surface.delete_lines(count);
    }

    fn delete_chars(&mut self, count: usize) {
        self.active_block_mut().surface.delete_chars(count);
    }

    fn erase_chars(&mut self, count: usize) {
        self.active_block_mut().surface.erase_chars(count);
    }

    fn backspace(&mut self) {
        self.active_block_mut().surface.backspace();
    }

    fn carriage_return(&mut self) {
        self.active_block_mut().surface.carriage_return();
    }

    fn line_feed(&mut self) {
        self.active_block_mut().surface.line_feed();
    }

    fn new_line(&mut self) {
        self.active_block_mut().surface.new_line();
    }

    fn set_horizontal_tab(&mut self) {
        self.active_block_mut().surface.set_horizontal_tab();
    }

    fn reverse_index(&mut self) {
        self.active_block_mut().surface.reverse_index();
    }

    fn reset(&mut self) {
        self.active_block_mut().surface.reset();
    }

    fn clear_screen(&mut self, mode: crate::escape::ClearMode) {
        self.active_block_mut().surface.clear_screen(mode);
    }

    fn clear_line(&mut self, mode: crate::escape::LineClearMode) {
        self.active_block_mut().surface.clear_line(mode);
    }

    fn insert_tabs(&mut self, count: usize) {
        self.active_block_mut().surface.insert_tabs(count);
    }

    fn clear_tabs(&mut self, mode: crate::escape::TabClearMode) {
        self.active_block_mut().surface.clear_tabs(mode);
    }

    fn screen_alignment_display(&mut self) {
        self.active_block_mut().surface.screen_alignment_display();
    }

    fn move_forward_tabs(&mut self, count: usize) {
        self.active_block_mut().surface.move_forward_tabs(count);
    }

    fn move_backward_tabs(&mut self, count: usize) {
        self.active_block_mut().surface.move_backward_tabs(count);
    }

    fn set_active_charset_index(&mut self, index: crate::escape::CharsetIndex) {
        self.active_block_mut()
            .surface
            .set_active_charset_index(index);
    }

    fn configure_charset(
        &mut self,
        charset: crate::escape::Charset,
        index: crate::escape::CharsetIndex,
    ) {
        self.active_block_mut()
            .surface
            .configure_charset(charset, index);
    }

    fn set_color(&mut self, index: usize, color: crate::escape::Rgb) {
        self.active_block_mut().surface.set_color(index, color);
    }

    fn query_color(&mut self, index: usize) {
        self.active_block_mut().surface.query_color(index);
    }

    fn reset_color(&mut self, index: usize) {
        self.active_block_mut().surface.reset_color(index);
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: usize) {
        self.active_block_mut()
            .surface
            .set_scrolling_region(top, bottom);
    }

    fn scroll_up(&mut self, count: usize) {
        self.active_block_mut().surface.scroll_up(count);
    }

    fn scroll_down(&mut self, count: usize) {
        self.active_block_mut().surface.scroll_down(count);
    }

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

    fn set_hyperlink(&mut self, link: Option<crate::escape::Hyperlink>) {
        self.active_block_mut().surface.set_hyperlink(link);
    }

    fn sgr(&mut self, attr: crate::escape::CharacterAttribute) {
        self.active_block_mut().surface.sgr(attr);
    }

    fn set_cursor_shape(&mut self, shape: crate::escape::CursorShape) {
        self.active_block_mut().surface.set_cursor_shape(shape);
    }

    fn set_cursor_style(&mut self, style: Option<crate::escape::CursorStyle>) {
        self.active_block_mut().surface.set_cursor_style(style);
    }

    fn save_cursor(&mut self) {
        self.active_block_mut().surface.save_cursor();
    }

    fn restore_cursor(&mut self) {
        self.active_block_mut().surface.restore_cursor();
    }

    fn move_up(&mut self, rows: usize, carriage_return: bool) {
        self.active_block_mut()
            .surface
            .move_up(rows, carriage_return);
    }

    fn move_down(&mut self, rows: usize, carriage_return: bool) {
        self.active_block_mut()
            .surface
            .move_down(rows, carriage_return);
    }

    fn move_forward(&mut self, cols: usize) {
        self.active_block_mut().surface.move_forward(cols);
    }

    fn move_backward(&mut self, cols: usize) {
        self.active_block_mut().surface.move_backward(cols);
    }

    fn goto(&mut self, row: i32, col: usize) {
        self.active_block_mut().surface.goto(row, col);
    }

    fn goto_row(&mut self, row: i32) {
        self.active_block_mut().surface.goto_row(row);
    }

    fn goto_column(&mut self, col: usize) {
        self.active_block_mut().surface.goto_column(col);
    }

    fn set_keypad_application_mode(&mut self, enabled: bool) {
        self.active_block_mut()
            .surface
            .set_keypad_application_mode(enabled);
    }

    fn set_keyboard_mode(
        &mut self,
        mode: crate::escape::KeyboardMode,
        behavior: crate::escape::KeyboardModeApplyBehavior,
    ) {
        self.active_block_mut()
            .surface
            .set_keyboard_mode(mode, behavior);
    }

    fn push_keyboard_mode(&mut self, mode: crate::escape::KeyboardMode) {
        self.active_block_mut().surface.push_keyboard_mode(mode);
    }

    fn pop_keyboard_modes(&mut self, count: u16) {
        self.active_block_mut().surface.pop_keyboard_modes(count);
    }

    fn report_keyboard_mode(
        &mut self,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_keyboard_mode(report_channel);
    }

    fn push_window_title(&mut self) {
        self.active_block_mut().surface.push_window_title();
    }

    fn pop_window_title(&mut self) -> Option<String> {
        self.active_block_mut().surface.pop_window_title()
    }

    fn set_window_title(&mut self, title: Option<String>) {
        self.active_block_mut().surface.set_window_title(title);
    }

    fn deccolm(&mut self) {
        self.active_block_mut().surface.deccolm();
    }

    fn set_private_mode(&mut self, mode: crate::escape::PrivateMode) {
        self.active_block_mut().surface.set_private_mode(mode);
    }

    fn unset_private_mode(&mut self, mode: crate::escape::PrivateMode) {
        self.active_block_mut().surface.unset_private_mode(mode);
    }

    fn report_private_mode(
        &mut self,
        mode: crate::escape::PrivateMode,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_private_mode(mode, report_channel);
    }

    fn set_mode(&mut self, mode: crate::escape::Mode) {
        self.active_block_mut().surface.set_mode(mode);
    }

    fn unset_mode(&mut self, mode: crate::escape::Mode) {
        self.active_block_mut().surface.unset_mode(mode);
    }

    fn report_mode(
        &mut self,
        mode: crate::escape::Mode,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_mode(mode, report_channel);
    }

    fn identify_terminal(
        &mut self,
        attr: Option<char>,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .identify_terminal(attr, report_channel);
    }

    fn report_device_status(
        &mut self,
        status: usize,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .report_device_status(status, report_channel);
    }

    fn request_text_area_by_pixels(
        &mut self,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .request_text_area_by_pixels(report_channel);
    }

    fn request_text_area_by_chars(
        &mut self,
        report_channel: &mut std::collections::VecDeque<u8>,
    ) {
        self.active_block_mut()
            .surface
            .request_text_area_by_chars(report_channel);
    }

    fn start_selection(
        &mut self,
        ty: crate::SelectionType,
        point: crate::index::Point,
        side: crate::Side,
    ) {
        self.active_block_mut()
            .surface
            .start_selection(ty, point, side);
    }

    fn update_selection(
        &mut self,
        point: crate::index::Point,
        side: crate::Side,
    ) {
        self.active_block_mut()
            .surface
            .update_selection(point, side);
    }

    fn handle_block_event(&mut self, event: crate::escape::BlockEvent) {
        use crate::escape::{BlockKind as EscapeBlockKind, BlockPhase};

        let escape_meta = event.meta;

        let mut meta = BlockMeta {
            id: escape_meta.id,
            kind: match escape_meta.kind {
                EscapeBlockKind::Command => BlockKind::Command,
                EscapeBlockKind::Prompt => BlockKind::Prompt,
                EscapeBlockKind::FullScreen => BlockKind::FullScreen,
            },
            cmd: escape_meta.cmd,
            cwd: escape_meta.cwd,
            shell: escape_meta.shell,
            started_at: escape_meta.started_at,
            finished_at: escape_meta.finished_at,
            exit_code: escape_meta.exit_code,
            is_alt_screen: escape_meta.is_alt_screen,
            is_finished: false,
        };

        match event.phase {
            BlockPhase::Preexec => {
                meta.kind = BlockKind::Command;
                let _ = self.begin_block(meta);
            },
            BlockPhase::Exit => {
                self.end_block_by_id(&meta);
            },
            BlockPhase::Precmd => {
                meta.kind = BlockKind::Prompt;

                let exists = self.blocks.iter().any(|b| b.meta.id == meta.id);

                if exists {
                    self.end_block_by_id(&meta);
                } else {
                    let _ = self.begin_block(meta);
                }
            },
        }
    }
}

impl SurfaceModel for BlockSurface {
    fn snapshot_owned(&mut self) -> SnapshotOwned {
        if self.blocks.is_empty() {
            return SnapshotOwned::default();
        }

        self.clamp_display_offset();

        let columns = self.columns();
        let viewport_lines = self.screen_lines();
        let slices = self.block_slices();
        let total_lines = slices
            .last()
            .map(|slice| slice.end)
            .unwrap_or(viewport_lines);
        let start =
            total_lines.saturating_sub(viewport_lines + self.display_offset);
        let viewport_end = start + viewport_lines;

        let mut cells =
            Vec::with_capacity(columns.saturating_mul(viewport_lines));
        let mut slice_idx = 0;
        while slice_idx + 1 < slices.len() && slices[slice_idx].end <= start {
            slice_idx += 1;
        }

        let mut current_slice = &slices[slice_idx];
        for row in 0..viewport_lines {
            let global_index = start + row;
            while global_index >= current_slice.end
                && slice_idx + 1 < slices.len()
            {
                slice_idx += 1;
                current_slice = &slices[slice_idx];
            }

            let block_index = current_slice.index;
            let block = &self.blocks[block_index];
            let grid = block.surface.grid();
            let local_index = global_index - current_slice.start;
            let line = current_slice.top_line + local_index;

            for col in 0..columns {
                let column = Column(col);
                let cell = grid[line][column].clone();
                let point_line = row as i32 - self.display_offset as i32;
                let point = Point::new(Line(point_line), column);
                cells.push(SnapshotCell { point, cell });
            }
        }

        let active_block = &self.blocks[self.active].surface;
        let mut cursor = CursorSnapshot::new(active_block);
        let selection = active_block
            .selection
            .as_ref()
            .and_then(|s| s.to_range(active_block));

        let active_slice =
            slices.iter().find(|slice| slice.index == self.active);
        if let Some(slice) = active_slice {
            if let Some(point) =
                self.convert_point_to_view(slice, cursor.point, start)
            {
                cursor.point = point;
            }
        }

        let selection =
            if let (Some(slice), Some(range)) = (active_slice, selection) {
                self.convert_selection_to_view(range, slice, start)
            } else {
                None
            };

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

        let mut block_snapshots = Vec::with_capacity(self.blocks.len());
        for slice in &slices {
            let block = &self.blocks[slice.index];
            let visible_start = max(slice.start, start);
            let visible_end = min(slice.end, viewport_end);
            let line_count = visible_end.saturating_sub(visible_start);
            let line_offset = visible_start.saturating_sub(start);
            let start_line = line_offset as i32 - self.display_offset as i32;

            block_snapshots.push(BlockSnapshot {
                id: block.meta.id.clone(),
                meta: BlockMetaPublic::from(&block.meta),
                start_line,
                line_count,
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
        assert_eq!(surface.active, 0);
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

        let _ = surface.begin_block(meta);
        surface.print('b');

        assert_eq!(surface.blocks.len(), 2);
        assert_eq!(surface.active, 1);

        let second_snapshot = surface.snapshot_owned();
        let second_view = second_snapshot.view();
        assert!(second_view.cells.iter().any(|cell| cell.cell.c == 'b'));
        assert!(
            second_view.cells.iter().all(|cell| cell.cell.c != 'a'),
            "new block should start with an empty surface"
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

        let _ = surface.begin_block(start_meta);

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
            let _ = surface.begin_block(meta);
        }

        assert!(
            surface.blocks.len() <= 3,
            "block history should be bounded by max_blocks"
        );
        assert!(
            surface.blocks.iter().any(|b| !b.meta.is_finished),
            "unfinished blocks should not be deleted when applying max_blocks"
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
        let _ = surface.begin_block(meta);

        for _ in 0..4 {
            surface.print('B');
            surface.new_line();
        }

        let bottom_snapshot = surface.snapshot_owned();
        let bottom_view = bottom_snapshot.view();
        assert!(
            bottom_view.cells.iter().any(|cell| cell.cell.c == 'B'),
            "bottom view should display active block output"
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
            "scrolling to top should reveal the first block"
        );

        let top_blocks = top_view.blocks();
        assert_eq!(top_blocks[0].line_count, top_view.size.screen_lines);
        assert_eq!(top_blocks[1].line_count, 0);
    }
}
