use std::cmp::{max, min};

use crate::cell::Cell;
use crate::grid::{Grid, Scroll};
use crate::hyperlink::HyperlinkMap;
use crate::index::{Column, Line, Point};
use crate::selection::SelectionRange;
use crate::snapshot::{
    CursorSnapshot, SnapshotCell, SnapshotDamage, SnapshotOwned, SnapshotSize,
    SurfaceModel,
};
use crate::{Dimensions, Surface, SurfaceActor, SurfaceConfig, SurfaceMode};

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

struct ViewportContext {
    content_lines: usize,
    start: usize,
    viewport_end: usize,
    bottom_padding: usize,
    effective_start: isize,
}

#[derive(Clone, Copy)]
struct GlobalPoint {
    line_index: usize,
    column: Column,
}

#[derive(Clone, Copy)]
struct GlobalSelection {
    start: GlobalPoint,
    end: GlobalPoint,
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
    selection_block: Option<usize>,
    selection_anchor: Option<GlobalPoint>,
    global_selection: Option<GlobalSelection>,
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
            selection_block: None,
            selection_anchor: None,
            global_selection: None,
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
            .map(Self::block_visible_line_count)
            .sum::<usize>();

        max(viewport, history)
    }

    fn clamp_display_offset(&mut self) {
        let viewport_lines = self.screen_lines();
        let max_offset = self.total_lines().saturating_sub(viewport_lines);
        self.display_offset = min(self.display_offset, max_offset);
    }

    fn block_slices(&self) -> Vec<BlockSliceInfo> {
        if self.is_alt_screen_active() {
            if let Some(block) = self.blocks.get(self.active) {
                let (top_line, total_lines) = Self::block_visible_extent(block);
                return vec![BlockSliceInfo {
                    index: self.active,
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

    fn is_alt_screen_active(&self) -> bool {
        self.blocks
            .get(self.active)
            .map(|block| block.surface.mode().contains(SurfaceMode::ALT_SCREEN))
            .unwrap_or(false)
    }

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
    fn enforce_max_blocks(&mut self) {
        if self.blocks.len() <= self.max_blocks {
            return;
        }

        let mut index = 0;
        while self.blocks.len() > self.max_blocks && index < self.blocks.len() {
            if self.blocks[index].meta.is_finished && index != self.active {
                self.remove_block_at(index);
            } else {
                index += 1;
            }
        }
    }

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

        if self.active > index {
            self.active -= 1;
        } else if self.active >= self.blocks.len() {
            self.active = self.blocks.len().saturating_sub(1);
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

    fn merge_block_meta(target: &mut BlockMeta, meta: &BlockMeta) {
        if let Some(cmd) = &meta.cmd {
            target.cmd = Some(cmd.clone());
        }

        if let Some(cwd) = &meta.cwd {
            target.cwd = Some(cwd.clone());
        }

        if let Some(shell) = &meta.shell {
            target.shell = Some(shell.clone());
        }

        if let Some(started_at) = meta.started_at {
            target.started_at = Some(started_at);
        }

        if let Some(finished_at) = meta.finished_at {
            target.finished_at = Some(finished_at);
        }

        if let Some(exit_code) = meta.exit_code {
            target.exit_code = Some(exit_code);
        }

        target.kind = meta.kind.clone();
        target.is_alt_screen = meta.is_alt_screen;
    }

    fn active_prompt_index(&self) -> Option<usize> {
        self.blocks.get(self.active).and_then(|block| {
            if block.meta.kind == BlockKind::Prompt && !block.meta.is_finished {
                Some(self.active)
            } else {
                None
            }
        })
    }

    /// Обновляет метаданные и помечает блок с данным `id` как завершённым.
    pub fn end_block_by_id(&mut self, meta: &BlockMeta) {
        if let Some(block) =
            self.blocks.iter_mut().find(|b| b.meta.id == meta.id)
        {
            Self::merge_block_meta(&mut block.meta, meta);
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
        let columns = size.columns();
        let screen_lines = size.screen_lines();

        for block in &mut self.blocks {
            block.surface.resize(BlockDimensions {
                columns,
                screen_lines,
            });
        }

        self.clamp_display_offset();
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

        if let Some(block_index) = self.selection_block {
            if let Some((index, local_point, global_index)) = resolved {
                if self.global_selection.is_none() && index == block_index {
                    if let Some(block) = self.blocks.get_mut(index) {
                        block.surface.update_selection(local_point, side);
                        return;
                    }
                } else if self.global_selection.is_none() {
                    if let Some(block) = self.blocks.get_mut(block_index) {
                        block.surface.selection = None;
                    }
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
                }
            } else {
                return;
            }
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
                if let Some(prompt_idx) = self.active_prompt_index() {
                    self.blocks[prompt_idx].meta = meta;
                    self.active = prompt_idx;
                    self.display_offset = 0;
                    self.clamp_display_offset();
                    return;
                }

                let _ = self.begin_block(meta);
            },
            BlockPhase::Exit => {
                self.end_block_by_id(&meta);
            },
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

                let _ = self.begin_block(meta);
            },
        }
    }
}

impl SurfaceModel for BlockSurface {
    fn snapshot_owned(&mut self) -> SnapshotOwned {
        if self.blocks.is_empty() {
            return SnapshotOwned::default();
        }

        if self.is_alt_screen_active() {
            return self.snapshot_active_alt_screen_block();
        }

        self.clamp_display_offset();

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

        let active_block = &self.blocks[self.active].surface;
        let mut cursor = CursorSnapshot::new(active_block);
        let active_slice =
            slices.iter().find(|slice| slice.index == self.active);
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

impl BlockSurface {
    fn snapshot_active_alt_screen_block(&mut self) -> SnapshotOwned {
        self.display_offset = 0;
        let Some(block) = self.blocks.get_mut(self.active) else {
            return SnapshotOwned::default();
        };

        block.meta.is_alt_screen =
            block.surface.mode().contains(SurfaceMode::ALT_SCREEN);

        let mut snapshot = SnapshotOwned::from_surface(&mut block.surface);
        let line_count = snapshot.view().size.screen_lines;
        snapshot.blocks = vec![BlockSnapshot {
            id: block.meta.id.clone(),
            meta: BlockMetaPublic::from(&block.meta),
            start_line: 0,
            line_count,
            is_alt_screen: block.meta.is_alt_screen,
        }];

        snapshot
    }

    fn block_visible_line_count(block: &Block) -> usize {
        Self::block_visible_extent(block).1
    }

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
            let index = self.active.min(self.blocks.len().saturating_sub(1));
            let block = &self.blocks[index];
            let column = min(
                point.column,
                Column(block.surface.columns().saturating_sub(1)),
            );
            return Some((index, Point::new(Line(0), column), 0));
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

    fn global_to_view_point(
        &self,
        context: &ViewportContext,
        global_line: usize,
        column: Column,
        viewport_lines: usize,
    ) -> Option<Point> {
        if context.viewport_end <= context.start {
            return None;
        }
        let viewport_span = context.viewport_end - context.start;
        let relative_line = global_line.checked_sub(context.start)?;
        if relative_line >= viewport_span {
            return None;
        }
        if relative_line >= viewport_lines {
            return None;
        }
        let line = relative_line as i32 - self.display_offset as i32;
        Some(Point::new(Line(line), column))
    }

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

        Some(SelectionRange::new(start_point, end_point, false))
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
        let blocks = second_view.blocks();
        assert_eq!(blocks.len(), 2);
        assert!(
            blocks.last().is_some_and(|block| block.line_count > 0),
            "new block should contribute its own surface lines"
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

        let _ = surface.begin_block(meta);
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
            "first block should never expand when scrolled to the bottom"
        );
        assert_eq!(bottom_blocks[1].line_count, 1);
        assert_eq!(
            bottom_blocks[1].start_line,
            bottom_view.size.screen_lines as i32 - 1
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

        let _ = surface.begin_block(meta);
        surface.print('B');

        surface.resize(TestDimensions::new(6, 4));

        for block in &surface.blocks {
            let grid = block.surface.grid();
            assert_eq!(grid.columns(), 6);
            assert_eq!(grid.screen_lines(), 4);
        }
    }
}
