//! High‑level terminal surface implementation.
//!
//! The [`Surface`] type owns the terminal grid, scrollback history, cursor,
//! selection, colors, modes and various other pieces of state. It implements
//! [`crate::SurfaceActor`], meaning it can consume the semantic actions
//! produced by `otty-escape` and update its internal state accordingly.
//!
//! Rendering frontends typically interact with [`Surface`] by:
//! - Constructing it with [`Surface::new`] for a given size.
//! - Driving it via the [`crate::SurfaceActor`] methods.
//! - Periodically calling [`Surface::snapshot`] (or
//!   [`crate::SurfaceSnapshotSource::capture_snapshot`]) to obtain a
//!   read‑only view suitable for drawing.

use std::cmp::max;
use std::collections::VecDeque;
use std::ops::{Index, IndexMut, Range};
use std::{cmp, mem, ptr, str};

use log::{debug, trace};
use unicode_width::UnicodeWidthChar;

use crate::Side;
use crate::actor::SurfaceActor;
use crate::cell::{Cell, Flags, LineLength};
use crate::color::Colors;
use crate::damage::{SurfaceDamage, SurfaceDamageIterator, SurfaceDamageState};
use crate::escape::{
    CharacterAttribute, Charset, CharsetIndex, ClearMode, Color, CursorStyle,
    Hyperlink, KeyboardMode, KeyboardModeApplyBehavior, LineClearMode, Mode,
    NamedMode, NamedPrivateMode, PrivateMode, Rgb, StdColor, TabClearMode,
};
use crate::grid::{BidirectionalIterator, Dimensions, Grid, Scroll};
use crate::index::{Boundary, Column, Direction, Line, Point};
use crate::mode::SurfaceMode;
use crate::selection::{Selection, SelectionRange, SelectionType};
use crate::snapshot::SurfaceSnapshot;

/// Max size of the window title stack.
const TITLE_STACK_MAX_DEPTH: usize = 4096;

/// Max size of the keyboard modes.
const KEYBOARD_MODE_STACK_MAX_DEPTH: usize = TITLE_STACK_MAX_DEPTH;

/// Default semantic escape characters.
pub const SEMANTIC_ESCAPE_CHARS: &str = ",│`|:\"' ()[]{}<>\t";

/// Used to match equal brackets, when performing a bracket-pair selection.
const BRACKET_PAIRS: [(char, char); 4] =
    [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];

/// Default tab interval, corresponding to surfaceinfo `it` value.
const INITIAL_TABSTOPS: usize = 8;

/// Convert a terminal point to a viewport‑relative point.
#[inline]
pub fn point_to_viewport(
    display_offset: usize,
    point: Point,
) -> Option<Point<usize>> {
    let viewport_line = point.line.0 + display_offset as i32;
    usize::try_from(viewport_line)
        .ok()
        .map(|line| Point::new(line, point.column))
}

/// Convert a viewport‑relative point to a terminal point.
#[inline]
pub fn viewport_to_point(display_offset: usize, point: Point<usize>) -> Point {
    let line = Line(point.line as i32) - display_offset;
    Point::new(line, point.column)
}

/// In‑memory representation of a terminal surface.
///
/// A `Surface` is responsible for tracking all state required to faithfully
/// emulate a terminal screen: primary/alternate grids, scrollback history,
/// cursor and its attributes, current color palette, active modes, selection,
/// damage information and configuration.
pub struct Surface {
    /// Terminal focus controlling the cursor shape.
    pub is_focused: bool,

    pub selection: Option<Selection>,
    /// Currently active grid.
    ///
    /// Tracks the screen buffer currently in use. While the alternate screen
    /// buffer is active, this will be the alternate grid. Otherwise it is
    /// the primary screen buffer.
    grid: Grid<Cell>,

    /// Currently inactive grid.
    ///
    /// Opposite of the active grid. While the alternate screen buffer is
    /// active, this will be the primary grid. Otherwise it is the alternate
    /// screen buffer.
    inactive_grid: Grid<Cell>,

    /// Index into `charsets`, pointing to what ASCII is currently being mapped to.
    active_charset: CharsetIndex,

    /// Tabstops.
    tabs: TabStops,

    /// Mode flags.
    mode: SurfaceMode,

    /// Scroll region.
    ///
    /// Range going from top to bottom of the terminal, indexed from the top
    /// of the viewport.
    scroll_region: Range<Line>,

    /// Modified terminal colors.
    colors: Colors,

    /// Current style of the cursor.
    cursor_style: Option<CursorStyle>,

    /// Current title of the window.
    title: Option<String>,

    /// Stack of saved window titles. When a title is popped from this stack,
    /// the `title` for the surface is set.
    title_stack: Vec<Option<String>>,

    /// The stack for the keyboard modes.
    keyboard_mode_stack: Vec<KeyboardMode>,

    /// Currently inactive keyboard mode stack.
    inactive_keyboard_mode_stack: Vec<KeyboardMode>,

    /// Information about damaged cells.
    damage: SurfaceDamageState,

    /// Static configuration for this surface.
    config: SurfaceConfig,
}

/// Configuration options for the [`Surface`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceConfig {
    /// The maximum amount of scrolling history.
    pub scrolling_history: usize,

    /// Default cursor style to reset the cursor to.
    pub default_cursor_style: CursorStyle,

    /// The characters which delimit semantic selection.
    /// The default value is [`SEMANTIC_ESCAPE_CHARS`].
    pub semantic_escape_chars: String,

    /// Whether to enable kitty keyboard protocol.
    pub kitty_keyboard: bool,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            scrolling_history: 10000,
            semantic_escape_chars: SEMANTIC_ESCAPE_CHARS.to_owned(),
            default_cursor_style: Default::default(),
            kitty_keyboard: Default::default(),
        }
    }
}

impl Surface {
    /// Create a new surface for the given dimensions and configuration.
    pub fn new<D: Dimensions>(
        config: SurfaceConfig,
        dimensions: &D,
    ) -> Surface {
        let num_cols = dimensions.columns();
        let num_lines = dimensions.screen_lines();

        let history_size = config.scrolling_history;
        let grid = Grid::new(num_lines, num_cols, history_size);
        let inactive_grid = Grid::new(num_lines, num_cols, 0);

        let tabs = TabStops::new(grid.columns());

        let scroll_region = Line(0)..Line(grid.screen_lines() as i32);

        // Initialize terminal damage, covering the entire surface upon launch.
        let damage = SurfaceDamageState::new(num_cols, num_lines);

        Surface {
            inactive_grid,
            scroll_region,
            damage,
            config,
            grid,
            tabs,
            inactive_keyboard_mode_stack: Default::default(),
            keyboard_mode_stack: Default::default(),
            active_charset: Default::default(),
            cursor_style: Default::default(),
            colors: Colors::default(),
            title_stack: Default::default(),
            is_focused: Default::default(),
            selection: Default::default(),
            title: Default::default(),
            mode: Default::default(),
        }
    }

    /// Collect the information about the changes in the lines, which
    /// could be used to minimize the amount of drawing operations.
    #[must_use]
    pub fn damage(&mut self) -> SurfaceDamage<'_> {
        // Ensure the entire surface is damaged after entering insert mode.
        // Leaving is handled in the ansi handler.
        if self.mode.contains(SurfaceMode::INSERT) {
            self.mark_fully_damaged();
        }

        let previous_cursor =
            mem::replace(&mut self.damage.last_cursor, self.grid.cursor.point);

        if self.damage.full {
            return SurfaceDamage::Full;
        }

        // Add information about old cursor position and new one if they are not the same, so we
        // cover everything that was produced by `surface::input`.
        if self.damage.last_cursor != previous_cursor {
            // Cursor coordinates are always inside viewport even if you have `display_offset`.
            let point = Point::new(
                previous_cursor.line.0 as usize,
                previous_cursor.column,
            );
            self.damage.damage_point(point);
        }

        // Always damage current cursor.
        self.damage_cursor();

