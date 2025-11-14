use cursor_icon::CursorIcon;
use log::{debug, trace, warn};
use otty_escape::{
    CharacterAttribute, Charset, CharsetIndex, ClearMode, Hyperlink,
    LineClearMode, Mode, NamedMode, NamedPrivateMode, PrivateMode, Rgb,
    TabClearMode,
};
use std::sync::Arc;
use unicode_width::UnicodeWidthChar;

use crate::{
    SurfaceActor,
    cell::{Cell, CellAttributes, CellBlink, CellUnderline},
    grid::Grid,
    state::{
        CursorSnapshot, LineDamage, SurfaceDamage, SurfacePalette,
        SurfaceSnapshot,
    },
};

const DEFAULT_COLUMNS: usize = 80;
const DEFAULT_ROWS: usize = 24;
const DEFAULT_SCROLL_LIMIT: usize = 1000;
const TAB_WIDTH: usize = 8;

fn default_charsets() -> [Charset; 4] {
    [Charset::Ascii; 4]
}

fn default_tab_stops(width: usize) -> Vec<bool> {
    let mut stops = vec![false; width];
    for col in (TAB_WIDTH..width).step_by(TAB_WIDTH) {
        stops[col] = true;
    }
    stops
}

#[derive(Debug, Clone)]
struct DamageTracker {
    full: bool,
    width: usize,
    lines: Vec<LineDamage>,
}

impl DamageTracker {
    fn new(columns: usize, rows: usize) -> Self {
        let mut lines = Vec::with_capacity(rows);
        for row in 0..rows {
            lines.push(LineDamage::undamaged(row, columns));
        }
        Self {
            full: true,
            width: columns,
            lines,
        }
    }

    fn resize(&mut self, columns: usize, rows: usize) {
        self.width = columns;

        if rows > self.lines.len() {
            for row in self.lines.len()..rows {
                self.lines.push(LineDamage::undamaged(row, columns));
            }
        } else if rows < self.lines.len() {
            self.lines.truncate(rows);
        }

        for (row_idx, entry) in self.lines.iter_mut().enumerate() {
            entry.row = row_idx;
            entry.reset(columns);
        }

        self.full = true;
    }

    fn mark_full(&mut self) {
        self.full = true;
    }

    fn damage_rows(&mut self, start: usize, end: usize) {
        if self.width == 0 || start > end || self.lines.is_empty() {
            return;
        }
        let max_col = self.width.saturating_sub(1);
        let end = end.min(self.lines.len().saturating_sub(1));
        for row in start..=end {
            self.damage_range(row, 0, max_col);
        }
    }

    fn damage_cell(&mut self, row: usize, col: usize) {
        self.damage_range(row, col, col);
    }

    fn damage_range(&mut self, row: usize, left: usize, right: usize) {
        if self.full || self.width == 0 || row >= self.lines.len() {
            return;
        }
        let max_col = self.width.saturating_sub(1);
        let start = left.min(max_col);
        let end = right.min(max_col);
        if start > end {
            return;
        }
        self.lines[row].include(start, end);
    }

    fn take(&mut self) -> SurfaceDamage {
        if self.full {
            self.full = false;
            for (row_idx, entry) in self.lines.iter_mut().enumerate() {
                entry.row = row_idx;
                entry.reset(self.width);
            }
            return SurfaceDamage::Full;
        }

        let mut damaged = Vec::new();
        for (row_idx, entry) in self.lines.iter_mut().enumerate() {
            entry.row = row_idx;
            if entry.is_damaged() {
                damaged.push(*entry);
                entry.reset(self.width);
            }
        }

        if damaged.is_empty() {
            SurfaceDamage::None
        } else {
            SurfaceDamage::Partial(damaged)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SurfaceConfig {
    pub columns: usize,
    pub rows: usize,
    pub max_scroll_limit: usize,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            columns: DEFAULT_COLUMNS,
            rows: DEFAULT_ROWS,
            max_scroll_limit: DEFAULT_SCROLL_LIMIT,
        }
    }
}

#[derive(Debug, Default)]
struct Osc133Tracker {
    _placeholder: Option<String>,
}

#[derive(Debug, Clone)]
struct ScreenState {
    grid: Grid,
    cursor_row: usize,
    cursor_col: usize,
    saved_cursor: Option<CursorSnapshot>,
    tab_stops: Vec<bool>,
    scroll_top: usize,
    scroll_bottom: usize,
    insert_mode: bool,
    linefeed_newline: bool,
    autowrap: bool,
    origin_mode: bool,
    wrap_pending: bool,
    current_attributes: CellAttributes,
    palette: SurfacePalette,
    cursor_icon: Option<CursorIcon>,
    cursor_shape: Option<otty_escape::CursorShape>,
    cursor_style: Option<otty_escape::CursorStyle>,
    keypad_application_mode: bool,
    keyboard_stack_depth: u16,
    sync_depth: u32,
    window_title_stack: Vec<String>,
    window_title: Option<String>,
    charsets: [Charset; 4],
    active_charset: CharsetIndex,
}

impl ScreenState {
    fn from_surface(surface: &Surface) -> Self {
        Self {
            grid: surface.grid.clone(),
            cursor_row: surface.cursor_row,
            cursor_col: surface.cursor_col,
            saved_cursor: surface.saved_cursor.clone(),
            tab_stops: surface.tab_stops.clone(),
            scroll_top: surface.scroll_top,
            scroll_bottom: surface.scroll_bottom,
            insert_mode: surface.insert_mode,
            linefeed_newline: surface.linefeed_newline,
            autowrap: surface.autowrap,
            origin_mode: surface.origin_mode,
            wrap_pending: surface.wrap_pending,
            current_attributes: surface.current_attributes.clone(),
            palette: surface.palette.clone(),
            cursor_icon: surface.cursor_icon,
            cursor_shape: surface.cursor_shape,
            cursor_style: surface.cursor_style,
            keypad_application_mode: surface.keypad_application_mode,
            keyboard_stack_depth: surface.keyboard_stack_depth,
            sync_depth: surface.sync_depth,
            window_title_stack: surface.window_title_stack.clone(),
            window_title: surface.window_title.clone(),
            charsets: surface.charsets,
            active_charset: surface.active_charset,
        }
    }

