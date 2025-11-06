use cursor_icon::CursorIcon;
use log::{debug, trace, warn};
use otty_escape::{
    Action, Actor, CharacterAttribute, ClearMode, LineClearMode, Mode,
    NamedMode, NamedPrivateMode, PrivateMode, TabClearMode,
};
use unicode_width::UnicodeWidthChar;

use crate::{
    cell::{Cell, CellAttributes, CellBlink, CellUnderline},
    grid::Grid,
    state::{CursorSnapshot, SurfacePalette},
};

const DEFAULT_COLUMNS: usize = 80;
const DEFAULT_ROWS: usize = 24;
const DEFAULT_SCROLL_LIMIT: usize = 10_000;
const TAB_WIDTH: usize = 8;

fn default_tab_stops(width: usize) -> Vec<bool> {
    let mut stops = vec![false; width];
    for col in (TAB_WIDTH..width).step_by(TAB_WIDTH) {
        stops[col] = true;
    }
    stops
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
        };
        surface.reset_tab_stops();
        surface
    }

    pub fn resize(&mut self, columns: usize, rows: usize) {
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

    fn line_feed(&mut self) {
        let (_, bottom) = self.scroll_region_limits();

        if self.cursor_row == bottom {
            self.grid.scroll_up(
                self.scroll_top,
                bottom,
                1,
                &self.default_attributes,
            );
            // Reset display offset when new content arrives (snap to bottom).
            if self.grid.display_offset() > 0 {
                self.grid
                    .scroll_display(crate::grid::ScrollDirection::Bottom);
            }
        } else {
            self.cursor_row = (self.cursor_row + 1).min(bottom);
        }

        if self.linefeed_newline {
            self.cursor_col = 0;
        }
        self.wrap_pending = false;
    }

    fn reverse_index(&mut self) {
        if self.cursor_row == self.scroll_top {
            self.grid.scroll_down(
                self.scroll_top,
                self.scroll_bottom,
                1,
                &self.default_attributes,
            );
        } else {
            self.cursor_row = self.cursor_row.saturating_sub(1);
        }
        self.wrap_pending = false;
    }

    fn put_char(&mut self, ch: char) {
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
            self.line_feed();
        }

        let columns = self.grid.width();
        let rows = self.grid.height();
        if columns == 0 || rows == 0 {
            return;
        }

        if width == 2 && (self.cursor_col + 1 >= columns) {
            if self.autowrap {
                self.cursor_col = 0;
                self.line_feed();
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
                &self.default_attributes,
            );
        }

        if self.cursor_col >= self.grid.width() {
            self.cursor_col = self.grid.width().saturating_sub(1);
        }

        self.clear_wide_cell(self.cursor_row, self.cursor_col);

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
                    }
                }
                if self.cursor_col + 1 < columns {
                    self.cursor_col += 1;
                }
            }
        }

        if self.cursor_col + 1 < self.grid.width() {
            self.cursor_col += 1;
        } else {
            self.wrap_pending = self.autowrap;
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
    }

    fn erase_in_display(&mut self, mode: ClearMode) {
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
    }

    fn erase_in_line(&mut self, mode: LineClearMode) {
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
    }

    fn handle_sgr(&mut self, attribute: CharacterAttribute) {
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
        match mode {
            PrivateMode::Named(named) => match named {
                NamedPrivateMode::LineWrap => self.autowrap = enabled,
                NamedPrivateMode::Origin => {
                    self.origin_mode = enabled;
                    self.cursor_home();
                },
                NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                    if enabled {
                        self.enter_alternate_screen();
                    } else {
                        self.exit_alternate_screen();
                    }
                },
                NamedPrivateMode::BlinkingCursor => {
                    debug!("Blinking cursor mode toggled -> {enabled}")
                },
                NamedPrivateMode::ShowCursor => {
                    debug!("Cursor visibility toggled -> {enabled}")
                },
                NamedPrivateMode::SyncUpdate => {
                    if !enabled {
                        self.sync_depth = 0;
                    }
                },
                _ => debug!("Private mode {:?} => {}", named, enabled),
            },
            PrivateMode::Unknown(id) => {
                debug!("Unknown private mode {} => {}", id, enabled)
            },
        }
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
    }

    fn enter_alternate_screen(&mut self) {
        if self.primary_screen.is_some() {
            return;
        }

        let columns = self.grid.width();
        let rows = self.grid.height();
        self.primary_screen = Some(ScreenState::from_surface(self));
        self.grid = Grid::new(rows, columns, 0, &self.default_attributes);
        self.reset_state();
    }

    fn exit_alternate_screen(&mut self) {
        if let Some(state) = self.primary_screen.take() {
            state.apply(self);
        } else {
            warn!(
                "Alternate screen exit requested without active primary snapshot"
            );
        }
    }
}