        // NOTE: damage which changes all the content when the display offset is non-zero (e.g.
        // scrolling) is handled via full damage.
        let display_offset = self.grid().display_offset();
        SurfaceDamage::Partial(SurfaceDamageIterator::new(
            &self.damage.lines,
            display_offset,
        ))
    }

    /// Reset the accumulated damage information.
    #[inline]
    pub fn reset_damage(&mut self) {
        self.damage.reset(self.columns());
    }

    #[inline]
    fn mark_fully_damaged(&mut self) {
        self.damage.full = true;
    }

    /// Convert the active selection to a String.
    #[inline]
    pub fn selection_to_string(&self) -> Option<String> {
        let selection_range =
            self.selection.as_ref().and_then(|s| s.to_range(self))?;
        let SelectionRange { start, end, .. } = selection_range;

        let mut res = String::new();

        match self.selection.as_ref() {
            Some(Selection {
                ty: SelectionType::Block,
                ..
            }) => {
                for line in (start.line.0..end.line.0).map(Line::from) {
                    res += self
                        .line_to_string(
                            line,
                            start.column..end.column,
                            start.column.0 != 0,
                        )
                        .trim_end();
                    res += "\n";
                }

                res += self
                    .line_to_string(end.line, start.column..end.column, true)
                    .trim_end();
            },
            Some(Selection {
                ty: SelectionType::Lines,
                ..
            }) => {
                res = self.bounds_to_string(start, end) + "\n";
            },
            _ => {
                res = self.bounds_to_string(start, end);
            },
        }

        Some(res)
    }

    /// Convert range between two points to a String.
    #[inline]
    pub fn bounds_to_string(&self, start: Point, end: Point) -> String {
        let mut res = String::new();

        for line in (start.line.0..=end.line.0).map(Line::from) {
            let start_col = if line == start.line {
                start.column
            } else {
                Column(0)
            };
            let end_col = if line == end.line {
                end.column
            } else {
                self.last_column()
            };

            res += &self.line_to_string(
                line,
                start_col..end_col,
                line == end.line,
            );
        }

        res.strip_suffix('\n').map(str::to_owned).unwrap_or(res)
    }

    /// Convert a single line in the grid to a String.
    fn line_to_string(
        &self,
        line: Line,
        mut cols: Range<Column>,
        include_wrapped_wide: bool,
    ) -> String {
        let mut text = String::new();

        let grid_line = &self.grid[line];
        let line_length = cmp::min(grid_line.line_length(), cols.end + 1);

        // Include wide char when trailing spacer is selected.
        if grid_line[cols.start]
            .flags
            .contains(Flags::WIDE_CHAR_SPACER)
        {
            cols.start -= 1;
        }

        let mut tab_mode = false;
        for column in (cols.start.0..line_length.0).map(Column::from) {
            let cell = &grid_line[column];

            // Skip over cells until next tab-stop once a tab was found.
            if tab_mode {
                if self.tabs[column] || cell.c != ' ' {
                    tab_mode = false;
                } else {
                    continue;
                }
            }

            if cell.c == '\t' {
                tab_mode = true;
            }

            if !cell.flags.intersects(
                Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER,
            ) {
                // Push cells primary character.
                text.push(cell.c);

                // Push zero-width characters.
                for c in cell.zerowidth().into_iter().flatten() {
                    text.push(*c);
                }
            }
        }

        if cols.end >= self.columns() - 1
            && (line_length.0 == 0
                || !self.grid[line][line_length - 1]
                    .flags
                    .contains(Flags::WRAPLINE))
        {
            text.push('\n');
        }

        // If wide char is not part of the selection, but leading spacer is, include it.
        if line_length == self.columns()
            && line_length.0 >= 2
            && grid_line[line_length - 1]
                .flags
                .contains(Flags::LEADING_WIDE_CHAR_SPACER)
            && include_wrapped_wide
        {
            text.push(self.grid[line - 1i32][Column(0)].c);
        }

        text
    }

    /// Surface content required for rendering.
    #[inline]
    pub fn snapshot(&self) -> SurfaceSnapshot<'_> {
        SurfaceSnapshot::new(self)
    }

    /// Access to the raw grid data structure.
    pub fn grid(&self) -> &Grid<Cell> {
        &self.grid
    }

    /// Mutable access to the raw grid data structure.
    pub fn grid_mut(&mut self) -> &mut Grid<Cell> {
        &mut self.grid
    }

    /// Active surface modes.
    #[inline]
    pub fn mode(&self) -> &SurfaceMode {
        &self.mode
    }

    #[inline]
    pub fn colors(&self) -> &Colors {
        &self.colors
    }

    #[inline]
    pub fn semantic_escape_chars(&self) -> &str {
        &self.config.semantic_escape_chars
    }

    /// Scroll screen down.
    ///
    /// Text moves down; clear at bottom
    /// Expects origin to be in scroll range.
    #[inline]
    fn scroll_down_relative(&mut self, origin: Line, mut lines: usize) {
        trace!("Scrolling down relative: origin={origin}, lines={lines}");

        lines = cmp::min(
            lines,
            (self.scroll_region.end - self.scroll_region.start).0 as usize,
        );
        lines = cmp::min(lines, (self.scroll_region.end - origin).0 as usize);

        let region = origin..self.scroll_region.end;

        // Scroll selection.
        self.selection = self
            .selection
            .take()
            .and_then(|s| s.rotate(self, &region, -(lines as i32)));

        // Scroll between origin and bottom
        self.grid.scroll_down(&region, lines);
        self.mark_fully_damaged();
    }

    /// Scroll screen up
    ///
    /// Text moves up; clear at top
    /// Expects origin to be in scroll range.
    #[inline]
    fn scroll_up_relative(&mut self, origin: Line, mut lines: usize) {
        trace!("Scrolling up relative: origin={origin}, lines={lines}");

        lines = cmp::min(
            lines,
            (self.scroll_region.end - self.scroll_region.start).0 as usize,
        );

        let region = origin..self.scroll_region.end;

        // Scroll selection.
        self.selection = self
            .selection
            .take()
            .and_then(|s| s.rotate(self, &region, lines as i32));

        self.grid.scroll_up(&region, lines);
        self.mark_fully_damaged();
    }

    /// Scroll display to point if it is outside of viewport.
    pub fn scroll_to_point(&mut self, point: Point) {
        let display_offset = self.grid.display_offset() as i32;
        let screen_lines = self.grid.screen_lines() as i32;

        if point.line < -display_offset {
            let lines = point.line + display_offset;
            self.scroll_display(Scroll::Delta(-lines.0));
        } else if point.line >= (screen_lines - display_offset) {
            let lines = point.line + display_offset - screen_lines + 1i32;
            self.scroll_display(Scroll::Delta(-lines.0));
        }
    }

    /// Jump to the end of a wide cell.
    pub fn expand_wide(&self, mut point: Point, direction: Direction) -> Point {
        let flags = self.grid[point.line][point.column].flags;

        match direction {
            Direction::Right
                if flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) =>
            {
                point.column = Column(1);
                point.line += 1;
            },
            Direction::Right if flags.contains(Flags::WIDE_CHAR) => {
                point.column = cmp::min(point.column + 1, self.last_column());
            },
            Direction::Left
                if flags
                    .intersects(Flags::WIDE_CHAR | Flags::WIDE_CHAR_SPACER) =>
            {
                if flags.contains(Flags::WIDE_CHAR_SPACER) {
                    point.column -= 1;
                }

                let prev = point.sub(self, Boundary::Grid, 1);
                if self.grid[prev]
                    .flags
                    .contains(Flags::LEADING_WIDE_CHAR_SPACER)
                {
                    point = prev;
                }
            },
            _ => (),
        }

        point
    }

    /// Active surface cursor style.
    ///
    /// While vi mode is active, this will automatically return the vi mode cursor style.
    #[inline]
    pub fn cursor_style(&self) -> CursorStyle {
        self.cursor_style
            .unwrap_or(self.config.default_cursor_style)
    }

    /// Insert a linebreak at the current cursor position.
    #[inline]
    fn wrapline(&mut self) {
        if !self.mode.contains(SurfaceMode::LINE_WRAP) {
            return;
        }

        trace!("Wrapping input");

        self.grid.cursor_cell().flags.insert(Flags::WRAPLINE);

        if self.grid.cursor.point.line + 1 >= self.scroll_region.end {
            self.line_feed();
        } else {
            self.damage_cursor();
            self.grid.cursor.point.line += 1;
        }

        self.grid.cursor.point.column = Column(0);
        self.grid.cursor.input_needs_wrap = false;
        self.damage_cursor();
    }

    /// Write `c` to the cell at the cursor position.
    #[inline(always)]
    fn write_at_cursor(&mut self, c: char) {
        // TODO:
        let c = self.grid.cursor.charsets[self.active_charset].map(c);
        let fg = self.grid.cursor.template.fg;
        let bg = self.grid.cursor.template.bg;
        let flags = self.grid.cursor.template.flags;
        let extra = self.grid.cursor.template.extra.clone();

        let mut cursor_cell = self.grid.cursor_cell();

        // Clear all related cells when overwriting a fullwidth cell.
        if cursor_cell
            .flags
            .intersects(Flags::WIDE_CHAR | Flags::WIDE_CHAR_SPACER)
        {
            // Remove wide char and spacer.
            let wide = cursor_cell.flags.contains(Flags::WIDE_CHAR);
            let point = self.grid.cursor.point;
            if wide && point.column < self.last_column() {
                self.grid[point.line][point.column + 1]
                    .flags
                    .remove(Flags::WIDE_CHAR_SPACER);
            } else if point.column > 0 {
                self.grid[point.line][point.column - 1].clear_wide();
            }

            // Remove leading spacers.
            if point.column <= 1 && point.line != self.topmost_line() {
                let column = self.last_column();
                self.grid[point.line - 1i32][column]
                    .flags
                    .remove(Flags::LEADING_WIDE_CHAR_SPACER);
            }

            cursor_cell = self.grid.cursor_cell();
        }

        cursor_cell.c = c;
        cursor_cell.fg = fg;
        cursor_cell.bg = bg;
        cursor_cell.flags = flags;
        cursor_cell.extra = extra;
    }

    #[inline]
    fn damage_cursor(&mut self) {
        // The normal cursor coordinates are always in viewport.
        let point = Point::new(
            self.grid.cursor.point.line.0 as usize,
            self.grid.cursor.point.column,
        );
        self.damage.damage_point(point);
    }

    #[inline]
    fn set_keyboard_mode(
        &mut self,
        mode: SurfaceMode,
        apply: KeyboardModeApplyBehavior,
    ) {
        let active_mode = self.mode & SurfaceMode::KITTY_KEYBOARD_PROTOCOL;
        self.mode &= !SurfaceMode::KITTY_KEYBOARD_PROTOCOL;
        let new_mode = match apply {
            KeyboardModeApplyBehavior::Replace => mode,
            KeyboardModeApplyBehavior::Union => active_mode.union(mode),
            KeyboardModeApplyBehavior::Difference => {
                active_mode.difference(mode)
            },
        };
        trace!("Setting keyboard mode to {new_mode:?}");
        self.mode |= new_mode;
    }

    pub fn bracket_search(&self, point: Point) -> Option<Point> {
        let start_char = self.grid[point].c;

        // Find the matching bracket we're looking for
        let (forward, end_char) =
            BRACKET_PAIRS.iter().find_map(|(open, close)| {
                if open == &start_char {
                    Some((true, *close))
                } else if close == &start_char {
                    Some((false, *open))
                } else {
                    None
                }
            })?;

        let mut iter = self.grid.iter_from(point);

        // For every character match that equals the starting bracket, we
        // ignore one bracket of the opposite type.
        let mut skip_pairs = 0;

        loop {
            // Check the next cell
            let cell = if forward { iter.next() } else { iter.prev() };

            // Break if there are no more cells
            let cell = match cell {
                Some(cell) => cell,
                None => break,
            };

            // Check if the bracket matches
            if cell.c == end_char && skip_pairs == 0 {
                return Some(cell.point);
            } else if cell.c == start_char {
                skip_pairs += 1;
            } else if cell.c == end_char {
                skip_pairs -= 1;
            }
        }

        None
    }

    /// Find left end of semantic block.
    #[must_use]
    pub fn semantic_search_left(&self, point: Point) -> Point {
        match self.inline_search_left(point, self.semantic_escape_chars()) {
            // If we found a match, reverse for at least one cell, skipping over wide cell spacers.
            Ok(point) => {
                let wide_spacer =
                    Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
                self.grid
                    .iter_from(point)
                    .find(|cell| !cell.flags.intersects(wide_spacer))
                    .map_or(point, |cell| cell.point)
            },
            Err(point) => point,
        }
    }

    /// Find right end of semantic block.
    #[must_use]
    pub fn semantic_search_right(&self, point: Point) -> Point {
        match self.inline_search_right(point, self.semantic_escape_chars()) {
            Ok(point) => self
                .grid
                .iter_from(point)
                .prev()
                .map_or(point, |cell| cell.point),
            Err(point) => point,
        }
    }

    /// Searching to the left, find the next character contained in `needles`.
    pub fn inline_search_left(
        &self,
        mut point: Point,
        needles: &str,
    ) -> Result<Point, Point> {
        // Limit the starting point to the last line in the history
        point.line = max(point.line, self.topmost_line());

        let mut iter = self.grid.iter_from(point);
        let last_column = self.columns() - 1;

        let wide_spacer =
            Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
        while let Some(cell) = iter.prev() {
            if cell.point.column == last_column
                && !cell.flags.contains(Flags::WRAPLINE)
            {
                break;
            }

            point = cell.point;

            if !cell.flags.intersects(wide_spacer) && needles.contains(cell.c) {
                return Ok(point);
            }
        }

        Err(point)
    }

    /// Searching to the right, find the next character contained in `needles`.
    pub fn inline_search_right(
        &self,
        mut point: Point,
        needles: &str,
    ) -> Result<Point, Point> {
        // Limit the starting point to the last line in the history
        point.line = max(point.line, self.topmost_line());

        let wide_spacer =
            Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
        let last_column = self.columns() - 1;

        // Immediately stop if start point in on line break.
        if point.column == last_column
            && !self.grid[point].flags.contains(Flags::WRAPLINE)
        {
            return Err(point);
        }

        for cell in self.grid.iter_from(point) {
            point = cell.point;

            if !cell.flags.intersects(wide_spacer) && needles.contains(cell.c) {
                return Ok(point);
            }

            if point.column == last_column
                && !cell.flags.contains(Flags::WRAPLINE)
            {
                break;
            }
        }

        Err(point)
    }

    /// Find the beginning of the current line across linewraps.
    pub fn line_search_left(&self, mut point: Point) -> Point {
        while point.line > self.topmost_line()
            && self.grid[point.line - 1i32][self.last_column()]
                .flags
                .contains(Flags::WRAPLINE)
        {
            point.line -= 1;
        }

        point.column = Column(0);

        point
    }

    /// Find the end of the current line across linewraps.
    pub fn line_search_right(&self, mut point: Point) -> Point {
        while point.line + 1 < self.screen_lines()
            && self.grid[point.line][self.last_column()]
                .flags
                .contains(Flags::WRAPLINE)
        {
            point.line += 1;
        }

        point.column = self.last_column();

        point
    }

    fn swap_altscreen(&mut self) {
        if !self.mode.contains(SurfaceMode::ALT_SCREEN) {
            // Set alt screen cursor to the current primary screen cursor.
            self.inactive_grid.cursor = self.grid.cursor.clone();

            // Drop information about the primary screens saved cursor.
            self.grid.saved_cursor = self.grid.cursor.clone();

            // Reset alternate screen contents.
            self.inactive_grid.reset_region(..);
        }

        mem::swap(
            &mut self.keyboard_mode_stack,
            &mut self.inactive_keyboard_mode_stack,
        );
        let keyboard_mode = self
            .keyboard_mode_stack
            .last()
            .copied()
            .unwrap_or(KeyboardMode::NO_MODE)
            .into();
        self.set_keyboard_mode(
            keyboard_mode,
            KeyboardModeApplyBehavior::Replace,
        );

        mem::swap(&mut self.grid, &mut self.inactive_grid);
        self.mode ^= SurfaceMode::ALT_SCREEN;
        self.selection = None;
        self.mark_fully_damaged();
    }
}