    fn apply(self, surface: &mut Surface) {
        surface.grid = self.grid;
        surface.cursor_row = self.cursor_row;
        surface.cursor_col = self.cursor_col;
        surface.saved_cursor = self.saved_cursor;
        surface.tab_stops = self.tab_stops;
        surface.scroll_top = self.scroll_top;
        surface.scroll_bottom = self.scroll_bottom;
        surface.insert_mode = self.insert_mode;
        surface.linefeed_newline = self.linefeed_newline;
        surface.autowrap = self.autowrap;
        surface.origin_mode = self.origin_mode;
        surface.wrap_pending = self.wrap_pending;
        surface.current_attributes = self.current_attributes;
        surface.palette = self.palette;
        surface.cursor_icon = self.cursor_icon;
        surface.cursor_shape = self.cursor_shape;
        surface.cursor_style = self.cursor_style;
        surface.keypad_application_mode = self.keypad_application_mode;
        surface.keyboard_stack_depth = self.keyboard_stack_depth;
        surface.sync_depth = self.sync_depth;
        surface.window_title_stack = self.window_title_stack;
        surface.window_title = self.window_title;
        surface.charsets = self.charsets;
        surface.active_charset = self.active_charset;
    }

    fn resize(
        &mut self,
        columns: usize,
        rows: usize,
        template: &CellAttributes,
    ) {
        self.grid.resize(columns, rows, template);
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
        let max_col = columns.saturating_sub(1);
        let max_row = rows.saturating_sub(1);
        self.cursor_row = self.cursor_row.min(max_row);
        self.cursor_col = self.cursor_col.min(max_col);
        if let Some(snapshot) = &mut self.saved_cursor {
            snapshot.row = snapshot.row.min(max_row);
            snapshot.col = snapshot.col.min(max_col);
        }
        self.tab_stops = default_tab_stops(columns);
    }
}

#[derive(Debug)]
pub struct Surface {
    grid: Grid,
    cursor_row: usize,
    cursor_col: usize,
    saved_cursor: Option<CursorSnapshot>,
    tab_stops: Vec<bool>,
    scroll_top: usize,
    scroll_bottom: usize,
    insert_mode: bool,
    linefeed_newline: bool,
    autowrap: bool,
    origin_mode: bool,
    wrap_pending: bool,
    default_attributes: CellAttributes,
    current_attributes: CellAttributes,
    palette: SurfacePalette,
    cursor_icon: Option<CursorIcon>,
    cursor_shape: Option<otty_escape::CursorShape>,
    cursor_style: Option<otty_escape::CursorStyle>,
    keypad_application_mode: bool,
    keyboard_stack_depth: u16,
    sync_depth: u32,
    _osc133_tracker: Osc133Tracker,
    window_title_stack: Vec<String>,
    window_title: Option<String>,
    primary_screen: Option<ScreenState>,
    charsets: [Charset; 4],
    active_charset: CharsetIndex,
    damage: DamageTracker,
}

impl Surface {
    pub fn new(config: SurfaceConfig) -> Self {
        let columns = config.columns.max(1);
        let rows = config.rows.max(1);
        let max_scroll_limit = config.max_scroll_limit;
        let default_attributes = CellAttributes::default();
        let mut surface = Self {
            grid: Grid::new(
                rows,
                columns,
                max_scroll_limit,
                &default_attributes,
            ),
            cursor_row: 0,
            cursor_col: 0,
            saved_cursor: None,
            tab_stops: Vec::new(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            insert_mode: false,
            linefeed_newline: false,
            autowrap: true,
            origin_mode: false,
            wrap_pending: false,
            default_attributes: default_attributes.clone(),
            current_attributes: default_attributes.clone(),
            palette: SurfacePalette::default(),
            cursor_icon: None,
            cursor_shape: None,
            cursor_style: None,
            keypad_application_mode: false,
            keyboard_stack_depth: 0,
            sync_depth: 0,
            _osc133_tracker: Osc133Tracker::default(),
            window_title_stack: Vec::new(),
            window_title: None,
            primary_screen: None,
            charsets: default_charsets(),
            active_charset: CharsetIndex::G0,
            damage: DamageTracker::new(columns, rows),
        };
        surface.reset_tab_stops();
        surface
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn columns(&self) -> usize {
        self.grid.width()
    }

    pub fn rows(&self) -> usize {
        self.grid.height()
    }

    pub fn cursor_icon(&self) -> Option<CursorIcon> {
        self.cursor_icon
    }

    pub fn cursor_shape(&self) -> Option<otty_escape::CursorShape> {
        self.cursor_shape
    }

    pub fn cursor_style(&self) -> Option<otty_escape::CursorStyle> {
        self.cursor_style
    }

    fn mark_full_dirty(&mut self) {
        self.damage.mark_full();
    }

    fn mark_row_dirty(&mut self, row: usize, left: usize, right: usize) {
        self.damage.damage_range(row, left, right);
    }

    fn mark_rows_dirty(&mut self, start: usize, end: usize) {
        self.damage.damage_rows(start, end);
    }

    fn mark_cell_dirty(&mut self, row: usize, col: usize) {
        self.damage.damage_cell(row, col);
    }

    /// Get the number of lines in scrollback history.
    pub fn history_size(&self) -> usize {
        self.grid.history_size()
    }

    /// Get the current display offset (how far scrolled back).
    pub fn display_offset(&self) -> usize {
        self.grid.display_offset()
    }

    /// Scroll the display viewport through history.
    pub fn scroll_display(&mut self, direction: crate::grid::ScrollDirection) {
        self.grid.scroll_display(direction);
        self.damage.mark_full();
    }

    pub fn snapshot(&mut self) -> SurfaceSnapshot {
        let damage = self.damage.take();
        SurfaceSnapshot::new(
            Arc::new(self.grid.clone()),
            self.grid.width(),
            self.grid.height(),
            self.cursor_row,
            self.cursor_col,
            self.grid.display_offset(),
            self.cursor_icon,
            self.cursor_shape,
            self.cursor_style,
            damage,
        )
    }

    fn reset_state(&mut self) {
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.saved_cursor = None;
        self.scroll_top = 0;
        self.scroll_bottom = self.grid.height().saturating_sub(1);
        self.insert_mode = false;
        self.linefeed_newline = false;
        self.autowrap = true;
        self.origin_mode = false;
        self.wrap_pending = false;
        self.current_attributes = self.default_attributes.clone();
        self.palette = SurfacePalette::default();
        self.reset_tab_stops();
        self.window_title_stack.clear();
        self.window_title = None;
        self.charsets = default_charsets();
        self.active_charset = CharsetIndex::G0;
    }

    fn clamp_cursor(&mut self) {
        self.cursor_row =
            self.cursor_row.min(self.grid.height().saturating_sub(1));
        self.cursor_col =
            self.cursor_col.min(self.grid.width().saturating_sub(1));
    }

    fn reset_tab_stops(&mut self) {
        self.tab_stops = default_tab_stops(self.grid.width());
    }

    fn set_tab_stop(&mut self, col: usize) {
        if col >= self.tab_stops.len() {
            self.tab_stops.resize(col + 1, false);
        }
        self.tab_stops[col] = true;
    }

    fn clear_tab_stop(&mut self, col: usize) {
        if col < self.tab_stops.len() {
            self.tab_stops[col] = false;
        }
    }

    fn clear_all_tab_stops(&mut self) {
        for tab in &mut self.tab_stops {
            *tab = false;
        }
    }

    fn next_tab_stop(&self, mut col: usize, count: usize) -> usize {
        let mut remaining = count;

        while remaining > 0 && col + 1 < self.grid.width() {
            col += 1;
            if col < self.tab_stops.len() && self.tab_stops[col] {
                remaining -= 1;
            }
        }

        col
    }

    fn previous_tab_stop(&self, mut col: usize, count: usize) -> usize {
        let mut remaining = count;

        while remaining > 0 && col > 0 {
            col -= 1;
            if col < self.tab_stops.len() && self.tab_stops[col] {
                remaining -= 1;
            }
        }

        col
    }

    fn charset_slot(index: CharsetIndex) -> usize {
        match index {
            CharsetIndex::G0 => 0,
            CharsetIndex::G1 => 1,
            CharsetIndex::G2 => 2,
            CharsetIndex::G3 => 3,
        }
    }

    fn active_charset(&self) -> Charset {
        self.charsets[Self::charset_slot(self.active_charset)]
    }

    fn move_cursor_to(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.grid.height().saturating_sub(1));
        self.cursor_col = col.min(self.grid.width().saturating_sub(1));
        self.wrap_pending = false;
    }

    fn cursor_home(&mut self) {
        let row = if self.origin_mode { self.scroll_top } else { 0 };
        self.move_cursor_to(row, 0);
    }

    fn scroll_region_limits(&self) -> (usize, usize) {
        (self.scroll_top, self.scroll_bottom)
    }

    fn advance_line(&mut self, from_wrap: bool) {
        if self.cursor_row < self.grid.height() {
            self.grid.row_mut(self.cursor_row).set_soft_wrap(from_wrap);
        }

        let (_, bottom) = self.scroll_region_limits();

        if self.cursor_row == bottom {
            self.grid.scroll_up(
                self.scroll_top,
                bottom,
                1,
                &self.default_attributes,
            );
            self.mark_full_dirty();
            // Reset display offset when new content arrives (snap to bottom).
            if self.grid.display_offset() > 0 {
                debug!(
                    "[surface] line_feed: forcing scroll to Bottom, offset was {}",
                    self.grid.display_offset()
                );
                self.grid
                    .scroll_display(crate::grid::ScrollDirection::Bottom);
                self.mark_full_dirty();
            }
        } else {
            self.cursor_row = (self.cursor_row + 1).min(bottom);
        }

        if self.linefeed_newline {
            self.cursor_col = 0;
        }
        self.wrap_pending = false;

        if self.cursor_row < self.grid.height() {
            self.grid.row_mut(self.cursor_row).set_soft_wrap(false);
        }
    }

    fn put_zero_width_char(&mut self, ch: char) {
        let rows = self.grid.height();
        let columns = self.grid.width();
        if rows == 0 || columns == 0 {
            return;
        }

        let row_idx = self.cursor_row.min(rows.saturating_sub(1));
        let mut col_idx = self.cursor_col.min(columns.saturating_sub(1));

        if !self.wrap_pending {
            if col_idx == 0 {
                return;
            }
            col_idx = col_idx.saturating_sub(1);
        }

        {
            let row = self.grid.row(row_idx);
            if col_idx >= row.cells.len() {
                return;
            }
            if row.cells[col_idx].is_wide_trailing() {
                if col_idx == 0 {
                    return;
                }
                col_idx = col_idx.saturating_sub(1);
            }
        }

        {
            let row_len = self.grid.row(row_idx).cells.len();
            if col_idx >= row_len {
                return;
            }
        }

        let row = self.grid.row_mut(row_idx);
        if col_idx < row.cells.len() {
            row.cells[col_idx].push_zero_width(ch);
            self.mark_cell_dirty(row_idx, col_idx);
        }
    }

    fn clear_wide_cell(&mut self, row: usize, col: usize) {
        let rows = self.grid.height();
        let columns = self.grid.width();

        if rows == 0 || columns == 0 || row >= rows {
            return;
        }

        let (is_trailing, is_leading, row_len) = {
            let row_ref = self.grid.row(row);
            let len = row_ref.cells.len();
            if col >= len {
                return;
            }
            (
                row_ref.cells[col].is_wide_trailing(),
                row_ref.cells[col].is_wide_leading(),
                len,
            )
        };

        let template = &self.default_attributes;
        let row_mut = self.grid.row_mut(row);

        if is_trailing {
            if col > 0 && col - 1 < row_len {
                row_mut.cells[col - 1] = Cell::blank(template);
            }
            row_mut.cells[col] = Cell::blank(template);
        } else if is_leading {
            row_mut.cells[col] = Cell::blank(template);
            if col + 1 < row_len {
                row_mut.cells[col + 1] = Cell::blank(template);
            }
        } else {
            row_mut.cells[col].clear_zero_width();
        }

        let max_col = self.grid.width().saturating_sub(1);
        let start = col.saturating_sub(1).min(max_col);
        let end = col.saturating_add(1).min(max_col);
        self.mark_row_dirty(row, start, end);
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: usize) {
        let height = self.grid.height();
        if top >= bottom || bottom >= height {
            self.scroll_top = 0;
            self.scroll_bottom = height.saturating_sub(1);
        } else {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
        self.cursor_home();
    }

    fn insert_blank_lines(&mut self, count: usize) {
        let count =
            count.min(self.scroll_bottom.saturating_sub(self.scroll_top) + 1);
        self.grid.scroll_down(
            self.cursor_row,
            self.scroll_bottom,
            count,
            &self.default_attributes,
        );
        self.mark_full_dirty();
    }

    fn delete_lines(&mut self, count: usize) {
        let count =
            count.min(self.scroll_bottom.saturating_sub(self.scroll_top) + 1);
        self.grid.scroll_up(
            self.cursor_row,
            self.scroll_bottom,
            count,
            &self.default_attributes,
        );
        self.mark_full_dirty();
    }
}

impl Default for Surface {
    fn default() -> Self {
        Self::new(SurfaceConfig::default())
    }
}

impl SurfaceActor for Surface {
    fn print(&mut self, ch: char) {
        let ch = self.active_charset().map(ch);
        let width = match ch.width() {
            Some(width) => width,
            None => return,
        };

        if width == 0 {
            self.put_zero_width_char(ch);
            return;
        }

        if self.wrap_pending {
            self.cursor_col = 0;
            self.advance_line(true);
        }

        let columns = self.grid.width();
        let rows = self.grid.height();
        if columns == 0 || rows == 0 {
            return;
        }

        if width == 2 && (self.cursor_col + 1 >= columns) {
            if self.autowrap {
                self.cursor_col = 0;
                self.advance_line(true);
            } else {
                self.wrap_pending = true;
                return;
            }
        }

        if self.cursor_row >= self.grid.height() {
            return;
        }

        if self.insert_mode {
            self.grid.insert_blank_cells(
                self.cursor_row,
                self.cursor_col,
                width,
                &self.current_attributes,
            );
        }

        if self.cursor_col >= self.grid.width() {
            self.cursor_col = self.grid.width().saturating_sub(1);
        }

        self.clear_wide_cell(self.cursor_row, self.cursor_col);

        let draw_col = self.cursor_col;

        if self.cursor_row < self.grid.height()
            && self.cursor_col < self.grid.width()
        {
            if width == 1 {
                let row_len = {
                    let row = self.grid.row_mut(self.cursor_row);
                    let len = row.cells.len();
                    if self.cursor_col < len {
                        row.cells[self.cursor_col] =
                            Cell::with_char(ch, &self.current_attributes);
                    }
                    len
                };
                debug_assert!(self.cursor_col < row_len);
            } else {
                let columns = self.grid.width();
                {
                    let row = self.grid.row_mut(self.cursor_row);
                    let len = row.cells.len();
                    if self.cursor_col < len {
                        row.cells[self.cursor_col] =
                            Cell::with_char(ch, &self.current_attributes);
                        row.cells[self.cursor_col].wide_leading = true;
                        row.cells[self.cursor_col].wide_trailing = false;
                    }
                    if self.cursor_col + 1 < len {
                        row.cells[self.cursor_col + 1] =
                            Cell::blank(&self.current_attributes);
                        row.cells[self.cursor_col + 1].wide_trailing = true;
                        row.cells[self.cursor_col + 1].wide_leading = false;
                        row.cells[self.cursor_col + 1].touched = true;
                    }
                }
                if self.cursor_col + 1 < columns {
                    self.cursor_col += 1;
                }
            }
        }

        let max_col = self.grid.width().saturating_sub(1);
        let end_col = draw_col
            .saturating_add(width.saturating_sub(1))
            .min(max_col);
        self.mark_row_dirty(self.cursor_row, draw_col, end_col);

        if self.cursor_col + 1 < self.grid.width() {
            self.cursor_col += 1;
        } else {
            self.wrap_pending = self.autowrap;
        }
    }

    fn resize(&mut self, columns: usize, rows: usize) {
        let columns = columns.max(1);
        let rows = rows.max(1);
        self.grid.resize(columns, rows, &self.default_attributes);
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
        self.clamp_cursor();
        self.reset_tab_stops();
        if let Some(primary) = self.primary_screen.as_mut() {
            primary.resize(columns, rows, &self.default_attributes);
        }
        self.damage.resize(columns, rows);
        self.mark_full_dirty();
    }

    fn insert_blank(&mut self, count: usize) {
        self.grid.insert_blank_cells(
            self.cursor_row,
            self.cursor_col,
            count,
            &self.current_attributes,
        );
        let max_col = self.grid.width().saturating_sub(1);
        self.mark_row_dirty(self.cursor_row, self.cursor_col, max_col);
    }

    fn insert_blank_lines(&mut self, count: usize) {
        self.insert_blank_lines(count);
        self.mark_rows_dirty(self.cursor_row, self.scroll_bottom);
    }

    fn delete_lines(&mut self, count: usize) {
        self.delete_lines(count);
        self.mark_rows_dirty(self.cursor_row, self.scroll_bottom);
    }

    fn delete_chars(&mut self, count: usize) {
        self.grid.delete_cells(
            self.cursor_row,
            self.cursor_col,
            count,
            &self.current_attributes,
        );
        let max_col = self.grid.width().saturating_sub(1);
        self.mark_row_dirty(self.cursor_row, self.cursor_col, max_col);
    }

    fn erase_chars(&mut self, count: usize) {
        let end = self.cursor_col.saturating_add(count.saturating_sub(1));
        self.grid.clear_range(
            self.cursor_row,
            self.cursor_col,
            end,
            &self.current_attributes,
        );
        self.mark_row_dirty(self.cursor_row, self.cursor_col, end);
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
        self.wrap_pending = false;
    }

    fn carriage_return(&mut self) {
        self.cursor_col = 0;
        self.wrap_pending = false;
    }

    fn line_feed(&mut self) {
        self.advance_line(false);
    }

    fn new_line(&mut self) {
        self.advance_line(false);
        self.cursor_col = 0;
    }

    fn set_horizontal_tab(&mut self) {
        self.set_tab_stop(self.cursor_col);
    }

    fn reverse_index(&mut self) {
        if self.cursor_row == self.scroll_top {
            self.grid.scroll_down(
                self.scroll_top,
                self.scroll_bottom,
                1,
                &self.default_attributes,
            );
            self.mark_rows_dirty(self.scroll_top, self.scroll_bottom);
        } else {
            self.cursor_row = self.cursor_row.saturating_sub(1);
        }
        self.wrap_pending = false;
    }

    fn reset(&mut self) {
        self.reset_state();
        self.grid.clear(&self.default_attributes);
        self.mark_full_dirty();
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        match mode {
            ClearMode::All | ClearMode::Saved => {
                self.grid.clear(&self.current_attributes);
                self.move_cursor_to(0, 0);
            },
            ClearMode::Below => {
                self.grid.clear_range(
                    self.cursor_row,
                    self.cursor_col,
                    self.grid.width().saturating_sub(1),
                    &self.current_attributes,
                );
                for row in (self.cursor_row + 1)..self.grid.height() {
                    self.grid.row_mut(row).clear(&self.current_attributes);
                }
            },
            ClearMode::Above => {
                for row in 0..self.cursor_row {
                    self.grid.row_mut(row).clear(&self.current_attributes);
                }
                self.grid.clear_range(
                    self.cursor_row,
                    0,
                    self.cursor_col,
                    &self.current_attributes,
                );
            },
        }
        self.mark_full_dirty();
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        match mode {
            LineClearMode::All => {
                self.grid
                    .row_mut(self.cursor_row)
                    .clear(&self.current_attributes);
            },
            LineClearMode::Right => {
                self.grid.clear_range(
                    self.cursor_row,
                    self.cursor_col,
                    self.grid.width().saturating_sub(1),
                    &self.current_attributes,
                );
            },
            LineClearMode::Left => {
                self.grid.clear_range(
                    self.cursor_row,
                    0,
                    self.cursor_col,
                    &self.current_attributes,
                );
            },
        }
        let max_col = self.grid.width().saturating_sub(1);
        self.mark_row_dirty(self.cursor_row, 0, max_col);
    }

    fn insert_tabs(&mut self, count: usize) {
        let col = self.next_tab_stop(self.cursor_col, count);
        self.cursor_col = col;
        self.wrap_pending = false;
    }

    fn set_tabs(&mut self, mask: u16) {
        for bit in 0..16 {
            if (mask & (1 << bit)) != 0 {
                self.set_tab_stop(self.cursor_col + bit as usize);
            }
        }
    }

    fn clear_tabs(&mut self, mode: TabClearMode) {
        match mode {
            TabClearMode::Current => self.clear_tab_stop(self.cursor_col),
            TabClearMode::All => self.clear_all_tab_stops(),
        }
    }

    fn screen_alignment_display(&mut self) {
        for row in 0..self.grid.height() {
            for col in 0..self.grid.width() {
                self.grid.row_mut(row).cells[col] =
                    Cell::with_char('E', &self.default_attributes);
            }
        }
        self.cursor_home();
        self.mark_full_dirty();
    }

    fn move_forward_tabs(&mut self, count: usize) {
        let col = self.next_tab_stop(self.cursor_col, count);
        self.cursor_col = col;
    }

    fn move_backward_tabs(&mut self, count: usize) {
        let col = self.previous_tab_stop(self.cursor_col, count);
        self.cursor_col = col;
    }

    fn set_active_charset_index(&mut self, index: CharsetIndex) {
        self.active_charset = index;
    }

    fn configure_charset(&mut self, charset: Charset, index: CharsetIndex) {
        let slot = Self::charset_slot(index);
        self.charsets[slot] = charset;
    }

    fn set_color(&mut self, index: usize, color: Rgb) {
        self.palette.set(index, color);
    }

    fn query_color(&mut self, index: usize) {
        debug!("Query color {}", index);
    }

    fn reset_color(&mut self, index: usize) {
        self.palette.reset(index);
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: usize) {
        let top = top.saturating_sub(1);
        let bottom = bottom.saturating_sub(1);
        self.set_scrolling_region(top, bottom);
    }

    fn scroll_up(&mut self, count: usize) {
        self.grid.scroll_up(
            self.scroll_top,
            self.scroll_bottom,
            count,
            &self.default_attributes,
        );
        self.mark_rows_dirty(self.scroll_top, self.scroll_bottom);
    }

    fn scroll_down(&mut self, count: usize) {
        self.grid.scroll_down(
            self.scroll_top,
            self.scroll_bottom,
            count,
            &self.default_attributes,
        );
        self.mark_rows_dirty(self.scroll_top, self.scroll_bottom);
    }

    fn set_hyperlink(&mut self, link: Option<Hyperlink>) {
        self.current_attributes.set_hyperlink(link);
    }

    fn sgr(&mut self, attribute: CharacterAttribute) {
        use CharacterAttribute::*;
        match attribute {
            Reset => self.current_attributes = self.default_attributes.clone(),
            Bold => self.current_attributes.bold = true,
            Dim => self.current_attributes.dim = true,
            Italic => self.current_attributes.italic = true,
            Underline => {
                self.current_attributes.underline = CellUnderline::Single
            },
            DoubleUnderline => {
                self.current_attributes.underline = CellUnderline::Double
            },
            Undercurl => {
                self.current_attributes.underline = CellUnderline::Curl
            },
            DottedUnderline => {
                self.current_attributes.underline = CellUnderline::Dotted
            },
            DashedUnderline => {
                self.current_attributes.underline = CellUnderline::Dashed
            },
            BlinkSlow => self.current_attributes.blink = CellBlink::Slow,
            BlinkFast => self.current_attributes.blink = CellBlink::Fast,
            Reverse => self.current_attributes.reverse = true,
            Hidden => self.current_attributes.hidden = true,
            Strike => self.current_attributes.strike = true,
            CancelBold => self.current_attributes.bold = false,
            CancelBoldDim => {
                self.current_attributes.bold = false;
                self.current_attributes.dim = false;
            },
            CancelItalic => self.current_attributes.italic = false,
            CancelUnderline => {
                self.current_attributes.underline = CellUnderline::None
            },
            CancelBlink => self.current_attributes.blink = CellBlink::None,
            CancelReverse => self.current_attributes.reverse = false,
            CancelHidden => self.current_attributes.hidden = false,
            CancelStrike => self.current_attributes.strike = false,
            Foreground(color) => self.current_attributes.foreground = color,
            Background(color) => self.current_attributes.background = color,
            UnderlineColor(color) => {
                self.current_attributes.underline_color = color
            },
        }
    }

    fn set_cursor_shape(&mut self, shape: otty_escape::CursorShape) {
        self.cursor_shape = Some(shape);
    }

    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        self.cursor_icon = Some(icon);
    }

    fn set_cursor_style(&mut self, style: Option<otty_escape::CursorStyle>) {
        self.cursor_style = style;
    }

    fn save_cursor(&mut self) {
        self.saved_cursor = Some(CursorSnapshot::new(
            self.cursor_row,
            self.cursor_col,
            self.current_attributes.clone(),
            self.charsets,
            self.active_charset,
        ));
    }

    fn restore_cursor(&mut self) {
        if let Some(snapshot) = self.saved_cursor.clone() {
            self.cursor_row = snapshot.row;
            self.cursor_col = snapshot.col;
            self.current_attributes = snapshot.attributes;
            self.charsets = snapshot.charsets;
            self.active_charset = snapshot.active_charset;
        }
    }

    fn move_up(&mut self, rows: usize, carriage_return: bool) {
        self.cursor_row = self.cursor_row.saturating_sub(rows);
        if carriage_return {
            self.cursor_col = 0;
        }
        self.clamp_cursor();
    }

    fn move_down(&mut self, rows: usize, carriage_return: bool) {
        self.cursor_row = self.cursor_row.saturating_add(rows);
        if carriage_return {
            self.cursor_col = 0;
        }
        self.clamp_cursor();
    }

    fn move_forward(&mut self, cols: usize) {
        self.cursor_col = self
            .cursor_col
            .saturating_add(cols)
            .min(self.grid.width().saturating_sub(1));
        self.wrap_pending = false;
    }

    fn move_backward(&mut self, cols: usize) {
        self.cursor_col = self.cursor_col.saturating_sub(cols);
        self.wrap_pending = false;
    }

    fn goto(&mut self, row: i32, col: usize) {
        let row = if row <= 0 { 0 } else { row as usize - 1 };
        let col = col.saturating_sub(1);
        let base_row = if self.origin_mode { self.scroll_top } else { 0 };
        self.move_cursor_to(base_row + row, col);
    }

    fn goto_row(&mut self, row: i32) {
        let row = if row <= 0 { 0 } else { row as usize - 1 };
        let base_row = if self.origin_mode { self.scroll_top } else { 0 };
        self.cursor_row =
            (base_row + row).min(self.grid.height().saturating_sub(1));
        self.wrap_pending = false;
    }

    fn goto_column(&mut self, col: usize) {
        let col = if col == 0 { 0 } else { col - 1 };
        self.cursor_col = col.min(self.grid.width().saturating_sub(1));
        self.wrap_pending = false;
    }

    fn set_keypad_application_mode(&mut self, enabled: bool) {
        self.keypad_application_mode = enabled;
    }

    fn push_keyboard_mode(&mut self) {
        self.keyboard_stack_depth = self.keyboard_stack_depth.saturating_add(1);
    }

    fn pop_keyboard_modes(&mut self, amount: u16) {
        self.keyboard_stack_depth =
            self.keyboard_stack_depth.saturating_sub(amount);
    }

    fn set_mode(&mut self, mode: Mode, enabled: bool) {
        if let Mode::Named(named) = mode {
            match named {
                NamedMode::Insert => self.insert_mode = enabled,
                NamedMode::LineFeedNewLine => self.linefeed_newline = enabled,
            }
        } else {
            trace!("Unhandled ANSI mode {:?}", mode);
        }
    }

    fn set_private_mode(&mut self, mode: PrivateMode, enabled: bool) {
        if let PrivateMode::Named(named) = mode {
            match named {
                NamedPrivateMode::Origin => {
                    self.origin_mode = enabled;
                    self.cursor_home();
                },
                NamedPrivateMode::LineWrap => {
                    self.autowrap = enabled;
                    if !enabled {
                        self.wrap_pending = false;
                    }
                },
                NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                    if enabled {
                        self.enter_altscreem();
                    } else {
                        self.exit_altscreem();
                    }
                },
                NamedPrivateMode::ColumnMode => {
                    self.decolumn();
                },
                _ => {},
            }
        }
    }