impl Default for Surface {
    fn default() -> Self {
        Self::new(SurfaceConfig::default())
    }
}

impl Actor for Surface {
    fn handle(&mut self, action: Action) {
        use Action::*;
        match action {
            Print(ch) => self.put_char(ch),
            Bell => debug!("Bell"),
            InsertBlank(count) => {
                self.grid.insert_blank_cells(
                    self.cursor_row,
                    self.cursor_col,
                    count,
                    &self.default_attributes,
                );
            },
            InsertBlankLines(count) => self.insert_blank_lines(count),
            DeleteLines(count) => self.delete_lines(count),
            DeleteChars(count) => self.grid.delete_cells(
                self.cursor_row,
                self.cursor_col,
                count,
                &self.default_attributes,
            ),
            EraseChars(count) => {
                let end =
                    self.cursor_col.saturating_add(count.saturating_sub(1));
                self.grid.clear_range(
                    self.cursor_row,
                    self.cursor_col,
                    end,
                    &self.current_attributes,
                );
            },
            Backspace => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
                self.wrap_pending = false;
            },
            CarriageReturn => {
                self.cursor_col = 0;
                self.wrap_pending = false;
            },
            LineFeed => self.line_feed(),
            NextLine | NewLine => {
                self.line_feed();
                self.cursor_col = 0;
            },
            Substitute => self.put_char('�'),
            SetHorizontalTab => self.set_tab_stop(self.cursor_col),
            ReverseIndex => self.reverse_index(),
            ResetState => {
                self.reset_state();
                self.grid.clear(&self.default_attributes);
            },
            ClearScreen(mode) => self.erase_in_display(mode),
            ClearLine(mode) => self.erase_in_line(mode),
            InsertTabs(count) => {
                let col = self.next_tab_stop(self.cursor_col, count as usize);
                self.cursor_col = col;
                self.wrap_pending = false;
            },
            SetTabs(mask) => {
                for bit in 0..16 {
                    if (mask & (1 << bit)) != 0 {
                        self.set_tab_stop(self.cursor_col + bit as usize);
                    }
                }
            },
            ClearTabs(mode) => match mode {
                TabClearMode::Current => self.clear_tab_stop(self.cursor_col),
                TabClearMode::All => self.clear_all_tab_stops(),
            },
            ScreenAlignmentDisplay => {
                for row in 0..self.grid.height() {
                    for col in 0..self.grid.width() {
                        self.grid.row_mut(row).cells[col] =
                            Cell::with_char('E', &self.default_attributes);
                    }
                }
                self.cursor_home();
            },
            MoveForwardTabs(count) => {
                let col = self.next_tab_stop(self.cursor_col, count as usize);
                self.cursor_col = col;
            },
            MoveBackwardTabs(count) => {
                let col =
                    self.previous_tab_stop(self.cursor_col, count as usize);
                self.cursor_col = col;
            },
            SetActiveCharsetIndex(_) | ConfigureCharset(_, _) => {
                trace!("Charset handling not implemented yet");
            },
            SetColor { index, color } => self.palette.set(index, color),
            QueryColor(index) => debug!("Query color {}", index),
            ResetColor(index) => self.palette.reset(index),
            SetScrollingRegion(top, bottom) => {
                let top = top.saturating_sub(1);
                let bottom = bottom.saturating_sub(1);
                self.set_scrolling_region(top, bottom);
            },
            ScrollUp(count) => self.grid.scroll_up(
                self.scroll_top,
                self.scroll_bottom,
                count,
                &self.default_attributes,
            ),
            ScrollDown(count) => self.grid.scroll_down(
                self.scroll_top,
                self.scroll_bottom,
                count,
                &self.default_attributes,
            ),
            SetHyperlink(link) => self.current_attributes.set_hyperlink(link),
            SGR(attribute) => self.handle_sgr(attribute),
            SetCursorShape(shape) => self.cursor_shape = Some(shape),
            SetCursorIcon(icon) => self.cursor_icon = Some(icon),
            SetCursorStyle(style) => self.cursor_style = style,
            SaveCursorPosition => {
                self.saved_cursor = Some(CursorSnapshot::new(
                    self.cursor_row,
                    self.cursor_col,
                    self.current_attributes.clone(),
                ));
            },
            RestoreCursorPosition => {
                if let Some(snapshot) = self.saved_cursor.clone() {
                    self.cursor_row = snapshot.row;
                    self.cursor_col = snapshot.col;
                    self.current_attributes = snapshot.attributes;
                }
            },
            MoveUp {
                rows,
                carrage_return_needed,
            } => {
                self.cursor_row = self.cursor_row.saturating_sub(rows);
                if carrage_return_needed {
                    self.cursor_col = 0;
                }
                self.clamp_cursor();
            },
            MoveDown {
                rows,
                carrage_return_needed,
            } => {
                self.cursor_row = self.cursor_row.saturating_add(rows);
                if carrage_return_needed {
                    self.cursor_col = 0;
                }
                self.clamp_cursor();
            },
            MoveForward(cols) => {
                self.cursor_col = self
                    .cursor_col
                    .saturating_add(cols)
                    .min(self.grid.width().saturating_sub(1));
                self.wrap_pending = false;
            },
            MoveBackward(cols) => {
                self.cursor_col = self.cursor_col.saturating_sub(cols);
                self.wrap_pending = false;
            },
            Goto(row, col) => {
                let row = if row <= 0 {
                    0
                } else {
                    (row as usize).saturating_sub(1)
                };
                let col = col.saturating_sub(1);
                let base_row =
                    if self.origin_mode { self.scroll_top } else { 0 };
                self.move_cursor_to(base_row + row, col);
            },
            GotoRow(row) => {
                let row = if row <= 0 { 0 } else { row as usize - 1 };
                let base_row =
                    if self.origin_mode { self.scroll_top } else { 0 };
                self.cursor_row =
                    (base_row + row).min(self.grid.height().saturating_sub(1));
                self.wrap_pending = false;
            },
            GotoColumn(col) => {
                let col = if col == 0 { 0 } else { col - 1 };
                self.cursor_col = col.min(self.grid.width().saturating_sub(1));
                self.wrap_pending = false;
            },
            IdentifyTerminal(response) => {
                debug!("Identify terminal {:?}", response)
            },
            ReportDeviceStatus(status) => {
                debug!("Report device status {}", status)
            },
            SetKeypadApplicationMode => {
                self.keypad_application_mode = true;
            },
            UnsetKeypadApplicationMode => {
                self.keypad_application_mode = false;
            },
            SetModifyOtherKeysState(state) => {
                debug!("modifyOtherKeys => {:?}", state);
            },
            ReportModifyOtherKeysState => debug!("Report modifyOtherKeys"),
            ReportKeyboardMode => debug!("Report keyboard mode"),
            SetKeyboardMode(mode, behavior) => {
                debug!("Set keyboard mode {:?} {:?}", mode, behavior);
            },
            PushKeyboardMode(_) => {
                self.keyboard_stack_depth =
                    self.keyboard_stack_depth.saturating_add(1);
            },
            PopKeyboardModes(amount) => {
                self.keyboard_stack_depth =
                    self.keyboard_stack_depth.saturating_sub(amount);
            },
            SetMode(mode) => self.set_mode(mode, true),
            SetPrivateMode(mode) => self.set_private_mode(mode, true),
            UnsetMode(mode) => self.set_mode(mode, false),
            UnsetPrivateMode(mode) => self.set_private_mode(mode, false),
            ReportMode(mode) => debug!("Report mode {:?}", mode),
            ReportPrivateMode(mode) => debug!("Report private mode {:?}", mode),
            RequestTextAreaSizeByPixels => {
                debug!("Request text area size (pixels)")
            },
            RequestTextAreaSizeByChars => {
                debug!("Request text area size (chars)")
            },
            PushWindowTitle => {
                if let Some(title) = &self.window_title {
                    self.window_title_stack.push(title.clone());
                }
            },
            PopWindowTitle => {
                self.window_title = self.window_title_stack.pop();
            },
            SetWindowTitle(title) => {
                self.window_title = Some(title);
            },
        }
    }

    fn begin_sync(&mut self) {
        self.sync_depth = self.sync_depth.saturating_add(1);
    }

    fn end_sync(&mut self) {
        self.sync_depth = self.sync_depth.saturating_sub(1);
    }
}
#[cfg(test)]
mod tests {
    use otty_escape::{
        Action, Actor, CharacterAttribute, Color, NamedPrivateMode,
        PrivateMode, StdColor,
    };