impl Dimensions for Surface {
    #[inline]
    fn columns(&self) -> usize {
        self.grid.columns()
    }

    #[inline]
    fn screen_lines(&self) -> usize {
        self.grid.screen_lines()
    }

    #[inline]
    fn total_lines(&self) -> usize {
        self.grid.total_lines()
    }
}

impl SurfaceActor for Surface {
    fn print(&mut self, ch: char) {
        // Number of cells the char will occupy.
        let width = match ch.width() {
            Some(width) => width,
            None => return,
        };

        // Handle zero-width characters.
        if width == 0 {
            // Get previous column.
            let mut column = self.grid.cursor.point.column;
            if !self.grid.cursor.input_needs_wrap {
                column.0 = column.saturating_sub(1);
            }

            // Put zerowidth characters over first fullwidth character cell.
            let line = self.grid.cursor.point.line;
            if self.grid[line][column]
                .flags
                .contains(Flags::WIDE_CHAR_SPACER)
            {
                column.0 = column.saturating_sub(1);
            }

            self.grid[line][column].push_zerowidth(ch);
            return;
        }

        // Move cursor to next line.
        if self.grid.cursor.input_needs_wrap {
            self.wrapline();
        }

        // If in insert mode, first shift cells to the right.
        let columns = self.columns();
        if self.mode.contains(SurfaceMode::INSERT)
            && self.grid.cursor.point.column + width < columns
        {
            let line = self.grid.cursor.point.line;
            let col = self.grid.cursor.point.column;
            let row = &mut self.grid[line][..];

            for col in (col.0..(columns - width)).rev() {
                row.swap(col + width, col);
            }
        }

        if width == 1 {
            self.write_at_cursor(ch);
        } else {
            if self.grid.cursor.point.column + 1 >= columns {
                if self.mode.contains(SurfaceMode::LINE_WRAP) {
                    // Insert placeholder before wide char if glyph does not fit in this row.
                    self.grid
                        .cursor
                        .template
                        .flags
                        .insert(Flags::LEADING_WIDE_CHAR_SPACER);
                    self.write_at_cursor(' ');
                    self.grid
                        .cursor
                        .template
                        .flags
                        .remove(Flags::LEADING_WIDE_CHAR_SPACER);
                    self.wrapline();
                } else {
                    // Prevent out of bounds crash when linewrapping is disabled.
                    self.grid.cursor.input_needs_wrap = true;
                    return;
                }
            }

            // Write full width glyph to current cursor cell.
            self.grid.cursor.template.flags.insert(Flags::WIDE_CHAR);
            self.write_at_cursor(ch);
            self.grid.cursor.template.flags.remove(Flags::WIDE_CHAR);

            // Write spacer to cell following the wide glyph.
            self.grid.cursor.point.column += 1;
            self.grid
                .cursor
                .template
                .flags
                .insert(Flags::WIDE_CHAR_SPACER);
            self.write_at_cursor(' ');
            self.grid
                .cursor
                .template
                .flags
                .remove(Flags::WIDE_CHAR_SPACER);
        }

        if self.grid.cursor.point.column + 1 < columns {
            self.grid.cursor.point.column += 1;
        } else {
            self.grid.cursor.input_needs_wrap = true;
        }
    }

    fn resize<S: Dimensions>(&mut self, size: S) {
        let old_cols = self.columns();
        let old_lines = self.screen_lines();

        let num_cols = size.columns();
        let num_lines = size.screen_lines();

        if old_cols == num_cols && old_lines == num_lines {
            debug!("surface::resize dimensions unchanged");
            return;
        }

        debug!("New num_cols is {num_cols} and num_lines is {num_lines}");

        // Move vi mode cursor with the content.
        let history_size = self.history_size();
        let mut delta = num_lines as i32 - old_lines as i32;
        let min_delta =
            cmp::min(0, num_lines as i32 - self.grid.cursor.point.line.0 - 1);
        delta = cmp::min(cmp::max(delta, min_delta), history_size as i32);

        let is_alt = self.mode.contains(SurfaceMode::ALT_SCREEN);
        self.grid.resize(!is_alt, num_lines, num_cols);
        self.inactive_grid.resize(is_alt, num_lines, num_cols);

        // Invalidate selection and tabs only when necessary.
        if old_cols != num_cols {
            self.selection = None;

            // Recreate tabs list.
            self.tabs.resize(num_cols);
        } else if let Some(selection) = self.selection.take() {
            let max_lines = cmp::max(num_lines, old_lines) as i32;
            let range = Line(0)..Line(max_lines);
            self.selection = selection.rotate(self, &range, -delta);
        }

        // Reset scrolling region.
        self.scroll_region = Line(0)..Line(self.screen_lines() as i32);

        // Resize damage information.
        self.damage.resize(num_cols, num_lines);
    }

    fn insert_blank(&mut self, count: usize) {
        let cursor = &self.grid.cursor;
        let bg = cursor.template.bg;

        // Ensure inserting within terminal bounds
        let count = cmp::min(count, self.columns() - cursor.point.column.0);

        let source = cursor.point.column;
        let destination = cursor.point.column.0 + count;
        let num_cells = self.columns() - destination;

        let line = cursor.point.line;
        self.damage
            .damage_line(line.0 as usize, 0, self.columns() - 1);

        let row = &mut self.grid[line][..];

        for offset in (0..num_cells).rev() {
            row.swap(destination + offset, source.0 + offset);
        }

        // Cells were just moved out toward the end of the line;
        // fill in between source and dest with blanks.
        for cell in &mut row[source.0..destination] {
            *cell = bg.into();
        }
    }

    fn insert_blank_lines(&mut self, count: usize) {
        trace!("Inserting blank {count} lines");

        let origin = self.grid.cursor.point.line;
        if self.scroll_region.contains(&origin) {
            self.scroll_down_relative(origin, count);
        }
    }

    fn delete_lines(&mut self, count: usize) {
        let origin = self.grid.cursor.point.line;
        let lines = cmp::min(self.screen_lines() - origin.0 as usize, count);

        trace!("Deleting {lines} lines");

        if lines > 0 && self.scroll_region.contains(&origin) {
            self.scroll_up_relative(origin, lines);
        }
    }

    fn delete_chars(&mut self, count: usize) {
        let columns = self.columns();
        let cursor = &self.grid.cursor;
        let bg = cursor.template.bg;

        // Ensure deleting within terminal bounds.
        let count = cmp::min(count, columns);

        let start = cursor.point.column.0;
        let end = cmp::min(start + count, columns - 1);
        let num_cells = columns - end;

        let line = cursor.point.line;
        self.damage
            .damage_line(line.0 as usize, 0, self.columns() - 1);
        let row = &mut self.grid[line][..];

        for offset in 0..num_cells {
            row.swap(start + offset, end + offset);
        }

        // Clear last `count` cells in the row. If deleting 1 char, need to delete
        // 1 cell.
        let end = columns - count;
        for cell in &mut row[end..] {
            *cell = bg.into();
        }
    }

    fn erase_chars(&mut self, count: usize) {
        let cursor = &self.grid.cursor;

        trace!(
            "Erasing chars: count={}, col={}",
            count, cursor.point.column
        );

        let start = cursor.point.column;
        let end = cmp::min(start + count, Column(self.columns()));

        // Cleared cells have current background color set.
        let bg = self.grid.cursor.template.bg;
        let line = cursor.point.line;
        self.damage.damage_line(line.0 as usize, start.0, end.0);
        let row = &mut self.grid[line];
        for cell in &mut row[start..end] {
            *cell = bg.into();
        }
    }

    fn backspace(&mut self) {
        trace!("Backspace");

        if self.grid.cursor.point.column > Column(0) {
            let line = self.grid.cursor.point.line.0 as usize;
            let column = self.grid.cursor.point.column.0;
            self.grid.cursor.point.column -= 1;
            self.grid.cursor.input_needs_wrap = false;
            self.damage.damage_line(line, column - 1, column);
        }
    }

    fn carriage_return(&mut self) {
        trace!("Carriage return");
        let new_col = 0;
        let line = self.grid.cursor.point.line.0 as usize;
        self.damage
            .damage_line(line, new_col, self.grid.cursor.point.column.0);
        self.grid.cursor.point.column = Column(new_col);
        self.grid.cursor.input_needs_wrap = false;
    }

    fn line_feed(&mut self) {
        trace!("line_feed");
        let next = self.grid.cursor.point.line + 1;
        if next == self.scroll_region.end {
            self.scroll_up(1);
        } else if next < self.screen_lines() {
            self.damage_cursor();
            self.grid.cursor.point.line += 1;
            self.damage_cursor();
        }
    }

    fn new_line(&mut self) {
        trace!("new_line");
        self.line_feed();

        if self.mode.contains(SurfaceMode::LINE_FEED_NEW_LINE) {
            self.carriage_return();
        }
    }

    fn set_horizontal_tab(&mut self) {
        trace!("Setting horizontal tabstop");
        self.tabs[self.grid.cursor.point.column] = true;
    }