    fn push_window_title(&mut self) {
        if let Some(title) = &self.window_title {
            self.window_title_stack.push(title.clone());
        }
    }

    fn pop_window_title(&mut self) {
        self.window_title = self.window_title_stack.pop();
    }

    fn set_window_title(&mut self, title: String) {
        self.window_title = Some(title);
    }

    fn scroll_display(&mut self, direction: crate::grid::ScrollDirection) {
        Surface::scroll_display(self, direction);
    }

    fn enter_altscreem(&mut self) {
        if self.primary_screen.is_some() {
            return;
        }

        let columns = self.grid.width();
        let rows = self.grid.height();
        self.primary_screen = Some(ScreenState::from_surface(self));
        self.grid = Grid::new(rows, columns, 0, &self.default_attributes);
        self.reset_state();
        self.mark_full_dirty();
    }

    fn exit_altscreem(&mut self) {
        if let Some(state) = self.primary_screen.take() {
            state.apply(self);
            self.mark_full_dirty();
        } else {
            warn!(
                "Alternate screen exit requested without active primary snapshot"
            );
        }
    }

    // TODO: rename
    fn decolumn(&mut self) {
        self.set_scrolling_region(1, self.grid.total_lines());
        self.grid.clear(&self.default_attributes);
        self.mark_full_dirty();
    }

    fn begin_sync(&mut self) {
        self.sync_depth = self.sync_depth.saturating_add(1);
    }

    fn end_sync(&mut self) {
        if self.sync_depth > 0 {
            self.sync_depth -= 1;
        }
    }
}