    use super::Surface;

    #[test]
    fn prints_text_across_rows() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('H'));
        surface.handle(Action::Print('i'));
        surface.handle(Action::NewLine);
        surface.handle(Action::Print('!'));

        let grid = surface.grid();

        assert_eq!(grid.row(0).cells[0].ch, 'H');
        assert_eq!(grid.row(0).cells[1].ch, 'i');
        assert_eq!(grid.row(1).cells[0].ch, '!');
    }

    #[test]
    fn applies_basic_sgr_attributes() {
        let mut surface = Surface::default();

        surface.handle(Action::SGR(CharacterAttribute::Bold));
        surface.handle(Action::SGR(CharacterAttribute::Foreground(
            Color::Std(StdColor::Red),
        )));
        surface.handle(Action::Print('A'));

        let cell = &surface.grid().row(0).cells[0];
        assert_eq!(cell.ch, 'A');
        assert!(cell.attributes.bold);
        assert_eq!(cell.attributes.foreground, Color::Std(StdColor::Red));
    }

    #[test]
    fn clear_line_from_cursor() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('A'));
        surface.handle(Action::Print('B'));
        surface.handle(Action::Print('C'));
        surface.handle(Action::MoveBackward(2));
        surface.handle(Action::ClearLine(otty_escape::LineClearMode::Right));

        let row = surface.grid().row(0);
        assert_eq!(row.cells[0].ch, 'A');
        assert_eq!(row.cells[1].ch, ' ');
        assert_eq!(row.cells[2].ch, ' ');
    }

    #[test]
    fn alternate_screen_restores_primary_content() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('A'));
        assert_eq!(surface.grid().row(0).cells[0].ch, 'A');

        surface.handle(Action::SetPrivateMode(PrivateMode::Named(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor,
        )));

        assert_eq!(surface.grid().row(0).cells[0].ch, ' ');

        surface.handle(Action::Print('Z'));
        assert_eq!(surface.grid().row(0).cells[0].ch, 'Z');

        surface.handle(Action::UnsetPrivateMode(PrivateMode::Named(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor,
        )));

        assert_eq!(surface.grid().row(0).cells[0].ch, 'A');
    }

    #[test]
    fn wrapping_with_autowrap_enabled() {
        let mut surface = Surface::default();

        // Print exactly width characters.
        for i in 0..80 {
            surface.handle(Action::Print(
                char::from_digit((i % 10) as u32, 10).unwrap(),
            ));
        }

        // Next character should wrap to next line.
        surface.handle(Action::Print('X'));

        assert_eq!(surface.grid().row(1).cells[0].ch, 'X');
    }

    #[test]
    fn wrapping_disabled_when_autowrap_off() {
        let mut surface = Surface::default();

        // Disable autowrap.
        surface.handle(Action::UnsetPrivateMode(
            otty_escape::PrivateMode::Named(
                otty_escape::NamedPrivateMode::LineWrap,
            ),
        ));

        // Print beyond width.
        for _ in 0..85 {
            surface.handle(Action::Print('A'));
        }

        // Should stay on row 0, last column overwritten.
        assert_eq!(surface.cursor_position().0, 0);
        assert_eq!(surface.grid().row(1).cells[0].ch, ' ');
    }

    #[test]
    fn sgr_combinations() {
        let mut surface = Surface::default();

        surface.handle(Action::SGR(CharacterAttribute::Bold));
        surface.handle(Action::SGR(CharacterAttribute::Italic));
        surface.handle(Action::SGR(CharacterAttribute::Underline));
        surface.handle(Action::Print('A'));

        let cell = &surface.grid().row(0).cells[0];
        assert!(cell.attributes.bold);
        assert!(cell.attributes.italic);
        assert_eq!(
            cell.attributes.underline,
            crate::cell::CellUnderline::Single
        );
    }

    #[test]
    fn clear_screen_modes() {
        use otty_escape::ClearMode;

        let mut surface = Surface::default();

        // Fill grid with content.
        for _ in 0..5 {
            for _ in 0..10 {
                surface.handle(Action::Print('X'));
            }
            surface.handle(Action::NewLine);
        }

        // Move to middle.
        surface.handle(Action::Goto(3, 5));

        // Clear below.
        surface.handle(Action::ClearScreen(ClearMode::Below));

        // Row 2 should have 'X' chars before cursor.
        assert_eq!(surface.grid().row(2).cells[0].ch, 'X');
        // Row 2 at cursor and after should be cleared.
        assert_eq!(surface.grid().row(2).cells[4].ch, ' ');
        // Row 3+ should be cleared.
        assert_eq!(surface.grid().row(3).cells[0].ch, ' ');
    }

    #[test]
    fn insert_and_delete_chars() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('A'));
        surface.handle(Action::Print('B'));
        surface.handle(Action::Print('C'));

        // Move to 'B'.
        surface.handle(Action::Goto(1, 2));
        surface.handle(Action::InsertBlank(1));

        let row = surface.grid().row(0);
        assert_eq!(row.cells[0].ch, 'A');
        assert_eq!(row.cells[1].ch, ' ');
        assert_eq!(row.cells[2].ch, 'B');
        assert_eq!(row.cells[3].ch, 'C');
    }

    #[test]
    fn delete_chars_shifts_left() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('A'));
        surface.handle(Action::Print('B'));
        surface.handle(Action::Print('C'));
        surface.handle(Action::Print('D'));

        // Move to 'B'.
        surface.handle(Action::Goto(1, 2));
        surface.handle(Action::DeleteChars(2));

        let row = surface.grid().row(0);
        assert_eq!(row.cells[0].ch, 'A');
        assert_eq!(row.cells[1].ch, 'D');
        assert_eq!(row.cells[2].ch, ' ');
    }

    #[test]
    fn erase_chars() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('A'));
        surface.handle(Action::Print('B'));
        surface.handle(Action::Print('C'));

        surface.handle(Action::Goto(1, 2));
        surface.handle(Action::EraseChars(2));

        let row = surface.grid().row(0);
        assert_eq!(row.cells[0].ch, 'A');
        assert_eq!(row.cells[1].ch, ' ');
        assert_eq!(row.cells[2].ch, ' ');
    }

    #[test]
    fn insert_and_delete_lines() {
        let mut surface = Surface::default();

        for i in 0..5 {
            surface.handle(Action::Print(char::from_digit(i, 10).unwrap()));
            surface.handle(Action::NewLine);
        }

        surface.handle(Action::Goto(2, 1));
        surface.handle(Action::InsertBlankLines(1));

        assert_eq!(surface.grid().row(0).cells[0].ch, '0');
        assert_eq!(surface.grid().row(1).cells[0].ch, ' ');
        assert_eq!(surface.grid().row(2).cells[0].ch, '1');
    }

    #[test]
    fn scroll_region_basic() {
        let mut surface = Surface::default();

        // Set scroll region to rows 2-5 (1-based).
        surface.handle(Action::SetScrollingRegion(2, 5));

        // Cursor should be at home (within region if origin mode).
        assert_eq!(surface.cursor_position(), (0, 0));
    }

    #[test]
    fn scrollback_accumulation() {
        let mut surface = Surface::default();

        // Initially no history.
        assert_eq!(surface.history_size(), 0);

        // Fill the screen.
        for _ in 0..24 {
            surface.handle(Action::Print('X'));
            surface.handle(Action::NewLine);
        }

        // Should have scrolled content into history.
        assert!(surface.history_size() > 0);
    }

    #[test]
    fn tab_stops() {
        let mut surface = Surface::default();

        // Default tabs every 8 columns.
        surface.handle(Action::MoveForwardTabs(1));
        assert_eq!(surface.cursor_position().1, 8);

        surface.handle(Action::MoveForwardTabs(1));
        assert_eq!(surface.cursor_position().1, 16);

        // Move back.
        surface.handle(Action::MoveBackwardTabs(1));
        assert_eq!(surface.cursor_position().1, 8);
    }

    #[test]
    fn cursor_save_restore() {
        let mut surface = Surface::default();

        surface.handle(Action::SGR(CharacterAttribute::Bold));
        surface.handle(Action::Print('A'));
        surface.handle(Action::Print('B'));

        surface.handle(Action::SaveCursorPosition);

        surface.handle(Action::Goto(10, 10));
        surface.handle(Action::SGR(CharacterAttribute::Italic));

        surface.handle(Action::RestoreCursorPosition);

        let (row, col) = surface.cursor_position();
        assert_eq!(row, 0);
        assert_eq!(col, 2);
        // Attributes should also be restored.
        assert!(surface.current_attributes.bold);
        assert!(!surface.current_attributes.italic);
    }

    #[test]
    fn origin_mode_addressing() {
        let mut surface = Surface::default();

        // Set scroll region 5-10.
        surface.handle(Action::SetScrollingRegion(5, 10));

        // Enable origin mode.
        surface.handle(Action::SetPrivateMode(
            otty_escape::PrivateMode::Named(
                otty_escape::NamedPrivateMode::Origin,
            ),
        ));

        // Goto(1,1) should now be relative to scroll_top (row 4 in 0-based).
        surface.handle(Action::Goto(1, 1));
        assert_eq!(surface.cursor_position().0, 4);
    }

    #[test]
    fn reverse_index_at_top() {
        let mut surface = Surface::default();

        // Start at top.
        assert_eq!(surface.cursor_position().0, 0);

        // Reverse index should scroll region down.
        surface.handle(Action::ReverseIndex);

        // Cursor should still be at top, but content scrolled.
        assert_eq!(surface.cursor_position().0, 0);
    }

    #[test]
    fn screen_alignment_display() {
        let mut surface = Surface::default();

        surface.handle(Action::ScreenAlignmentDisplay);

        // All cells should be 'E'.
        for row in 0..surface.grid().height() {
            for col in 0..surface.grid().width() {
                assert_eq!(surface.grid().row(row).cells[col].ch, 'E');
            }
        }

        // Cursor should be at home.
        assert_eq!(surface.cursor_position(), (0, 0));
    }

    #[test]
    fn zero_width_characters_combine_with_previous_cell() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('a'));
        surface.handle(Action::Print('\u{0301}')); // combining acute accent

        let cell = &surface.grid().row(0).cells[0];
        assert_eq!(cell.ch, 'a');
        assert_eq!(cell.zero_width, vec!['\u{0301}']);
        assert_eq!(surface.cursor_position(), (0, 1));
    }

    #[test]
    fn wide_characters_occupy_two_cells() {
        let mut surface = Surface::default();

        surface.handle(Action::Print('漢'));

        let row = surface.grid().row(0);
        assert_eq!(row.cells[0].ch, '漢');
        assert!(row.cells[0].wide_leading);
        assert!(!row.cells[0].wide_trailing);
        assert_eq!(row.cells[1].ch, ' ');
        assert!(!row.cells[1].wide_leading);
        assert!(row.cells[1].wide_trailing);
        assert_eq!(surface.cursor_position(), (0, 2));
    }
}