    fn reverse_index(&mut self) {
        trace!("Reversing index");
        // If cursor is at the top.
        if self.grid.cursor.point.line == self.scroll_region.start {
            self.scroll_down(1);
        } else {
            self.damage_cursor();
            self.grid.cursor.point.line =
                cmp::max(self.grid.cursor.point.line - 1, Line(0));
            self.damage_cursor();
        }
    }

    fn reset(&mut self) {
        if self.mode.contains(SurfaceMode::ALT_SCREEN) {
            mem::swap(&mut self.grid, &mut self.inactive_grid);
        }
        self.active_charset = Default::default();
        self.cursor_style = None;
        self.grid.reset();
        self.inactive_grid.reset();
        self.scroll_region = Line(0)..Line(self.screen_lines() as i32);
        self.tabs = TabStops::new(self.columns());
        self.title_stack = Vec::new();
        self.title = None;
        self.selection = None;
        self.keyboard_mode_stack = Default::default();
        self.inactive_keyboard_mode_stack = Default::default();
        self.mode.insert(SurfaceMode::default());
        self.mark_fully_damaged();
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        trace!("Clearing screen: {mode:?}");
        let bg = self.grid.cursor.template.bg;

        let screen_lines = self.screen_lines();

        match mode {
            ClearMode::Above => {
                let cursor = self.grid.cursor.point;

                // If clearing more than one line.
                if cursor.line > 1 {
                    // Fully clear all lines before the current line.
                    self.grid.reset_region(..cursor.line);
                }

                // Clear up to the current column in the current line.
                let end = cmp::min(cursor.column + 1, Column(self.columns()));
                for cell in &mut self.grid[cursor.line][..end] {
                    *cell = bg.into();
                }

                let range = Line(0)..=cursor.line;
                self.selection = self
                    .selection
                    .take()
                    .filter(|s| !s.intersects_range(range));
            },
            ClearMode::Below => {
                let cursor = self.grid.cursor.point;
                for cell in &mut self.grid[cursor.line][cursor.column..] {
                    *cell = bg.into();
                }

                if (cursor.line.0 as usize) < screen_lines - 1 {
                    self.grid.reset_region((cursor.line + 1)..);
                }

                let range = cursor.line..Line(screen_lines as i32);
                self.selection = self
                    .selection
                    .take()
                    .filter(|s| !s.intersects_range(range));
            },
            ClearMode::All => {
                if self.mode.contains(SurfaceMode::ALT_SCREEN) {
                    self.grid.reset_region(..);
                } else {
                    self.grid.clear_viewport();
                }

                self.selection = None;
            },
            ClearMode::Saved if self.history_size() > 0 => {
                self.grid.clear_history();
                self.selection = self
                    .selection
                    .take()
                    .filter(|s| !s.intersects_range(..Line(0)));
            },
            // We have no history to clear.
            ClearMode::Saved => (),
        }

        self.mark_fully_damaged();
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        trace!("Clearing line: {mode:?}");

        let cursor = &self.grid.cursor;
        let bg = cursor.template.bg;
        let point = cursor.point;

        let (left, right) = match mode {
            LineClearMode::Right if cursor.input_needs_wrap => return,
            LineClearMode::Right => (point.column, Column(self.columns())),
            LineClearMode::Left => (Column(0), point.column + 1),
            LineClearMode::All => (Column(0), Column(self.columns())),
        };

        self.damage
            .damage_line(point.line.0 as usize, left.0, right.0 - 1);

        let row = &mut self.grid[point.line];
        for cell in &mut row[left..right] {
            *cell = bg.into();
        }

        let range = self.grid.cursor.point.line..=self.grid.cursor.point.line;
        self.selection =
            self.selection.take().filter(|s| !s.intersects_range(range));
    }

    fn insert_tabs(&mut self, mut count: usize) {
        // A tab after the last column is the same as a linebreak.
        if self.grid.cursor.input_needs_wrap {
            self.wrapline();
            return;
        }

        while self.grid.cursor.point.column < self.columns() && count != 0 {
            count -= 1;

            // TODO:
            let c = self.grid.cursor.charsets[self.active_charset].map('\t');
            let cell = self.grid.cursor_cell();
            if cell.c == ' ' {
                cell.c = c;
            }

            loop {
                if (self.grid.cursor.point.column + 1) == self.columns() {
                    break;
                }

                self.grid.cursor.point.column += 1;

                if self.tabs[self.grid.cursor.point.column] {
                    break;
                }
            }
        }
    }

    fn clear_tabs(&mut self, mode: TabClearMode) {
        trace!("Clearing tabs: {mode:?}");
        match mode {
            TabClearMode::Current => {
                self.tabs[self.grid.cursor.point.column] = false;
            },
            TabClearMode::All => {
                self.tabs.clear_all();
            },
        }
    }

    fn screen_alignment_display(&mut self) {
        trace!("Decalnning");

        for line in (0..self.screen_lines()).map(Line::from) {
            for column in 0..self.columns() {
                let cell = &mut self.grid[line][Column(column)];
                *cell = Cell::default();
                cell.c = 'E';
            }
        }

        self.mark_fully_damaged();
    }

    fn move_forward_tabs(&mut self, count: usize) {
        trace!("Moving forward {count} tabs");

        let num_cols = self.columns();
        let old_col = self.grid.cursor.point.column.0;
        for _ in 0..count {
            let mut col = self.grid.cursor.point.column;

            if col == num_cols - 1 {
                break;
            }

            for i in col.0 + 1..num_cols {
                col = Column(i);
                if self.tabs[col] {
                    break;
                }
            }

            self.grid.cursor.point.column = col;
        }

        let line = self.grid.cursor.point.line.0 as usize;
        self.damage
            .damage_line(line, old_col, self.grid.cursor.point.column.0);
    }

    fn move_backward_tabs(&mut self, count: usize) {
        trace!("Moving backward {count} tabs");

        let old_col = self.grid.cursor.point.column.0;
        for _ in 0..count {
            let mut col = self.grid.cursor.point.column;

            if col == 0 {
                break;
            }

            for i in (0..(col.0)).rev() {
                if self.tabs[Column(i)] {
                    col = Column(i);
                    break;
                }
            }
            self.grid.cursor.point.column = col;
        }

        let line = self.grid.cursor.point.line.0 as usize;
        self.damage
            .damage_line(line, self.grid.cursor.point.column.0, old_col);
    }

    fn set_active_charset_index(&mut self, index: CharsetIndex) {
        self.active_charset = index;
    }

    fn configure_charset(&mut self, charset: Charset, index: CharsetIndex) {
        trace!("Configuring charset {index:?} as {charset:?}");
        self.grid.cursor.charsets[index] = charset;
    }

    fn set_color(&mut self, index: usize, color: Rgb) {
        trace!("Setting color[{index}] = {color:?}");

        // Damage surface if the color changed and it's not the cursor.
        if index != StdColor::Cursor as usize
            && self.colors[index] != Some(color)
        {
            self.mark_fully_damaged();
        }

        self.colors[index] = Some(color);
    }

    fn query_color(&mut self, index: usize) {
        debug!("Query color {}", index);
    }

    fn reset_color(&mut self, index: usize) {
        trace!("Resetting color[{index}]");

        // Damage surface if the color changed and it's not the cursor.
        if index != StdColor::Cursor as usize && self.colors[index].is_some() {
            self.mark_fully_damaged();
        }

        self.colors[index] = None;
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: usize) {
        // let top = top.saturating_sub(1);
        // let bottom = bottom.saturating_sub(1);
        // self.set_scrolling_region(top, bottom);

        // Fallback to the last line as default.
        // let bottom = bottom.unwrap_or_else(|| self.screen_lines());

        if top >= bottom {
            debug!("Invalid scrolling region: ({top};{bottom})");
            return;
        }

        // Bottom should be included in the range, but range end is not
        // usually included. One option would be to use an inclusive
        // range, but instead we just let the open range end be 1
        // higher.
        let start = Line(top as i32 - 1);
        let end = Line(bottom as i32);

        trace!("Setting scrolling region: ({start};{end})");

        let screen_lines = Line(self.screen_lines() as i32);
        self.scroll_region.start = cmp::min(start, screen_lines);
        self.scroll_region.end = cmp::min(end, screen_lines);
        self.goto(0, 0);
    }

    fn scroll_up(&mut self, count: usize) {
        let origin = self.scroll_region.start;
        self.scroll_up_relative(origin, count);
    }

    fn scroll_down(&mut self, count: usize) {
        let origin = self.scroll_region.start;
        self.scroll_down_relative(origin, count);
    }

    fn set_hyperlink(&mut self, link: Option<Hyperlink>) {
        trace!("Setting hyperlink: {link:?}");
        self.grid
            .cursor
            .template
            .set_hyperlink(link.map(|e| e.into()));
    }

    fn sgr(&mut self, attribute: CharacterAttribute) {
        trace!("Setting attribute: {attribute:?}");
        let cursor = &mut self.grid.cursor;
        match attribute {
            CharacterAttribute::Foreground(color) => cursor.template.fg = color,
            CharacterAttribute::Background(color) => cursor.template.bg = color,
            CharacterAttribute::UnderlineColor(color) => {
                cursor.template.set_underline_color(color)
            },
            CharacterAttribute::Reset => {
                cursor.template.fg = Color::Std(StdColor::Foreground);
                cursor.template.bg = Color::Std(StdColor::Background);
                cursor.template.flags = Flags::empty();
                cursor.template.set_underline_color(None);
            },
            CharacterAttribute::Reverse => {
                cursor.template.flags.insert(Flags::INVERSE)
            },
            CharacterAttribute::CancelReverse => {
                cursor.template.flags.remove(Flags::INVERSE)
            },
            CharacterAttribute::Bold => {
                cursor.template.flags.insert(Flags::BOLD)
            },
            CharacterAttribute::CancelBold => {
                cursor.template.flags.remove(Flags::BOLD)
            },
            CharacterAttribute::Dim => cursor.template.flags.insert(Flags::DIM),
            CharacterAttribute::CancelBoldDim => {
                cursor.template.flags.remove(Flags::BOLD | Flags::DIM)
            },
            CharacterAttribute::Italic => {
                cursor.template.flags.insert(Flags::ITALIC)
            },
            CharacterAttribute::CancelItalic => {
                cursor.template.flags.remove(Flags::ITALIC)
            },
            CharacterAttribute::Underline => {
                cursor.template.flags.remove(Flags::ALL_UNDERLINES);
                cursor.template.flags.insert(Flags::UNDERLINE);
            },
            CharacterAttribute::DoubleUnderline => {
                cursor.template.flags.remove(Flags::ALL_UNDERLINES);
                cursor.template.flags.insert(Flags::DOUBLE_UNDERLINE);
            },
            CharacterAttribute::Undercurl => {
                cursor.template.flags.remove(Flags::ALL_UNDERLINES);
                cursor.template.flags.insert(Flags::UNDERCURL);
            },
            CharacterAttribute::DottedUnderline => {
                cursor.template.flags.remove(Flags::ALL_UNDERLINES);
                cursor.template.flags.insert(Flags::DOTTED_UNDERLINE);
            },
            CharacterAttribute::DashedUnderline => {
                cursor.template.flags.remove(Flags::ALL_UNDERLINES);
                cursor.template.flags.insert(Flags::DASHED_UNDERLINE);
            },
            CharacterAttribute::CancelUnderline => {
                cursor.template.flags.remove(Flags::ALL_UNDERLINES)
            },
            CharacterAttribute::Hidden => {
                cursor.template.flags.insert(Flags::HIDDEN)
            },
            CharacterAttribute::CancelHidden => {
                cursor.template.flags.remove(Flags::HIDDEN)
            },
            CharacterAttribute::Strike => {
                cursor.template.flags.insert(Flags::STRIKEOUT)
            },
            CharacterAttribute::CancelStrike => {
                cursor.template.flags.remove(Flags::STRIKEOUT)
            },
            _ => {
                debug!("surface got unhandled attr: {attribute:?}");
            },
        }
    }

