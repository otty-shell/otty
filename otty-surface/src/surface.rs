use cursor_icon::CursorIcon;
use log::{debug, trace, warn};
use otty_escape::{
    Action, Actor, CharacterAttribute, ClearMode, LineClearMode, Mode,
    NamedMode, NamedPrivateMode, PrivateMode, TabClearMode,
};

use crate::{
    cell::{Cell, CellAttributes, CellBlink, CellUnderline},
    grid::Grid,
    state::{CursorSnapshot, SurfacePalette},
};

const DEFAULT_COLUMNS: usize = 80;
const DEFAULT_ROWS: usize = 24;
const TAB_WIDTH: usize = 8;

#[derive(Debug, Clone)]
pub struct SurfaceConfig {
    pub columns: usize,
    pub rows: usize,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            columns: DEFAULT_COLUMNS,
            rows: DEFAULT_ROWS,
        }
    }
}

#[derive(Debug, Default)]
struct Osc133Tracker {
    _placeholder: Option<String>,
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
}

impl Surface {
    pub fn new(config: SurfaceConfig) -> Self {
        let columns = config.columns.max(1);
        let rows = config.rows.max(1);
        let default_attributes = CellAttributes::default();
        let mut surface = Self {
            grid: Grid::new(columns, rows, &default_attributes),
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
        self.tab_stops = vec![false; self.grid.width()];
        for col in (TAB_WIDTH..self.grid.width()).step_by(TAB_WIDTH) {
            self.tab_stops[col] = true;
        }
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
        if self.wrap_pending {
            self.cursor_col = 0;
            self.line_feed();
        }

        if self.insert_mode {
            self.grid.insert_blank_cells(
                self.cursor_row,
                self.cursor_col,
                1,
                &self.default_attributes,
            );
        }

        if self.cursor_col >= self.grid.width() {
            self.cursor_col = self.grid.width().saturating_sub(1);
        }

        if self.cursor_row < self.grid.height()
            && self.cursor_col < self.grid.width()
        {
            self.grid.row_mut(self.cursor_row).cells[self.cursor_col] =
                Cell::with_char(ch, &self.current_attributes);
        }

        if self.cursor_col + 1 >= self.grid.width() {
            self.wrap_pending = self.autowrap;
        } else {
            self.cursor_col += 1;
        }
    }

    fn erase_in_display(&mut self, mode: ClearMode) {
        match mode {
            ClearMode::All | ClearMode::Saved => {
                self.grid.clear(&self.default_attributes);
                self.move_cursor_to(0, 0);
            },
            ClearMode::Below => {
                self.grid.clear_range(
                    self.cursor_row,
                    self.cursor_col,
                    self.grid.width().saturating_sub(1),
                    &self.default_attributes,
                );
                for row in (self.cursor_row + 1)..self.grid.height() {
                    self.grid.row_mut(row).clear(&self.default_attributes);
                }
            },
            ClearMode::Above => {
                for row in 0..self.cursor_row {
                    self.grid.row_mut(row).clear(&self.default_attributes);
                }
                self.grid.clear_range(
                    self.cursor_row,
                    0,
                    self.cursor_col,
                    &self.default_attributes,
                );
            },
        }
    }

    fn erase_in_line(&mut self, mode: LineClearMode) {
        match mode {
            LineClearMode::All => {
                self.grid
                    .row_mut(self.cursor_row)
                    .clear(&self.default_attributes);
            },
            LineClearMode::Right => {
                self.grid.clear_range(
                    self.cursor_row,
                    self.cursor_col,
                    self.grid.width().saturating_sub(1),
                    &self.default_attributes,
                );
            },
            LineClearMode::Left => {
                self.grid.clear_range(
                    self.cursor_row,
                    0,
                    self.cursor_col,
                    &self.default_attributes,
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
                    warn!("Alternate screen not yet implemented");
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
                    &self.default_attributes,
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
    use otty_escape::{Action, Actor, CharacterAttribute, Color, StdColor};

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
}