    fn set_cursor_shape(&mut self, shape: otty_escape::CursorShape) {
        trace!("Setting cursor shape {shape:?}");

        let style = self
            .cursor_style
            .get_or_insert(self.config.default_cursor_style);
        style.shape = shape;
    }

    fn set_cursor_style(&mut self, style: Option<CursorStyle>) {
        self.cursor_style = style;
    }

    fn save_cursor(&mut self) {
        trace!("Saving cursor position");
        self.grid.saved_cursor = self.grid.cursor.clone();
    }

    fn restore_cursor(&mut self) {
        trace!("Restoring cursor position");

        self.damage_cursor();
        self.grid.cursor = self.grid.saved_cursor.clone();
        self.damage_cursor();
    }

    fn move_up(&mut self, rows: usize, carriage_return: bool) {
        if carriage_return {
            trace!("Moving up and cr: {rows}");

            let line = self.grid.cursor.point.line - rows;
            self.goto(line.0, 0);
        } else {
            trace!("Moving up: {rows}");

            let line = self.grid.cursor.point.line - rows;
            let column = self.grid.cursor.point.column;
            self.goto(line.0, column.0);
        }
    }

    fn move_down(&mut self, rows: usize, carriage_return: bool) {
        if carriage_return {
            trace!("Moving down and cr: {rows}");

            let line = self.grid.cursor.point.line + rows;
            self.goto(line.0, 0);
        } else {
            trace!("Moving down: {rows}");

            let line = self.grid.cursor.point.line + rows;
            let column = self.grid.cursor.point.column;
            self.goto(line.0, column.0);
        }
    }

    fn move_forward(&mut self, cols: usize) {
        trace!("Moving forward: {cols}");
        let last_column =
            cmp::min(self.grid.cursor.point.column + cols, self.last_column());

        let cursor_line = self.grid.cursor.point.line.0 as usize;
        self.damage.damage_line(
            cursor_line,
            self.grid.cursor.point.column.0,
            last_column.0,
        );

        self.grid.cursor.point.column = last_column;
        self.grid.cursor.input_needs_wrap = false;
    }

    fn move_backward(&mut self, cols: usize) {
        trace!("Moving backward: {cols}");
        let column = self.grid.cursor.point.column.saturating_sub(cols);

        let cursor_line = self.grid.cursor.point.line.0 as usize;
        self.damage.damage_line(
            cursor_line,
            column,
            self.grid.cursor.point.column.0,
        );

        self.grid.cursor.point.column = Column(column);
        self.grid.cursor.input_needs_wrap = false;
    }

    fn goto(&mut self, line: i32, col: usize) {
        let line = Line(line);
        let col = Column(col);

        trace!("Going to: line={line}, col={col}");
        let (y_offset, max_y) = if self.mode.contains(SurfaceMode::ORIGIN) {
            (self.scroll_region.start, self.scroll_region.end - 1)
        } else {
            (Line(0), self.bottommost_line())
        };

        self.damage_cursor();
        self.grid.cursor.point.line =
            cmp::max(cmp::min(line + y_offset, max_y), Line(0));
        self.grid.cursor.point.column = cmp::min(col, self.last_column());
        self.damage_cursor();
        self.grid.cursor.input_needs_wrap = false;
    }

    fn goto_row(&mut self, row: i32) {
        self.goto(row, self.grid.cursor.point.column.0);
    }

    fn goto_column(&mut self, col: usize) {
        self.goto(self.grid.cursor.point.line.0, col);
    }

    fn set_keypad_application_mode(&mut self, enabled: bool) {
        if enabled {
            trace!("Setting keypad application mode");
            self.mode.insert(SurfaceMode::APP_KEYPAD);
        } else {
            trace!("Unsetting keypad application mode");
            self.mode.remove(SurfaceMode::APP_KEYPAD);
        }
    }

    fn set_keyboard_mode(
        &mut self,
        mode: KeyboardMode,
        apply: KeyboardModeApplyBehavior,
    ) {
        if !self.config.kitty_keyboard {
            return;
        }

        self.set_keyboard_mode(mode.into(), apply);
    }

    #[inline]
    fn push_keyboard_mode(&mut self, mode: KeyboardMode) {
        if !self.config.kitty_keyboard {
            return;
        }

        trace!("Pushing `{mode:?}` keyboard mode into the stack");

        if self.keyboard_mode_stack.len() >= KEYBOARD_MODE_STACK_MAX_DEPTH {
            let removed = self.title_stack.remove(0);
            trace!(
                "Removing '{removed:?}' from bottom of keyboard mode stack that exceeds its \
                maximum depth"
            );
        }

        self.keyboard_mode_stack.push(mode);
        self.set_keyboard_mode(mode.into(), KeyboardModeApplyBehavior::Replace);
    }

    #[inline]
    fn pop_keyboard_modes(&mut self, count: u16) {
        if !self.config.kitty_keyboard {
            return;
        }

        trace!("Attempting to pop {count} keyboard modes from the stack");
        let new_len = self
            .keyboard_mode_stack
            .len()
            .saturating_sub(count as usize);
        self.keyboard_mode_stack.truncate(new_len);

        // Reload active mode.
        let mode = self
            .keyboard_mode_stack
            .last()
            .copied()
            .unwrap_or(KeyboardMode::NO_MODE);
        self.set_keyboard_mode(mode.into(), KeyboardModeApplyBehavior::Replace);
    }

    #[inline]
    fn report_keyboard_mode(&mut self, report_channel: &mut VecDeque<u8>) {
        if !self.config.kitty_keyboard {
            return;
        }

        trace!("Reporting active keyboard mode");
        let current_mode = self
            .keyboard_mode_stack
            .last()
            .unwrap_or(&KeyboardMode::NO_MODE)
            .bits();
        let text = format!("\x1b[?{current_mode}u");
        report_channel.extend(text.as_bytes());
    }

    fn push_window_title(&mut self) {
        trace!("Pushing '{:?}' onto title stack", self.title);

        if self.title_stack.len() >= TITLE_STACK_MAX_DEPTH {
            let removed = self.title_stack.remove(0);
            trace!(
                "Removing '{removed:?}' from bottom of title stack that exceeds its maximum depth"
            );
        }

        self.title_stack.push(self.title.clone());
    }

    fn pop_window_title(&mut self) -> Option<String> {
        trace!("Attempting to pop title from stack...");

        match self.title_stack.pop() {
            Some(popped) => {
                trace!("Title '{popped:?}' popped from stack");
                self.set_window_title(popped.clone());
                popped
            },
            None => None,
        }
    }

    fn set_window_title(&mut self, title: Option<String>) {
        self.title.clone_from(&title);
    }

    fn scroll_display(&mut self, scroll: Scroll) {
        let old_display_offset = self.grid.display_offset();
        self.grid.scroll_display(scroll);
        // Damage everything if display offset changed.
        if old_display_offset != self.grid().display_offset() {
            self.mark_fully_damaged();
        }
    }

    fn deccolm(&mut self) {
        // Setting 132 column font makes no sense, but run the other side effects.
        // Clear scrolling region.
        self.set_scrolling_region(1, self.screen_lines());

        // Clear grid.
        self.grid.reset_region(..);
        self.mark_fully_damaged();
    }

    #[inline]
    fn set_private_mode(&mut self, mode: PrivateMode) {
        let mode = match mode {
            PrivateMode::Named(mode) => mode,
            PrivateMode::Unknown(mode) => {
                debug!("Ignoring unknown mode {mode} in set_private_mode");
                return;
            },
        };

        trace!("Setting private mode: {mode:?}");
        match mode {
            NamedPrivateMode::UrgencyHints => {
                self.mode.insert(SurfaceMode::URGENCY_HINTS)
            },
            NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                if !self.mode.contains(SurfaceMode::ALT_SCREEN) {
                    self.swap_altscreen();
                }
            },
            NamedPrivateMode::ShowCursor => {
                self.mode.insert(SurfaceMode::SHOW_CURSOR)
            },
            NamedPrivateMode::CursorKeys => {
                self.mode.insert(SurfaceMode::APP_CURSOR)
            },
            // Mouse protocols are mutually exclusive.
            NamedPrivateMode::ReportMouseClicks => {
                self.mode.remove(SurfaceMode::MOUSE_MODE);
                self.mode.insert(SurfaceMode::MOUSE_REPORT_CLICK);
            },
            NamedPrivateMode::ReportCellMouseMotion => {
                self.mode.remove(SurfaceMode::MOUSE_MODE);
                self.mode.insert(SurfaceMode::MOUSE_DRAG);
            },
            NamedPrivateMode::ReportAllMouseMotion => {
                self.mode.remove(SurfaceMode::MOUSE_MODE);
                self.mode.insert(SurfaceMode::MOUSE_MOTION);
            },
            NamedPrivateMode::ReportFocusInOut => {
                self.mode.insert(SurfaceMode::FOCUS_IN_OUT)
            },
            NamedPrivateMode::BracketedPaste => {
                self.mode.insert(SurfaceMode::BRACKETED_PASTE)
            },
            // Mouse encodings are mutually exclusive.
            NamedPrivateMode::SgrMouse => {
                self.mode.remove(SurfaceMode::UTF8_MOUSE);
                self.mode.insert(SurfaceMode::SGR_MOUSE);
            },
            NamedPrivateMode::Utf8Mouse => {
                self.mode.remove(SurfaceMode::SGR_MOUSE);
                self.mode.insert(SurfaceMode::UTF8_MOUSE);
            },
            NamedPrivateMode::AlternateScroll => {
                self.mode.insert(SurfaceMode::ALTERNATE_SCROLL)
            },
            NamedPrivateMode::LineWrap => {
                self.mode.insert(SurfaceMode::LINE_WRAP)
            },
            NamedPrivateMode::Origin => {
                self.mode.insert(SurfaceMode::ORIGIN);
                self.goto(0, 0);
            },
            NamedPrivateMode::ColumnMode => self.deccolm(),
            NamedPrivateMode::BlinkingCursor => {
                let style = self
                    .cursor_style
                    .get_or_insert(self.config.default_cursor_style);
                style.blinking = true;
            },
            NamedPrivateMode::SyncUpdate => (),
        }
    }

    #[inline]
    fn unset_private_mode(&mut self, mode: PrivateMode) {
        let mode = match mode {
            PrivateMode::Named(mode) => mode,
            PrivateMode::Unknown(mode) => {
                debug!("Ignoring unknown mode {mode} in unset_private_mode");
                return;
            },
        };

        trace!("Unsetting private mode: {mode:?}");
        match mode {
            NamedPrivateMode::UrgencyHints => {
                self.mode.remove(SurfaceMode::URGENCY_HINTS)
            },
            NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                if self.mode.contains(SurfaceMode::ALT_SCREEN) {
                    self.swap_altscreen();
                }
            },
            NamedPrivateMode::ShowCursor => {
                self.mode.remove(SurfaceMode::SHOW_CURSOR)
            },
            NamedPrivateMode::CursorKeys => {
                self.mode.remove(SurfaceMode::APP_CURSOR)
            },
            NamedPrivateMode::ReportMouseClicks => {
                self.mode.remove(SurfaceMode::MOUSE_REPORT_CLICK);
            },
            NamedPrivateMode::ReportCellMouseMotion => {
                self.mode.remove(SurfaceMode::MOUSE_DRAG);
            },
            NamedPrivateMode::ReportAllMouseMotion => {
                self.mode.remove(SurfaceMode::MOUSE_MOTION);
            },
            NamedPrivateMode::ReportFocusInOut => {
                self.mode.remove(SurfaceMode::FOCUS_IN_OUT)
            },
            NamedPrivateMode::BracketedPaste => {
                self.mode.remove(SurfaceMode::BRACKETED_PASTE)
            },
            NamedPrivateMode::SgrMouse => {
                self.mode.remove(SurfaceMode::SGR_MOUSE)
            },
            NamedPrivateMode::Utf8Mouse => {
                self.mode.remove(SurfaceMode::UTF8_MOUSE)
            },
            NamedPrivateMode::AlternateScroll => {
                self.mode.remove(SurfaceMode::ALTERNATE_SCROLL)
            },
            NamedPrivateMode::LineWrap => {
                self.mode.remove(SurfaceMode::LINE_WRAP)
            },
            NamedPrivateMode::Origin => self.mode.remove(SurfaceMode::ORIGIN),
            NamedPrivateMode::ColumnMode => self.deccolm(),
            NamedPrivateMode::BlinkingCursor => {
                let style = self
                    .cursor_style
                    .get_or_insert(self.config.default_cursor_style);
                style.blinking = false;
            },
            NamedPrivateMode::SyncUpdate => (),
        }
    }

    #[inline]
    fn report_private_mode(
        &mut self,
        mode: PrivateMode,
        report_channel: &mut VecDeque<u8>,
    ) {
        trace!("Reporting private mode {mode:?}");
        let state = match mode {
            PrivateMode::Named(mode) => match mode {
                NamedPrivateMode::CursorKeys => {
                    self.mode.contains(SurfaceMode::APP_CURSOR).into()
                },
                NamedPrivateMode::Origin => {
                    self.mode.contains(SurfaceMode::ORIGIN).into()
                },
                NamedPrivateMode::LineWrap => {
                    self.mode.contains(SurfaceMode::LINE_WRAP).into()
                },
                NamedPrivateMode::BlinkingCursor => {
                    let style = self
                        .cursor_style
                        .get_or_insert(self.config.default_cursor_style);
                    style.blinking.into()
                },
                NamedPrivateMode::ShowCursor => {
                    self.mode.contains(SurfaceMode::SHOW_CURSOR).into()
                },
                NamedPrivateMode::ReportMouseClicks => {
                    self.mode.contains(SurfaceMode::MOUSE_REPORT_CLICK).into()
                },
                NamedPrivateMode::ReportCellMouseMotion => {
                    self.mode.contains(SurfaceMode::MOUSE_DRAG).into()
                },
                NamedPrivateMode::ReportAllMouseMotion => {
                    self.mode.contains(SurfaceMode::MOUSE_MOTION).into()
                },
                NamedPrivateMode::ReportFocusInOut => {
                    self.mode.contains(SurfaceMode::FOCUS_IN_OUT).into()
                },
                NamedPrivateMode::Utf8Mouse => {
                    self.mode.contains(SurfaceMode::UTF8_MOUSE).into()
                },
                NamedPrivateMode::SgrMouse => {
                    self.mode.contains(SurfaceMode::SGR_MOUSE).into()
                },
                NamedPrivateMode::AlternateScroll => {
                    self.mode.contains(SurfaceMode::ALTERNATE_SCROLL).into()
                },
                NamedPrivateMode::UrgencyHints => {
                    self.mode.contains(SurfaceMode::URGENCY_HINTS).into()
                },
                NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                    self.mode.contains(SurfaceMode::ALT_SCREEN).into()
                },
                NamedPrivateMode::BracketedPaste => {
                    self.mode.contains(SurfaceMode::BRACKETED_PASTE).into()
                },
                NamedPrivateMode::SyncUpdate => ModeState::Reset,
                NamedPrivateMode::ColumnMode => ModeState::NotSupported,
            },
            PrivateMode::Unknown(_) => ModeState::NotSupported,
        };

        report_channel.extend(
            format!("\x1b[?{};{}$y", mode.raw(), state as u8,).as_bytes(),
        );
    }

    #[inline]
    fn set_mode(&mut self, mode: Mode) {
        let mode = match mode {
            Mode::Named(mode) => mode,
            Mode::Unknown(mode) => {
                debug!("Ignoring unknown mode {mode} in set_mode");
                return;
            },
        };

        trace!("Setting public mode: {mode:?}");
        match mode {
            NamedMode::Insert => self.mode.insert(SurfaceMode::INSERT),
            NamedMode::LineFeedNewLine => {
                self.mode.insert(SurfaceMode::LINE_FEED_NEW_LINE)
            },
        }
    }

    #[inline]
    fn unset_mode(&mut self, mode: Mode) {
        let mode = match mode {
            Mode::Named(mode) => mode,
            Mode::Unknown(mode) => {
                debug!("Ignoring unknown mode {mode} in unset_mode");
                return;
            },
        };

        trace!("Setting public mode: {mode:?}");
        match mode {
            NamedMode::Insert => {
                self.mode.remove(SurfaceMode::INSERT);
                self.mark_fully_damaged();
            },
            NamedMode::LineFeedNewLine => {
                self.mode.remove(SurfaceMode::LINE_FEED_NEW_LINE)
            },
        }
    }

    #[inline]
    fn report_mode(&mut self, mode: Mode, report_channel: &mut VecDeque<u8>) {
        trace!("Reporting mode {mode:?}");
        let state = match mode {
            Mode::Named(mode) => match mode {
                NamedMode::Insert => {
                    self.mode.contains(SurfaceMode::INSERT).into()
                },
                NamedMode::LineFeedNewLine => {
                    self.mode.contains(SurfaceMode::LINE_FEED_NEW_LINE).into()
                },
            },
            Mode::Unknown(_) => ModeState::NotSupported,
        };

        report_channel.extend(
            format!("\x1b[{};{}$y", mode.raw(), state as u8,).as_bytes(),
        );
    }

    #[inline]
    fn identify_terminal(
        &mut self,
        attr: Option<char>,
        report_channel: &mut VecDeque<u8>,
    ) {
        match attr {
            None => {
                trace!("Reporting primary device attributes");
                report_channel.extend("\x1b[?6c".as_bytes());
            },
            Some('>') => {
                trace!("Reporting secondary device attributes");
                let version = version_number(env!("CARGO_PKG_VERSION"));
                let text = format!("\x1b[>0;{version};1c");
                report_channel.extend(text.as_bytes());
            },
            _ => debug!("Unsupported device attributes insurfaceediate"),
        }
    }

    #[inline]
    fn report_device_status(
        &mut self,
        status: usize,
        report_channel: &mut VecDeque<u8>,
    ) {
        trace!("Reporting device status: {status}");
        match status {
            5 => {
                let text = String::from("\x1b[0n");
                report_channel.extend(text.as_bytes());
            },
            6 => {
                let pos = self.grid.cursor.point;
                let text = format!("\x1b[{};{}R", pos.line + 1, pos.column + 1);
                report_channel.extend(text.as_bytes());
            },
            _ => debug!("unknown device status query: {status}"),
        };
    }

    fn start_selection(&mut self, ty: SelectionType, point: Point, side: Side) {
        self.selection = Some(Selection::new(ty, point, side));
    }

    fn update_selection(&mut self, point: Point, side: Side) {
        self.selection = self.selection.take().map(|mut s| {
            s.update(point, side);
            s
        })
    }
}

/// The state of the [`Mode`] and [`PrivateMode`].
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum ModeState {
    /// The mode is not supported.
    NotSupported = 0,
    /// The mode is currently set.
    Set = 1,
    /// The mode is currently not set.
    Reset = 2,
}

impl From<bool> for ModeState {
    fn from(value: bool) -> Self {
        if value { Self::Set } else { Self::Reset }
    }
}

/// Surface version for escape sequence reports.
///
/// This returns the current surface version as a unique number based on otty-surface's
/// semver version. The different versions are padded to ensure that a higher semver version will
/// always report a higher version number.
fn version_number(mut version: &str) -> usize {
    if let Some(separator) = version.rfind('-') {
        version = &version[..separator];
    }

    let mut version_number = 0;

    let semver_versions = version.split('.');
    for (i, semver_version) in semver_versions.rev().enumerate() {
        let semver_number = semver_version.parse::<usize>().unwrap_or(0);
        version_number += usize::pow(100, i as u32) * semver_number;
    }

    version_number
}

struct TabStops {
    tabs: Vec<bool>,
}

impl TabStops {
    #[inline]
    fn new(columns: usize) -> TabStops {
        TabStops {
            tabs: (0..columns).map(|i| i % INITIAL_TABSTOPS == 0).collect(),
        }
    }

    /// Remove all tabstops.
    #[inline]
    fn clear_all(&mut self) {
        unsafe {
            ptr::write_bytes(self.tabs.as_mut_ptr(), 0, self.tabs.len());
        }
    }

    /// Increase tabstop capacity.
    #[inline]
    fn resize(&mut self, columns: usize) {
        let mut index = self.tabs.len();
        self.tabs.resize_with(columns, || {
            let is_tabstop = index.is_multiple_of(INITIAL_TABSTOPS);
            index += 1;
            is_tabstop
        });
    }
}

impl Index<Column> for TabStops {
    type Output = bool;

    fn index(&self, index: Column) -> &bool {
        &self.tabs[index.0]
    }
}

impl IndexMut<Column> for TabStops {
    fn index_mut(&mut self, index: Column) -> &mut bool {
        self.tabs.index_mut(index.0)
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use crate::cell::{Cell, Flags};
    use crate::damage::LineDamageBounds;
    use crate::grid::{Grid, Scroll};
    use crate::index::{Column, Point, Side};
    use crate::selection::{Selection, SelectionType};

    use super::*;

    pub struct SurfaceSize {
        pub columns: usize,
        pub screen_lines: usize,
    }

    impl SurfaceSize {
        pub fn new(columns: usize, screen_lines: usize) -> Self {
            Self {
                columns,
                screen_lines,
            }
        }
    }

    impl Dimensions for SurfaceSize {
        fn total_lines(&self) -> usize {
            self.screen_lines()
        }

        fn screen_lines(&self) -> usize {
            self.screen_lines
        }

        fn columns(&self) -> usize {
            self.columns
        }
    }

    #[test]
    fn scroll_display_page_up() {
        let size = SurfaceSize::new(5, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 11 lines of scrollback.
        for _ in 0..20 {
            surface.new_line();
        }

        // Scrollable amount to top is 11.
        surface.scroll_display(Scroll::PageUp);
        assert_eq!(surface.grid.display_offset(), 10);

        // Scrollable amount to top is 1.
        surface.scroll_display(Scroll::PageUp);
        assert_eq!(surface.grid.display_offset(), 11);

        // Scrollable amount to top is 0.
        surface.scroll_display(Scroll::PageUp);
        assert_eq!(surface.grid.display_offset(), 11);
    }

    #[test]
    fn scroll_display_page_down() {
        let size = SurfaceSize::new(5, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 11 lines of scrollback.
        for _ in 0..20 {
            surface.new_line();
        }

        // Change display_offset to topmost.
        surface.grid_mut().scroll_display(Scroll::Top);

        // Scrollable amount to bottom is 11.
        surface.scroll_display(Scroll::PageDown);
        assert_eq!(surface.grid.display_offset(), 1);

        // Scrollable amount to bottom is 1.
        surface.scroll_display(Scroll::PageDown);
        assert_eq!(surface.grid.display_offset(), 0);

        // Scrollable amount to bottom is 0.
        surface.scroll_display(Scroll::PageDown);
        assert_eq!(surface.grid.display_offset(), 0);
    }

    #[test]
    fn simple_selection_works() {
        let size = SurfaceSize::new(5, 5);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let grid = surface.grid_mut();
        for i in 0..4 {
            if i == 1 {
                continue;
            }

            grid[Line(i)][Column(0)].c = '"';

            for j in 1..4 {
                grid[Line(i)][Column(j)].c = 'a';
            }

            grid[Line(i)][Column(4)].c = '"';
        }
        grid[Line(2)][Column(0)].c = ' ';
        grid[Line(2)][Column(4)].c = ' ';
        grid[Line(2)][Column(4)].flags.insert(Flags::WRAPLINE);
        grid[Line(3)][Column(0)].c = ' ';

        // Multiple lines contain an empty line.
        surface.selection = Some(Selection::new(
            SelectionType::Simple,
            Point {
                line: Line(0),
                column: Column(0),
            },
            Side::Left,
        ));
        if let Some(s) = surface.selection.as_mut() {
            s.update(
                Point {
                    line: Line(2),
                    column: Column(4),
                },
                Side::Right,
            );
        }
        assert_eq!(
            surface.selection_to_string(),
            Some(String::from("\"aaa\"\n\n aaa "))
        );

        // A wrapline.
        surface.selection = Some(Selection::new(
            SelectionType::Simple,
            Point {
                line: Line(2),
                column: Column(0),
            },
            Side::Left,
        ));
        if let Some(s) = surface.selection.as_mut() {
            s.update(
                Point {
                    line: Line(3),
                    column: Column(4),
                },
                Side::Right,
            );
        }
        assert_eq!(
            surface.selection_to_string(),
            Some(String::from(" aaa  aaa\""))
        );
    }

    #[test]
    fn semantic_selection_works() {
        let size = SurfaceSize::new(5, 3);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let mut grid: Grid<Cell> = Grid::new(3, 5, 0);
        for i in 0..5 {
            for j in 0..2 {
                grid[Line(j)][Column(i)].c = 'a';
            }
        }
        grid[Line(0)][Column(0)].c = '"';
        grid[Line(0)][Column(3)].c = '"';
        grid[Line(1)][Column(2)].c = '"';
        grid[Line(0)][Column(4)].flags.insert(Flags::WRAPLINE);

        let mut escape_chars = String::from("\"");

        mem::swap(&mut surface.grid, &mut grid);
        mem::swap(&mut surface.config.semantic_escape_chars, &mut escape_chars);

        {
            surface.selection = Some(Selection::new(
                SelectionType::Semantic,
                Point {
                    line: Line(0),
                    column: Column(1),
                },
                Side::Left,
            ));
            assert_eq!(surface.selection_to_string(), Some(String::from("aa")));
        }

        {
            surface.selection = Some(Selection::new(
                SelectionType::Semantic,
                Point {
                    line: Line(0),
                    column: Column(4),
                },
                Side::Left,
            ));
            assert_eq!(
                surface.selection_to_string(),
                Some(String::from("aaa"))
            );
        }

        {
            surface.selection = Some(Selection::new(
                SelectionType::Semantic,
                Point {
                    line: Line(1),
                    column: Column(1),
                },
                Side::Left,
            ));
            assert_eq!(
                surface.selection_to_string(),
                Some(String::from("aaa"))
            );
        }
    }

    #[test]
    fn line_selection_works() {
        let size = SurfaceSize::new(5, 1);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let mut grid: Grid<Cell> = Grid::new(1, 5, 0);
        for i in 0..5 {
            grid[Line(0)][Column(i)].c = 'a';
        }
        grid[Line(0)][Column(0)].c = '"';
        grid[Line(0)][Column(3)].c = '"';

        mem::swap(&mut surface.grid, &mut grid);

        surface.selection = Some(Selection::new(
            SelectionType::Lines,
            Point {
                line: Line(0),
                column: Column(3),
            },
            Side::Left,
        ));
        assert_eq!(
            surface.selection_to_string(),
            Some(String::from("\"aa\"a\n"))
        );
    }

    #[test]
    fn block_selection_works() {
        let size = SurfaceSize::new(5, 5);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let grid = surface.grid_mut();
        for i in 1..4 {
            grid[Line(i)][Column(0)].c = '"';

            for j in 1..4 {
                grid[Line(i)][Column(j)].c = 'a';
            }

            grid[Line(i)][Column(4)].c = '"';
        }
        grid[Line(2)][Column(2)].c = ' ';
        grid[Line(2)][Column(4)].flags.insert(Flags::WRAPLINE);
        grid[Line(3)][Column(4)].c = ' ';

        surface.selection = Some(Selection::new(
            SelectionType::Block,
            Point {
                line: Line(0),
                column: Column(3),
            },
            Side::Left,
        ));

        // The same column.
        if let Some(s) = surface.selection.as_mut() {
            s.update(
                Point {
                    line: Line(3),
                    column: Column(3),
                },
                Side::Right,
            );
        }
        assert_eq!(
            surface.selection_to_string(),
            Some(String::from("\na\na\na"))
        );

        // The first column.
        if let Some(s) = surface.selection.as_mut() {
            s.update(
                Point {
                    line: Line(3),
                    column: Column(0),
                },
                Side::Left,
            );
        }
        assert_eq!(
            surface.selection_to_string(),
            Some(String::from("\n\"aa\n\"a\n\"aa"))
        );

        // The last column.
        if let Some(s) = surface.selection.as_mut() {
            s.update(
                Point {
                    line: Line(3),
                    column: Column(4),
                },
                Side::Right,
            );
        }
        assert_eq!(
            surface.selection_to_string(),
            Some(String::from("\na\"\na\"\na"))
        );
    }

    #[test]
    fn input_line_drawing_character() {
        let size = SurfaceSize::new(7, 17);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let cursor = Point::new(Line(0), Column(0));
        surface.configure_charset(Charset::DecLineDrawing, CharsetIndex::G0);
        surface.print('a');

        assert_eq!(surface.grid()[cursor].c, '▒');
    }

    #[test]
    fn clearing_viewport_keeps_history_position() {
        let size = SurfaceSize::new(10, 20);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 10 lines of scrollback.
        for _ in 0..29 {
            surface.new_line();
        }

        // Change the display area.
        surface.scroll_display(Scroll::Top);

        assert_eq!(surface.grid.display_offset(), 10);

        // Clear the viewport.
        surface.clear_screen(ClearMode::All);

        assert_eq!(surface.grid.display_offset(), 10);
    }

    #[test]
    fn clearing_scrollback_resets_display_offset() {
        let size = SurfaceSize::new(10, 20);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 10 lines of scrollback.
        for _ in 0..29 {
            surface.new_line();
        }

        // Change the display area.
        surface.scroll_display(Scroll::Top);

        assert_eq!(surface.grid.display_offset(), 10);

        // Clear the scrollback buffer.
        surface.clear_screen(ClearMode::Saved);

        assert_eq!(surface.grid.display_offset(), 0);
    }

    #[test]
    fn clear_saved_lines() {
        let size = SurfaceSize::new(7, 17);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Add one line of scrollback.
        surface.grid.scroll_up(&(Line(0)..Line(1)), 1);

        // Clear the history.
        surface.clear_screen(ClearMode::Saved);

        // Make sure that scrolling does not change the grid.
        let mut scrolled_grid = surface.grid.clone();
        scrolled_grid.scroll_display(Scroll::Top);

        // Truncate grids for comparison.
        scrolled_grid.truncate();
        surface.grid.truncate();

        assert_eq!(surface.grid, scrolled_grid);
    }

    #[test]
    fn grow_lines_updates_active_cursor_pos() {
        let mut size = SurfaceSize::new(100, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 10 lines of scrollback.
        for _ in 0..19 {
            surface.new_line();
        }
        assert_eq!(surface.history_size(), 10);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(9), Column(0)));

        // Increase visible lines.
        size.screen_lines = 30;
        surface.resize(size);

        assert_eq!(surface.history_size(), 0);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(19), Column(0)));
    }

    #[test]
    fn grow_lines_updates_inactive_cursor_pos() {
        let mut size = SurfaceSize::new(100, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 10 lines of scrollback.
        for _ in 0..19 {
            surface.new_line();
        }
        assert_eq!(surface.history_size(), 10);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(9), Column(0)));

        // Enter alt screen.
        surface.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );

        // Increase visible lines.
        size.screen_lines = 30;
        surface.resize(size);

        // Leave alt screen.
        surface.unset_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );

        assert_eq!(surface.history_size(), 0);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(19), Column(0)));
    }

    #[test]
    fn shrink_lines_updates_active_cursor_pos() {
        let mut size = SurfaceSize::new(100, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 10 lines of scrollback.
        for _ in 0..19 {
            surface.new_line();
        }
        assert_eq!(surface.history_size(), 10);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(9), Column(0)));

        // Increase visible lines.
        size.screen_lines = 5;
        surface.resize(size);

        assert_eq!(surface.history_size(), 15);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(4), Column(0)));
    }

    #[test]
    fn shrink_lines_updates_inactive_cursor_pos() {
        let mut size = SurfaceSize::new(100, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Create 10 lines of scrollback.
        for _ in 0..19 {
            surface.new_line();
        }
        assert_eq!(surface.history_size(), 10);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(9), Column(0)));

        // Enter alt screen.
        surface.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );

        // Increase visible lines.
        size.screen_lines = 5;
        surface.resize(size);

        // Leave alt screen.
        surface.unset_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );

        assert_eq!(surface.history_size(), 15);
        assert_eq!(surface.grid.cursor.point, Point::new(Line(4), Column(0)));
    }

    #[test]
    fn damage_public_usage() {
        let size = SurfaceSize::new(10, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        // Reset surface for partial damage tests since it's initialized as fully damaged.
        surface.reset_damage();

        // Test that we damage input form [`surface::input`].

        let left = surface.grid.cursor.point.column.0;
        surface.print('d');
        surface.print('a');
        surface.print('m');
        surface.print('a');
        surface.print('g');
        surface.print('e');
        let right = surface.grid.cursor.point.column.0;

        let mut damaged_lines = match surface.damage() {
            SurfaceDamage::Full => {
                panic!("Expected partial damage, however got Full")
            },
            SurfaceDamage::Partial(damaged_lines) => damaged_lines,
        };
        assert_eq!(
            damaged_lines.next(),
            Some(LineDamageBounds {
                line: 0,
                left,
                right
            })
        );
        assert_eq!(damaged_lines.next(), None);
        surface.reset_damage();

        // Create scrollback.
        for _ in 0..20 {
            surface.new_line();
        }

        match surface.damage() {
            SurfaceDamage::Full => (),
            SurfaceDamage::Partial(_) => {
                panic!("Expected Full damage, however got Partial ")
            },
        };
        surface.reset_damage();

        surface.scroll_display(Scroll::Delta(10));
        surface.reset_damage();

        // No damage when scrolled into viewport.
        for idx in 0..surface.columns() {
            surface.goto(idx as i32, idx);
        }
        let mut damaged_lines = match surface.damage() {
            SurfaceDamage::Full => {
                panic!("Expected partial damage, however got Full")
            },
            SurfaceDamage::Partial(damaged_lines) => damaged_lines,
        };
        assert_eq!(damaged_lines.next(), None);

        // Scroll back into the viewport, so we have 2 visible lines which the surface can write
        // to.
        surface.scroll_display(Scroll::Delta(-2));
        surface.reset_damage();

        surface.goto(0, 0);
        surface.goto(1, 0);
        surface.goto(2, 0);
        let display_offset = surface.grid().display_offset();
        let mut damaged_lines = match surface.damage() {
            SurfaceDamage::Full => {
                panic!("Expected partial damage, however got Full")
            },
            SurfaceDamage::Partial(damaged_lines) => damaged_lines,
        };
        assert_eq!(
            damaged_lines.next(),
            Some(LineDamageBounds {
                line: display_offset,
                left: 0,
                right: 0
            })
        );
        assert_eq!(
            damaged_lines.next(),
            Some(LineDamageBounds {
                line: display_offset + 1,
                left: 0,
                right: 0
            })
        );
        assert_eq!(damaged_lines.next(), None);
    }

    #[test]
    fn damage_cursor_movements() {
        let size = SurfaceSize::new(10, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let num_cols = surface.columns();
        // Reset surface for partial damage tests since it's initialized as fully damaged.
        surface.reset_damage();

        surface.goto(1, 1);

        // NOTE While we can use `[surface::damage]` to access surface damage information, in the
        // following tests we will be accessing `surface.damage.lines` directly to avoid adding extra
        // damage information (like cursor and Vi cursor), which we're not testing.

        assert_eq!(
            surface.damage.lines[0],
            LineDamageBounds {
                line: 0,
                left: 0,
                right: 0
            }
        );
        assert_eq!(
            surface.damage.lines[1],
            LineDamageBounds {
                line: 1,
                left: 1,
                right: 1
            }
        );
        surface.damage.reset(num_cols);

        surface.move_forward(3);
        assert_eq!(
            surface.damage.lines[1],
            LineDamageBounds {
                line: 1,
                left: 1,
                right: 4
            }
        );
        surface.damage.reset(num_cols);

        surface.move_backward(8);
        assert_eq!(
            surface.damage.lines[1],
            LineDamageBounds {
                line: 1,
                left: 0,
                right: 4
            }
        );
        surface.goto(5, 5);
        surface.damage.reset(num_cols);

        surface.backspace();
        surface.backspace();
        assert_eq!(
            surface.damage.lines[5],
            LineDamageBounds {
                line: 5,
                left: 3,
                right: 5
            }
        );
        surface.damage.reset(num_cols);

        surface.move_up(1, false);
        assert_eq!(
            surface.damage.lines[5],
            LineDamageBounds {
                line: 5,
                left: 3,
                right: 3
            }
        );
        assert_eq!(
            surface.damage.lines[4],
            LineDamageBounds {
                line: 4,
                left: 3,
                right: 3
            }
        );
        surface.damage.reset(num_cols);

        surface.move_down(1, false);
        surface.move_down(1, false);
        assert_eq!(
            surface.damage.lines[4],
            LineDamageBounds {
                line: 4,
                left: 3,
                right: 3
            }
        );
        assert_eq!(
            surface.damage.lines[5],
            LineDamageBounds {
                line: 5,
                left: 3,
                right: 3
            }
        );
        assert_eq!(
            surface.damage.lines[6],
            LineDamageBounds {
                line: 6,
                left: 3,
                right: 3
            }
        );
        surface.damage.reset(num_cols);

        surface.wrapline();
        assert_eq!(
            surface.damage.lines[6],
            LineDamageBounds {
                line: 6,
                left: 3,
                right: 3
            }
        );
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 0,
                right: 0
            }
        );
        surface.move_forward(3);
        surface.move_up(1, false);
        surface.damage.reset(num_cols);

        surface.line_feed();
        assert_eq!(
            surface.damage.lines[6],
            LineDamageBounds {
                line: 6,
                left: 3,
                right: 3
            }
        );
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 3,
                right: 3
            }
        );
        surface.damage.reset(num_cols);

        surface.carriage_return();
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 0,
                right: 3
            }
        );
        surface.damage.reset(num_cols);

        surface.erase_chars(5);
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 0,
                right: 5
            }
        );
        surface.damage.reset(num_cols);

        surface.delete_chars(3);
        let right = surface.columns() - 1;
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 0,
                right
            }
        );
        surface.move_forward(surface.columns());
        surface.damage.reset(num_cols);

        surface.move_backward_tabs(1);
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 8,
                right
            }
        );
        surface.save_cursor();
        surface.goto(1, 1);
        surface.damage.reset(num_cols);

        surface.restore_cursor();
        assert_eq!(
            surface.damage.lines[1],
            LineDamageBounds {
                line: 1,
                left: 1,
                right: 1
            }
        );
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 8,
                right: 8
            }
        );
        surface.damage.reset(num_cols);

        surface.clear_line(LineClearMode::All);
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 0,
                right
            }
        );
        surface.damage.reset(num_cols);

        surface.clear_line(LineClearMode::Left);
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 0,
                right: 8
            }
        );
        surface.damage.reset(num_cols);

        surface.clear_line(LineClearMode::Right);
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 8,
                right
            }
        );
        surface.damage.reset(num_cols);

        surface.reverse_index();
        assert_eq!(
            surface.damage.lines[7],
            LineDamageBounds {
                line: 7,
                left: 8,
                right: 8
            }
        );
        assert_eq!(
            surface.damage.lines[6],
            LineDamageBounds {
                line: 6,
                left: 8,
                right: 8
            }
        );
    }

    #[test]
    fn full_damage() {
        let size = SurfaceSize::new(100, 10);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        assert!(surface.damage.full);
        for _ in 0..20 {
            surface.new_line();
        }
        surface.reset_damage();

        surface.clear_screen(ClearMode::Above);
        assert!(surface.damage.full);
        surface.reset_damage();

        surface.scroll_display(Scroll::Top);
        assert!(surface.damage.full);
        surface.reset_damage();

        // Sequential call to scroll display without doing anything shouldn't damage.
        surface.scroll_display(Scroll::Top);
        assert!(!surface.damage.full);
        surface.reset_damage();

        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        assert!(surface.damage.full);
        surface.reset_damage();

        surface.scroll_down_relative(Line(5), 2);
        assert!(surface.damage.full);
        surface.reset_damage();

        surface.scroll_up_relative(Line(3), 2);
        assert!(surface.damage.full);
        surface.reset_damage();

        surface.deccolm();
        assert!(surface.damage.full);
        surface.reset_damage();

        surface.screen_alignment_display();
        assert!(surface.damage.full);
        surface.reset_damage();

        surface.set_mode(NamedMode::Insert.into());
        // Just setting `Insert` mode shouldn't mark the surface as damaged.
        assert!(!surface.damage.full);
        surface.reset_damage();

        let color_index = 257;
        surface.set_color(color_index, Rgb::default());
        assert!(surface.damage.full);
        surface.reset_damage();

        // Setting the same color once again shouldn't trigger full damage.
        surface.set_color(color_index, Rgb::default());
        assert!(!surface.damage.full);

        surface.reset_color(color_index);
        assert!(surface.damage.full);
        surface.reset_damage();

        // We shouldn't trigger fully damage when cursor gets update.
        surface.set_color(StdColor::Cursor as usize, Rgb::default());
        assert!(!surface.damage.full);

        // However requesting surface damage should mark the surface as fully damaged in `Insert`
        // mode.
        let _ = surface.damage();
        assert!(surface.damage.full);
        surface.reset_damage();

        surface.unset_mode(NamedMode::Insert.into());
        assert!(surface.damage.full);
        surface.reset_damage();

        // Keep this as a last check, so we don't have to deal with restoring from alt-screen.
        surface.swap_altscreen();
        assert!(surface.damage.full);
        surface.reset_damage();

        let size = SurfaceSize::new(10, 10);
        surface.resize(size);
        assert!(surface.damage.full);
    }

    #[test]
    fn window_title() {
        let size = SurfaceSize::new(7, 17);
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        // Title None by default.
        assert_eq!(surface.title, None);

        // Title can be set.
        surface.set_window_title(Some("Test".into()));
        assert_eq!(surface.title, Some("Test".into()));

        // Title can be pushed onto stack.
        surface.push_window_title();
        surface.set_window_title(Some("Next".into()));
        assert_eq!(surface.title, Some("Next".into()));
        assert_eq!(surface.title_stack.first().unwrap(), &Some("Test".into()));

        // Title can be popped from stack and set as the window title.
        surface.pop_window_title();
        assert_eq!(surface.title, Some("Test".into()));
        assert!(surface.title_stack.is_empty());

        // Title stack doesn't grow infinitely.
        for _ in 0..4097 {
            surface.push_window_title();
        }
        assert_eq!(surface.title_stack.len(), 4096);

        // Title and title stack reset when surface state is reset.
        surface.push_window_title();
        surface.reset();
        assert_eq!(surface.title, None);
        assert!(surface.title_stack.is_empty());

        // Title stack pops back to default.
        surface.title = None;
        surface.push_window_title();
        surface.set_window_title(Some("Test".into()));
        surface.pop_window_title();
        assert_eq!(surface.title, None);
    }

    #[test]
    fn parse_cargo_version() {
        assert_eq!(version_number("0.0.1-dev"), 1);
        assert_eq!(version_number("0.1.2-dev"), 1_02);
        assert_eq!(version_number("1.2.3-dev"), 1_02_03);
        assert_eq!(version_number("999.99.99"), 9_99_99_99);
    }
}
