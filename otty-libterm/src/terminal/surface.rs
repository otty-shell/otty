//! Exports the `Term` type which is a high-level API for the Grid.

use std::ops::{Index, IndexMut, Range};
use std::sync::Arc;
use std::time::Instant;
use std::{cmp, mem, ptr, slice, str};

use cursor_icon::CursorIcon;
use otty_escape::{
    Action, CharacterAttribute, Charset, CharsetIndex, ClearMode, Color,
    CursorShape, CursorStyle, EscapeActor, Hyperlink, KeyboardMode,
    KeyboardModeApplyBehavior, LineClearMode, Mode, NamedMode,
    NamedPrivateMode, PrivateMode, Rgb, StdColor, TabClearMode,
};

use log::{debug, trace};
use unicode_width::UnicodeWidthChar;

use crate::terminal::mode::TerminalMode;

use super::actor::SurfaceActor;
// use crate::event::{Event, EventListener};
use crate::grid::{Dimensions, Grid, GridIterator, Scroll};
use crate::terminal::index::{self, Boundary, Column, Direction, Line, Point, Side};
// use crate::selection::{Selection, SelectionRange, SelectionType};
use crate::terminal::cell::{Cell, Flags, LineLength};
use crate::terminal::color::Colors;
// use crate::vi_mode::{ViModeCursor, ViMotion};

/// Minimum number of columns.
///
/// A minimum of 2 is necessary to hold fullwidth unicode characters.
pub const MIN_COLUMNS: usize = 2;

/// Minimum number of visible lines.
pub const MIN_SCREEN_LINES: usize = 1;

/// Max size of the window title stack.
const TITLE_STACK_MAX_DEPTH: usize = 4096;

/// Max size of the keyboard modes.
const KEYBOARD_MODE_STACK_MAX_DEPTH: usize = TITLE_STACK_MAX_DEPTH;

/// Default semantic escape characters.
pub const SEMANTIC_ESCAPE_CHARS: &str = ",│`|:\"' ()[]{}<>\t";

/// Default tab interval, corresponding to terminfo `it` value.
const INITIAL_TABSTOPS: usize = 8;

/// Convert a terminal point to a viewport relative point.
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

/// Convert a viewport relative point to a terminal point.
#[inline]
pub fn viewport_to_point(display_offset: usize, point: Point<usize>) -> Point {
    let line = Line(point.line as i32) - display_offset;
    Point::new(line, point.column)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineDamageBounds {
    /// Damaged line number.
    pub line: usize,

    /// Leftmost damaged column.
    pub left: usize,

    /// Rightmost damaged column.
    pub right: usize,
}

impl LineDamageBounds {
    #[inline]
    pub fn new(line: usize, left: usize, right: usize) -> Self {
        Self { line, left, right }
    }

    #[inline]
    pub fn undamaged(line: usize, num_cols: usize) -> Self {
        Self {
            line,
            left: num_cols,
            right: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self, num_cols: usize) {
        *self = Self::undamaged(self.line, num_cols);
    }

    #[inline]
    pub fn expand(&mut self, left: usize, right: usize) {
        self.left = cmp::min(self.left, left);
        self.right = cmp::max(self.right, right);
    }

    #[inline]
    pub fn is_damaged(&self) -> bool {
        self.left <= self.right
    }
}

/// Terminal damage information collected since the last [`Term::reset_damage`] call.
#[derive(Debug)]
pub enum TermDamage<'a> {
    /// The entire terminal is damaged.
    Full,

    /// Iterator over damaged lines in the terminal.
    Partial(TermDamageIterator<'a>),
}

/// Iterator over the terminal's viewport damaged lines.
#[derive(Clone, Debug)]
pub struct TermDamageIterator<'a> {
    line_damage: slice::Iter<'a, LineDamageBounds>,
    display_offset: usize,
}

impl<'a> TermDamageIterator<'a> {
    pub fn new(
        line_damage: &'a [LineDamageBounds],
        display_offset: usize,
    ) -> Self {
        let num_lines = line_damage.len();
        // Filter out invisible damage.
        let line_damage =
            &line_damage[..num_lines.saturating_sub(display_offset)];
        Self {
            display_offset,
            line_damage: line_damage.iter(),
        }
    }
}

impl Iterator for TermDamageIterator<'_> {
    type Item = LineDamageBounds;

    fn next(&mut self) -> Option<Self::Item> {
        self.line_damage.find_map(|line| {
            line.is_damaged().then_some(LineDamageBounds::new(
                line.line + self.display_offset,
                line.left,
                line.right,
            ))
        })
    }
}

/// State of the terminal damage.
struct TermDamageState {
    /// Hint whether terminal should be damaged entirely regardless of the actual damage changes.
    full: bool,

    /// Information about damage on terminal lines.
    lines: Vec<LineDamageBounds>,

    /// Old terminal cursor point.
    last_cursor: Point,
}

impl TermDamageState {
    fn new(num_cols: usize, num_lines: usize) -> Self {
        let lines = (0..num_lines)
            .map(|line| LineDamageBounds::undamaged(line, num_cols))
            .collect();

        Self {
            full: true,
            lines,
            last_cursor: Default::default(),
        }
    }

    #[inline]
    fn resize(&mut self, num_cols: usize, num_lines: usize) {
        // Reset point, so old cursor won't end up outside of the viewport.
        self.last_cursor = Default::default();
        self.full = true;

        self.lines.clear();
        self.lines.reserve(num_lines);
        for line in 0..num_lines {
            self.lines.push(LineDamageBounds::undamaged(line, num_cols));
        }
    }

    /// Damage point inside of the viewport.
    #[inline]
    fn damage_point(&mut self, point: Point<usize>) {
        self.damage_line(point.line, point.column.0, point.column.0);
    }

    /// Expand `line`'s damage to span at least `left` to `right` column.
    #[inline]
    fn damage_line(&mut self, line: usize, left: usize, right: usize) {
        self.lines[line].expand(left, right);
    }

    /// Reset information about terminal damage.
    fn reset(&mut self, num_cols: usize) {
        self.full = false;
        self.lines.iter_mut().for_each(|line| line.reset(num_cols));
    }
}

/// Maximum number of actions to buffer during synchronized update (2MB equivalent).
const MAX_SYNC_ACTIONS: usize = 10000;

pub struct Surface {
    /// Terminal focus controlling the cursor shape.
    pub is_focused: bool,

    // pub selection: Option<Selection>,
    /// Currently active grid.
    ///
    /// Tracks the screen buffer currently in use. While the alternate screen buffer is active,
    /// this will be the alternate grid. Otherwise it is the primary screen buffer.
    grid: Grid<Cell>,

    /// Currently inactive grid.
    ///
    /// Opposite of the active grid. While the alternate screen buffer is active, this will be the
    /// primary grid. Otherwise it is the alternate screen buffer.
    inactive_grid: Grid<Cell>,

    /// Index into `charsets`, pointing to what ASCII is currently being mapped to.
    active_charset: CharsetIndex,

    /// Tabstops.
    tabs: TabStops,

    /// Mode flags.
    mode: TerminalMode,

    /// Scroll region.
    ///
    /// Range going from top to bottom of the terminal, indexed from the top of the viewport.
    scroll_region: Range<Line>,

    /// Modified terminal colors.
    colors: Colors,

    /// Current style of the cursor.
    cursor_style: Option<CursorStyle>,

    /// Current title of the window.
    title: Option<String>,

    /// Stack of saved window titles. When a title is popped from this stack, the `title` for the
    /// term is set.
    title_stack: Vec<Option<String>>,

    /// The stack for the keyboard modes.
    keyboard_mode_stack: Vec<KeyboardMode>,

    /// Currently inactive keyboard mode stack.
    inactive_keyboard_mode_stack: Vec<KeyboardMode>,

    /// Information about damaged cells.
    damage: TermDamageState,

    /// Config directly for the terminal.
    config: SurfaceConfig,
}

/// Configuration options for the [`Term`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceConfig {
    /// The maximum amount of scrolling history.
    pub scrolling_history: usize,

    /// Default cursor style to reset the cursor to.
    pub default_cursor_style: CursorStyle,

    /// The characters which terminate semantic selection.
    ///
    /// The default value is [`SEMANTIC_ESCAPE_CHARS`].
    pub semantic_escape_chars: String,

    /// Whether to enable kitty keyboard protocol.
    pub kitty_keyboard: bool,

    /// OSC52 support mode.
    pub osc52: Osc52,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            scrolling_history: 10000,
            semantic_escape_chars: SEMANTIC_ESCAPE_CHARS.to_owned(),
            default_cursor_style: Default::default(),
            kitty_keyboard: Default::default(),
            osc52: Default::default(),
        }
    }
}

/// OSC 52 behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Osc52 {
    /// The handling of the escape sequence is disabled.
    Disabled,
    /// Only copy sequence is accepted.
    ///
    /// This option is the default as a compromise between entirely
    /// disabling it (the most secure) and allowing `paste` (the less secure).
    #[default]
    OnlyCopy,
    /// Only paste sequence is accepted.
    OnlyPaste,
    /// Both are accepted.
    CopyPaste,
}

impl Surface {
    pub fn new<D: Dimensions>(
        config: SurfaceConfig,
        dimensions: &D,
        // event_proxy: T,
    ) -> Surface {
        let num_cols = dimensions.columns();
        let num_lines = dimensions.screen_lines();

        let history_size = config.scrolling_history;
        let grid = Grid::new(num_lines, num_cols, history_size);
        let inactive_grid = Grid::new(num_lines, num_cols, 0);

        let tabs = TabStops::new(grid.columns());

        let scroll_region = Line(0)..Line(grid.screen_lines() as i32);

        // Initialize terminal damage, covering the entire terminal upon launch.
        let damage = TermDamageState::new(num_cols, num_lines);

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
            // selection: Default::default(),
            title: Default::default(),
            mode: Default::default(),
        }
    }

    /// Collect the information about the changes in the lines, which
    /// could be used to minimize the amount of drawing operations.
    ///
    /// The user controlled elements, like `Vi` mode cursor and `Selection` are **not** part of the
    /// collected damage state. Those could easily be tracked by comparing their old and new
    /// value between adjacent frames.
    ///
    /// After reading damage [`reset_damage`] should be called.
    ///
    /// [`reset_damage`]: Self::reset_damage
    #[must_use]
    pub fn damage(&mut self) -> TermDamage<'_> {
        // Ensure the entire terminal is damaged after entering insert mode.
        // Leaving is handled in the ansi handler.
        if self.mode.contains(TerminalMode::INSERT) {
            self.mark_fully_damaged();
        }

        let previous_cursor =
            mem::replace(&mut self.damage.last_cursor, self.grid.cursor.point);

        if self.damage.full {
            return TermDamage::Full;
        }

        // Add information about old cursor position and new one if they are not the same, so we
        // cover everything that was produced by `Term::input`.
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
        TermDamage::Partial(TermDamageIterator::new(
            &self.damage.lines,
            display_offset,
        ))
    }

    /// Resets the terminal damage information.
    pub fn reset_damage(&mut self) {
        self.damage.reset(self.columns());
    }

    #[inline]
    fn mark_fully_damaged(&mut self) {
        self.damage.full = true;
    }

    // TODO: selection
    /// Convert the active selection to a String.
    // pub fn selection_to_string(&self) -> Option<String> {
    //     let selection_range =
    //         self.selection.as_ref().and_then(|s| s.to_range(self))?;
    //     let SelectionRange { start, end, .. } = selection_range;

    //     let mut res = String::new();

    //     match self.selection.as_ref() {
    //         Some(Selection {
    //             ty: SelectionType::Block,
    //             ..
    //         }) => {
    //             for line in (start.line.0..end.line.0).map(Line::from) {
    //                 res += self
    //                     .line_to_string(
    //                         line,
    //                         start.column..end.column,
    //                         start.column.0 != 0,
    //                     )
    //                     .trim_end();
    //                 res += "\n";
    //             }

    //             res += self
    //                 .line_to_string(end.line, start.column..end.column, true)
    //                 .trim_end();
    //         },
    //         Some(Selection {
    //             ty: SelectionType::Lines,
    //             ..
    //         }) => {
    //             res = self.bounds_to_string(start, end) + "\n";
    //         },
    //         _ => {
    //             res = self.bounds_to_string(start, end);
    //         },
    //     }

    //     Some(res)
    // }

    /// Convert range between two points to a String.
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

    // TODO: snapshot
    /// Terminal content required for rendering.
    #[inline]
    pub fn renderable_content(&self) -> SurfaceSnapshot<'_>
// where
    //     T: EventListener,
    {
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

    /// Resize terminal to new dimensions.
    pub fn resize<S: Dimensions>(&mut self, size: S) {
        let old_cols = self.columns();
        let old_lines = self.screen_lines();

        let num_cols = size.columns();
        let num_lines = size.screen_lines();

        if old_cols == num_cols && old_lines == num_lines {
            debug!("Term::resize dimensions unchanged");
            return;
        }

        debug!("New num_cols is {num_cols} and num_lines is {num_lines}");

        // Move vi mode cursor with the content.
        // let history_size = self.history_size();
        // let mut delta = num_lines as i32 - old_lines as i32;
        // let min_delta =
        //     cmp::min(0, num_lines as i32 - self.grid.cursor.point.line.0 - 1);
        // delta = cmp::min(cmp::max(delta, min_delta), history_size as i32);
        // self.vi_mode_cursor.point.line += delta;

        let is_alt = self.mode.contains(TerminalMode::ALT_SCREEN);
        self.grid.resize(!is_alt, num_lines, num_cols);
        self.inactive_grid.resize(is_alt, num_lines, num_cols);

        // Invalidate selection and tabs only when necessary.
        if old_cols != num_cols {
            // self.selection = None;

            // Recreate tabs list.
            self.tabs.resize(num_cols);
        }
        // else if let Some(selection) = self.selection.take() {
        //     let max_lines = cmp::max(num_lines, old_lines) as i32;
        //     let range = Line(0)..Line(max_lines);
        //     self.selection = selection.rotate(self, &range, -delta);
        // }

        // Clamp vi cursor to viewport.
        // let viewport_top = Line(-(self.grid.display_offset() as i32));
        // let viewport_bottom = viewport_top + self.bottommost_line();

        // Reset scrolling region.
        self.scroll_region = Line(0)..Line(self.screen_lines() as i32);

        // Resize damage information.
        self.damage.resize(num_cols, num_lines);
    }

    /// Active terminal modes.
    #[inline]
    pub fn mode(&self) -> &TerminalMode {
        &self.mode
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
        // self.selection = self
        //     .selection
        //     .take()
        //     .and_then(|s| s.rotate(self, &region, -(lines as i32)));

        // Scroll vi mode cursor.
        // let line = &mut self.vi_mode_cursor.point.line;
        // if region.start <= *line && region.end > *line {
        //     *line = cmp::min(*line + lines, region.end - 1);
        // }

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
        // self.selection = self
        //     .selection
        //     .take()
        //     .and_then(|s| s.rotate(self, &region, lines as i32));

        self.grid.scroll_up(&region, lines);

        // Scroll vi mode cursor.
        // let viewport_top = Line(-(self.grid.display_offset() as i32));
        // let top = if region.start == 0 {
        //     viewport_top
        // } else {
        //     region.start
        // };
        // TODO: delete
        // let line = &mut self.vi_mode_cursor.point.line;
        // if (top <= *line) && region.end > *line {
        //     *line = cmp::max(*line - lines, top);
        // }
        self.mark_fully_damaged();
    }

    // TODO: delete
    // #[inline]
    // pub fn exit(&mut self)
    // where
    //     T: EventListener,
    // {
    //     self.event_proxy.send_event(Event::Exit);
    // }

    /// Scroll display to point if it is outside of viewport.
    pub fn scroll_to_point(&mut self, point: Point)
    // where
    //     T: EventListener,
    {
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

    #[inline]
    pub fn semantic_escape_chars(&self) -> &str {
        &self.config.semantic_escape_chars
    }

    #[cfg(test)]
    pub(crate) fn set_semantic_escape_chars(
        &mut self,
        semantic_escape_chars: &str,
    ) {
        self.config.semantic_escape_chars = semantic_escape_chars.into();
    }

    /// Active terminal cursor style.
    ///
    /// While vi mode is active, this will automatically return the vi mode cursor style.
    #[inline]
    pub fn cursor_style(&self) -> CursorStyle {
        let cursor_style = self
            .cursor_style
            .unwrap_or(self.config.default_cursor_style);

        // if self.mode.contains(TerminalMode::VI) {
        //     self.config.vi_mode_cursor_style.unwrap_or(cursor_style)
        // } else {
        //     cursor_style
        // }

        cursor_style
    }

    pub fn colors(&self) -> &Colors {
        &self.colors
    }

    /// Insert a linebreak at the current cursor position.
    #[inline]
    fn wrapline(&mut self)
    // where
    //     T: EventListener,
    {
        if !self.mode.contains(TerminalMode::LINE_WRAP) {
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
        mode: TerminalMode,
        apply: KeyboardModeApplyBehavior,
    ) {
        let active_mode = self.mode & TerminalMode::KITTY_KEYBOARD_PROTOCOL;
        self.mode &= !TerminalMode::KITTY_KEYBOARD_PROTOCOL;
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

    // #[inline]
    // pub(crate) fn timeout(&self) -> Option<Instant> {
    //     self.sync_state.timeout
    // }
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

// TODO: to libterm
// impl<T: EventListener> Actor for Term<T> {
//     #[inline]
//     fn begin_sync(&mut self) {
//         if self.config.enable_sync_updates {
//             trace!("[sync] Beginning synchronized update");
//             self.sync_state.begin();
//         }
//     }

//     #[inline]
//     fn end_sync(&mut self) {
//         trace!(
//             "[sync] Ending synchronized update, flushing {} actions",
//             self.sync_state.buffer.len()
//         );
//         let buffered_actions = self.sync_state.end();
//         self.handle_actions_batch(buffered_actions);
//     }

//     #[inline]
//     fn handle(&mut self, action: otty_escape::Action) {
//         if !self.sync_state.active {
//             self.handle_action_internal(action);
//             return;
//         }

//         self.sync_state.buffer.push(action);

//         if self.sync_state.is_expired()
//             || self.sync_state.buffer.len() >= MAX_SYNC_ACTIONS
//         {
//             let buffered = std::mem::take(&mut self.sync_state.buffer);
//             self.sync_state.active = false;
//             self.handle_actions_batch(buffered);
//         }
//     }
// }

impl Surface {
    // TODO: to libterm
    // #[inline]
    // fn identify_terminal(&mut self, attr: Option<char>) {
    //     match attr {
    //         None => {
    //             trace!("Reporting primary device attributes");
    //             let text = String::from("\x1b[?6c");
    //             self.event_proxy.send_event(Event::PtyWrite(text));
    //         },
    //         Some('>') => {
    //             trace!("Reporting secondary device attributes");
    //             let version = version_number(env!("CARGO_PKG_VERSION"));
    //             let text = format!("\x1b[>0;{version};1c");
    //             self.event_proxy.send_event(Event::PtyWrite(text));
    //         },
    //         _ => debug!("Unsupported device attributes intermediate"),
    //     }
    // }

    // TODO: to libterm
    // #[inline]
    // fn report_keyboard_mode(&mut self) {
    //     if !self.config.kitty_keyboard {
    //         return;
    //     }

    //     trace!("Reporting active keyboard mode");
    //     let current_mode = self
    //         .keyboard_mode_stack
    //         .last()
    //         .unwrap_or(&KeyboardMode::NO_MODE)
    //         .bits();
    //     let text = format!("\x1b[?{current_mode}u");
    //     self.event_proxy.send_event(Event::PtyWrite(text));
    // }

    // TODO: to libterm
    // #[inline]
    // fn push_keyboard_mode(&mut self, mode: KeyboardMode) {
    //     if !self.config.kitty_keyboard {
    //         return;
    //     }

    //     trace!("Pushing `{mode:?}` keyboard mode into the stack");

    //     if self.keyboard_mode_stack.len() >= KEYBOARD_MODE_STACK_MAX_DEPTH {
    //         let removed = self.title_stack.remove(0);
    //         trace!(
    //             "Removing '{removed:?}' from bottom of keyboard mode stack that exceeds its \
    //             maximum depth"
    //         );
    //     }

    //     self.keyboard_mode_stack.push(mode);
    //     self.set_keyboard_mode(mode.into(), KeyboardModeApplyBehavior::Replace);
    // }

    // TODO: to libterm
    // #[inline]
    // fn pop_keyboard_modes(&mut self, count: u16) {
    //     if !self.config.kitty_keyboard {
    //         return;
    //     }

    //     trace!("Attempting to pop {count} keyboard modes from the stack");
    //     let new_len = self
    //         .keyboard_mode_stack
    //         .len()
    //         .saturating_sub(count as usize);
    //     self.keyboard_mode_stack.truncate(new_len);

    //     // Reload active mode.
    //     let mode = self
    //         .keyboard_mode_stack
    //         .last()
    //         .copied()
    //         .unwrap_or(KeyboardMode::NO_MODE);
    //     self.set_keyboard_mode(mode.into(), KeyboardModeApplyBehavior::Replace);
    // }

    // TODO: to libterm
    // #[inline]
    // fn report_device_status(&mut self, arg: usize) {
    //     trace!("Reporting device status: {arg}");
    //     match arg {
    //         5 => {
    //             let text = String::from("\x1b[0n");
    //             self.event_proxy.send_event(Event::PtyWrite(text));
    //         },
    //         6 => {
    //             let pos = self.grid.cursor.point;
    //             let text = format!("\x1b[{};{}R", pos.line + 1, pos.column + 1);
    //             self.event_proxy.send_event(Event::PtyWrite(text));
    //         },
    //         _ => debug!("unknown device status query: {arg}"),
    //     };
    // }

    // TODO: to libterm
    // #[inline]
    // fn bell(&mut self) {
    //     trace!("Bell");
    //     self.event_proxy.send_event(Event::Bell);
    // }

    // TODO: delete
    // #[inline]
    // fn set_private_mode(&mut self, mode: PrivateMode) {
    //     let mode = match mode {
    //         PrivateMode::Named(mode) => mode,
    //         PrivateMode::Unknown(mode) => {
    //             debug!("Ignoring unknown mode {mode} in set_private_mode");
    //             return;
    //         },
    //     };

    //     trace!("Setting private mode: {mode:?}");
    //     match mode {
    //         NamedPrivateMode::UrgencyHints => {
    //             self.mode.insert(TerminalMode::URGENCY_HINTS)
    //         },
    //         NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                // if !self.mode.contains(TerminalMode::ALT_SCREEN) {
                //     self.swap_alt();
                // }
    //         },
    //         NamedPrivateMode::ShowCursor => {
    //             self.mode.insert(TerminalMode::SHOW_CURSOR)
    //         },
    //         NamedPrivateMode::CursorKeys => {
    //             self.mode.insert(TerminalMode::APP_CURSOR)
    //         },
    //         // Mouse protocols are mutually exclusive.
    //         NamedPrivateMode::ReportMouseClicks => {
    //             self.mode.remove(TerminalMode::MOUSE_MODE);
    //             self.mode.insert(TerminalMode::MOUSE_REPORT_CLICK);
    //             self.event_proxy.send_event(Event::MouseCursorDirty);
    //         },
    //         NamedPrivateMode::ReportCellMouseMotion => {
    //             self.mode.remove(TerminalMode::MOUSE_MODE);
    //             self.mode.insert(TerminalMode::MOUSE_DRAG);
    //             self.event_proxy.send_event(Event::MouseCursorDirty);
    //         },
    //         NamedPrivateMode::ReportAllMouseMotion => {
    //             self.mode.remove(TerminalMode::MOUSE_MODE);
    //             self.mode.insert(TerminalMode::MOUSE_MOTION);
    //             self.event_proxy.send_event(Event::MouseCursorDirty);
    //         },
    //         NamedPrivateMode::ReportFocusInOut => {
    //             self.mode.insert(TerminalMode::FOCUS_IN_OUT)
    //         },
    //         NamedPrivateMode::BracketedPaste => {
    //             self.mode.insert(TerminalMode::BRACKETED_PASTE)
    //         },
    //         // Mouse encodings are mutually exclusive.
    //         NamedPrivateMode::SgrMouse => {
    //             self.mode.remove(TerminalMode::UTF8_MOUSE);
    //             self.mode.insert(TerminalMode::SGR_MOUSE);
    //         },
    //         NamedPrivateMode::Utf8Mouse => {
    //             self.mode.remove(TerminalMode::SGR_MOUSE);
    //             self.mode.insert(TerminalMode::UTF8_MOUSE);
    //         },
    //         NamedPrivateMode::AlternateScroll => {
    //             self.mode.insert(TerminalMode::ALTERNATE_SCROLL)
    //         },
    //         NamedPrivateMode::LineWrap => self.mode.insert(TerminalMode::LINE_WRAP),
    //         NamedPrivateMode::Origin => {
    //             self.mode.insert(TerminalMode::ORIGIN);
    //             self.handle(Action::Goto(0, 0));
    //         },
    //         NamedPrivateMode::ColumnMode => self.deccolm(),
    //         NamedPrivateMode::BlinkingCursor => {
    //             let style = self
    //                 .cursor_style
    //                 .get_or_insert(self.config.default_cursor_style);
    //             style.blinking = true;
    //             self.event_proxy.send_event(Event::CursorBlinkingChange);
    //         },
    //         NamedPrivateMode::SyncUpdate => (),
    //     }
    // }

    // #[inline]
    // fn unset_private_mode(&mut self, mode: PrivateMode) {
    //     let mode = match mode {
    //         PrivateMode::Named(mode) => mode,
    //         PrivateMode::Unknown(mode) => {
    //             debug!("Ignoring unknown mode {mode} in unset_private_mode");
    //             return;
    //         },
    //     };

    //     trace!("Unsetting private mode: {mode:?}");
    //     match mode {
    //         NamedPrivateMode::UrgencyHints => {
    //             self.mode.remove(TerminalMode::URGENCY_HINTS)
    //         },
    //         NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
    //             if self.mode.contains(TerminalMode::ALT_SCREEN) {
    //                 self.swap_alt();
    //             }
    //         },
    //         NamedPrivateMode::ShowCursor => {
    //             self.mode.remove(TerminalMode::SHOW_CURSOR)
    //         },
    //         NamedPrivateMode::CursorKeys => {
    //             self.mode.remove(TerminalMode::APP_CURSOR)
    //         },
    //         NamedPrivateMode::ReportMouseClicks => {
    //             self.mode.remove(TerminalMode::MOUSE_REPORT_CLICK);
    //             self.event_proxy.send_event(Event::MouseCursorDirty);
    //         },
    //         NamedPrivateMode::ReportCellMouseMotion => {
    //             self.mode.remove(TerminalMode::MOUSE_DRAG);
    //             self.event_proxy.send_event(Event::MouseCursorDirty);
    //         },
    //         NamedPrivateMode::ReportAllMouseMotion => {
    //             self.mode.remove(TerminalMode::MOUSE_MOTION);
    //             self.event_proxy.send_event(Event::MouseCursorDirty);
    //         },
    //         NamedPrivateMode::ReportFocusInOut => {
    //             self.mode.remove(TerminalMode::FOCUS_IN_OUT)
    //         },
    //         NamedPrivateMode::BracketedPaste => {
    //             self.mode.remove(TerminalMode::BRACKETED_PASTE)
    //         },
    //         NamedPrivateMode::SgrMouse => self.mode.remove(TerminalMode::SGR_MOUSE),
    //         NamedPrivateMode::Utf8Mouse => {
    //             self.mode.remove(TerminalMode::UTF8_MOUSE)
    //         },
    //         NamedPrivateMode::AlternateScroll => {
    //             self.mode.remove(TerminalMode::ALTERNATE_SCROLL)
    //         },
    //         NamedPrivateMode::LineWrap => self.mode.remove(TerminalMode::LINE_WRAP),
    //         NamedPrivateMode::Origin => self.mode.remove(TerminalMode::ORIGIN),
    //         NamedPrivateMode::ColumnMode => self.deccolm(),
    //         NamedPrivateMode::BlinkingCursor => {
    //             let style = self
    //                 .cursor_style
    //                 .get_or_insert(self.config.default_cursor_style);
    //             style.blinking = false;
    //             self.event_proxy.send_event(Event::CursorBlinkingChange);
    //         },
    //         NamedPrivateMode::SyncUpdate => (),
    //     }
    // }

    // #[inline]
    // fn report_private_mode(&mut self, mode: PrivateMode) {
    //     trace!("Reporting private mode {mode:?}");
    //     let state = match mode {
    //         PrivateMode::Named(mode) => match mode {
    //             NamedPrivateMode::CursorKeys => {
    //                 self.mode.contains(TerminalMode::APP_CURSOR).into()
    //             },
    //             NamedPrivateMode::Origin => {
    //                 self.mode.contains(TerminalMode::ORIGIN).into()
    //             },
    //             NamedPrivateMode::LineWrap => {
    //                 self.mode.contains(TerminalMode::LINE_WRAP).into()
    //             },
    //             NamedPrivateMode::BlinkingCursor => {
    //                 let style = self
    //                     .cursor_style
    //                     .get_or_insert(self.config.default_cursor_style);
    //                 style.blinking.into()
    //             },
    //             NamedPrivateMode::ShowCursor => {
    //                 self.mode.contains(TerminalMode::SHOW_CURSOR).into()
    //             },
    //             NamedPrivateMode::ReportMouseClicks => {
    //                 self.mode.contains(TerminalMode::MOUSE_REPORT_CLICK).into()
    //             },
    //             NamedPrivateMode::ReportCellMouseMotion => {
    //                 self.mode.contains(TerminalMode::MOUSE_DRAG).into()
    //             },
    //             NamedPrivateMode::ReportAllMouseMotion => {
    //                 self.mode.contains(TerminalMode::MOUSE_MOTION).into()
    //             },
    //             NamedPrivateMode::ReportFocusInOut => {
    //                 self.mode.contains(TerminalMode::FOCUS_IN_OUT).into()
    //             },
    //             NamedPrivateMode::Utf8Mouse => {
    //                 self.mode.contains(TerminalMode::UTF8_MOUSE).into()
    //             },
    //             NamedPrivateMode::SgrMouse => {
    //                 self.mode.contains(TerminalMode::SGR_MOUSE).into()
    //             },
    //             NamedPrivateMode::AlternateScroll => {
    //                 self.mode.contains(TerminalMode::ALTERNATE_SCROLL).into()
    //             },
    //             NamedPrivateMode::UrgencyHints => {
    //                 self.mode.contains(TerminalMode::URGENCY_HINTS).into()
    //             },
    //             NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
    //                 self.mode.contains(TerminalMode::ALT_SCREEN).into()
    //             },
    //             NamedPrivateMode::BracketedPaste => {
    //                 self.mode.contains(TerminalMode::BRACKETED_PASTE).into()
    //             },
    //             NamedPrivateMode::SyncUpdate => ModeState::Reset,
    //             NamedPrivateMode::ColumnMode => ModeState::NotSupported,
    //         },
    //         PrivateMode::Unknown(_) => ModeState::NotSupported,
    //     };

    //     self.event_proxy.send_event(Event::PtyWrite(format!(
    //         "\x1b[?{};{}$y",
    //         mode.raw(),
    //         state as u8,
    //     )));
    // }

    // TODO: delete
    // #[inline]
    // fn set_mode(&mut self, mode: Mode) {
    //     let mode = match mode {
    //         Mode::Named(mode) => mode,
    //         Mode::Unknown(mode) => {
    //             debug!("Ignoring unknown mode {mode} in set_mode");
    //             return;
    //         },
    //     };

    //     trace!("Setting public mode: {mode:?}");
    //     match mode {
    //         NamedMode::Insert => self.mode.insert(TerminalMode::INSERT),
    //         NamedMode::LineFeedNewLine => {
    //             self.mode.insert(TerminalMode::LINE_FEED_NEW_LINE)
    //         },
    //     }
    // }

    // #[inline]
    // fn unset_mode(&mut self, mode: Mode) {
    //     let mode = match mode {
    //         Mode::Named(mode) => mode,
    //         Mode::Unknown(mode) => {
    //             debug!("Ignoring unknown mode {mode} in unset_mode");
    //             return;
    //         },
    //     };

    //     trace!("Setting public mode: {mode:?}");
    //     match mode {
    //         NamedMode::Insert => {
    //             self.mode.remove(TerminalMode::INSERT);
    //             self.mark_fully_damaged();
    //         },
    //         NamedMode::LineFeedNewLine => {
    //             self.mode.remove(TerminalMode::LINE_FEED_NEW_LINE)
    //         },
    //     }
    // }

    // #[inline]
    // fn report_mode(&mut self, mode: Mode) {
    //     trace!("Reporting mode {mode:?}");
    //     let state = match mode {
    //         Mode::Named(mode) => match mode {
    //             NamedMode::Insert => {
    //                 self.mode.contains(TerminalMode::INSERT).into()
    //             },
    //             NamedMode::LineFeedNewLine => {
    //                 self.mode.contains(TerminalMode::LINE_FEED_NEW_LINE).into()
    //             },
    //         },
    //         Mode::Unknown(_) => ModeState::NotSupported,
    //     };

    //     self.event_proxy.send_event(Event::PtyWrite(format!(
    //         "\x1b[{};{}$y",
    //         mode.raw(),
    //         state as u8,
    //     )));
    // }

    #[inline]
    fn set_keypad_application_mode(&mut self) {
        trace!("Setting keypad application mode");
        self.mode.insert(TerminalMode::APP_KEYPAD);
    }

    #[inline]
    fn unset_keypad_application_mode(&mut self) {
        trace!("Unsetting keypad application mode");
        self.mode.remove(TerminalMode::APP_KEYPAD);
    }

    #[inline]
    fn set_active_charset_index(&mut self, index: CharsetIndex) {
        trace!("Setting active charset {index:?}");
        self.active_charset = index;
    }

    #[inline]
    fn set_cursor_style(&mut self, style: Option<CursorStyle>) {
        trace!("Setting cursor style {style:?}");
        self.cursor_style = style;

        // TODO: check
        // Notify UI about blinking changes.
        // self.event_proxy.send_event(Event::CursorBlinkingChange);
    }

    #[inline]
    fn set_cursor_shape(&mut self, shape: CursorShape) {
        trace!("Setting cursor shape {shape:?}");

        let style = self
            .cursor_style
            .get_or_insert(self.config.default_cursor_style);
        style.shape = shape;
    }

    // TODO: delete
    // #[inline]
    // fn set_window_title(&mut self, title: Option<String>) {
    //     trace!("Setting title to '{title:?}'");

    //     self.title.clone_from(&title);

    //     let event = match title {
    //         Some(value) => Event::Title(value),
    //         None => Event::ResetTitle,
    //     };

    //     self.event_proxy.send_event(event);
    // }

    // #[inline]
    // fn push_window_title(&mut self) {
    //     trace!("Pushing '{:?}' onto title stack", self.title);

    //     if self.title_stack.len() >= TITLE_STACK_MAX_DEPTH {
    //         let removed = self.title_stack.remove(0);
    //         trace!(
    //             "Removing '{removed:?}' from bottom of title stack that exceeds its maximum depth"
    //         );
    //     }

    //     self.title_stack.push(self.title.clone());
    // }

    // #[inline]
    // fn pop_window_title(&mut self) {
    //     trace!("Attempting to pop title from stack...");

    //     if let Some(popped) = self.title_stack.pop() {
    //         trace!("Title '{popped:?}' popped from stack");
    //         self.set_window_title(popped);
    //     }
    // }

    // TODO: move to libterm
    // #[inline]
    // fn request_text_area_by_pixels(&mut self) {
    //     self.event_proxy
    //         .send_event(Event::TextAreaSizeRequest(Arc::new(
    //             move |window_size| {
    //                 let height =
    //                     window_size.num_lines * window_size.cell_height;
    //                 let width = window_size.num_cols * window_size.cell_width;
    //                 format!("\x1b[4;{height};{width}t")
    //             },
    //         )));
    // }

    // #[inline]
    // fn request_text_area_by_chars(&mut self) {
    //     let text =
    //         format!("\x1b[8;{};{}t", self.screen_lines(), self.columns());
    //     self.event_proxy.send_event(Event::PtyWrite(text));
    // }

    // TODO: delete
    // /// Internal action handler (without sync buffering logic).
    // #[inline]
    // #[allow(clippy::enum_glob_use)]
    // fn handle_action_internal(&mut self, action: Action) {
    //     use Action::*;

    //     match action {
    //         Print(c) => self.print(c),
    //         ScreenAlignmentDisplay => self.screen_alignment_display(),
    //         Goto(line, col) => self.goto(line, col),
    //         GotoRow(line) => {
    //             trace!("Going to line: {line}");
    //             self.goto(line, self.grid.cursor.point.column.0);
    //         },
    //         GotoColumn(col) => {
    //             trace!("Going to column: {col}");
    //             self.goto(self.grid.cursor.point.line.0, col);
    //         },
    //         InsertBlank(count) => self.insert_blank(count),
    //         MoveUp {
    //             rows,
    //             carrage_return_needed,
    //         } => self.move_up(rows, carrage_return_needed),
    //         MoveDown {
    //             rows,
    //             carrage_return_needed,
    //         } => self.move_down(rows, carrage_return_needed),
    //         MoveForward(cols) => self.move_forward(cols),
    //         MoveBackward(cols) => self.move_backward(cols),
    //         InsertTabs(count) => self.insert_tabs(count),
    //         Backspace => self.backspace(),
    //         CarriageReturn => self.carriage_return(),
    //         LineFeed => self.linefeed(),
    //         Bell => self.bell(),
    //         NewLine => self.new_line(),
    //         NextLine => {
    //             self.linefeed();
    //             self.carriage_return();
    //         },
    //         Substitute => {},
    //         SetTabs(count) => self.insert_tabs(count),
    //         SetHorizontalTab => self.set_horizontal_tab(),
    //         ScrollUp(lines) => self.scroll_up(lines),
    //         ScrollDown(lines) => self.scroll_down(lines),
    //         InsertBlankLines(count) => self.insert_blank_lines(count),
    //         DeleteLines(count) => self.delete_lines(count),
    //         EraseChars(count) => self.erase_chars(count),
    //         DeleteChars(count) => self.delete_chars(count),
    //         MoveBackwardTabs(count) => self.move_backward_tabs(count),
    //         MoveForwardTabs(count) => self.move_forward_tabs(count),
    //         SaveCursorPosition => self.save_cursor_position(),
    //         RestoreCursorPosition => self.restore_cursor_position(),
    //         ClearLine(mode) => self.clear_line(mode),
    //         SetColor { index, color } => self.set_color(index, color),
    //         ResetColor(index) => self.reset_color(index),
    //         ClearScreen(mode) => self.clear_screen(mode),
    //         ClearTabs(mode) => self.clear_tabs(mode),
    //         ResetState => self.reset_state(),
    //         ReverseIndex => self.reverse_index(),
    //         SetHyperlink(hyperlink) => self.set_hyperlink(hyperlink),
    //         SGR(attr) => self.set_character_attribute(attr),
    //         SetScrollingRegion(top, bottom) => {
    //             self.set_scrolling_region(top, bottom)
    //         },
    //         ConfigureCharset(charset, index) => {
    //             self.configure_charset(charset, index)
    //         },
    //         SetActiveCharsetIndex(index) => {
    //             self.set_active_charset_index(index)
    //         },
    //         SetCursorStyle(style) => self.set_cursor_style(style),
    //         SetCursorShape(shape) => self.set_cursor_shape(shape),
    //         ReportDeviceStatus(arg) => self.report_device_status(arg),
    //         SetKeypadApplicationMode => self.set_keypad_application_mode(),
    //         UnsetKeypadApplicationMode => self.unset_keypad_application_mode(),
    //         PushWindowTitle => self.push_window_title(),
    //         PopWindowTitle => self.pop_window_title(),
    //         RequestTextAreaSizeByPixels => self.request_text_area_by_pixels(),
    //         RequestTextAreaSizeByChars => self.request_text_area_by_chars(),
    //         IdentifyTerminal(attr) => self.identify_terminal(attr),
    //         ReportKeyboardMode => self.report_keyboard_mode(),
    //         PushKeyboardMode(mode) => self.push_keyboard_mode(mode),
    //         PopKeyboardModes(count) => self.pop_keyboard_modes(count),
    //         SetKeyboardMode(mode, apply) => {
    //             if !self.config.kitty_keyboard {
    //                 return;
    //             }

    //             self.set_keyboard_mode(mode.into(), apply);
    //         },
    //         SetWindowTitle(title) => self.set_window_title(Some(title)),
    //         SetPrivateMode(mode) => self.set_private_mode(mode),
    //         UnsetPrivateMode(mode) => self.unset_private_mode(mode),
    //         ReportPrivateMode(mode) => self.report_private_mode(mode),
    //         SetMode(mode) => self.set_mode(mode),
    //         UnsetMode(mode) => self.unset_mode(mode),
    //         ReportMode(mode) => self.report_mode(mode),
    //         action => {
    //             trace!("[unimplemented] {:?}", action);
    //         },
    //     }
    // }
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
        if self.mode.contains(TerminalMode::INSERT)
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
                if self.mode.contains(TerminalMode::LINE_WRAP) {
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

    fn resize(&mut self, columns: usize, rows: usize) {
        // self.resize(Dimensions::);
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

        if self.mode.contains(TerminalMode::LINE_FEED_NEW_LINE) {
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
        if self.mode.contains(TerminalMode::ALT_SCREEN) {
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
        // self.selection = None;
        self.keyboard_mode_stack = Default::default();
        self.inactive_keyboard_mode_stack = Default::default();

        // Preserve vi mode across resets.
        // self.mode &= TerminalMode::VI;
        self.mode.insert(TerminalMode::default());

        // self.event_proxy.send_event(Event::CursorBlinkingChange);
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
                // self.selection = self
                //     .selection
                //     .take()
                //     .filter(|s| !s.intersects_range(range));
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
                // self.selection = self
                //     .selection
                //     .take()
                //     .filter(|s| !s.intersects_range(range));
            },
            ClearMode::All => {
                if self.mode.contains(TerminalMode::ALT_SCREEN) {
                    self.grid.reset_region(..);
                } else {
                    let old_offset = self.grid.display_offset();

                    self.grid.clear_viewport();

                    // Compute number of lines scrolled by clearing the viewport.
                    let lines =
                        self.grid.display_offset().saturating_sub(old_offset);
                }

                // self.selection = None;
            },
            ClearMode::Saved if self.history_size() > 0 => {
                self.grid.clear_history();
                // self.selection = self
                //     .selection
                //     .take()
                //     .filter(|s| !s.intersects_range(..Line(0)));
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
        // self.selection =
        //     self.selection.take().filter(|s| !s.intersects_range(range));
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
                col = index::Column(i);
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
                if self.tabs[index::Column(i)] {
                    col = index::Column(i);
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

        // Damage terminal if the color changed and it's not the cursor.
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

        // Damage terminal if the color changed and it's not the cursor.
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
                debug!("Term got unhandled attr: {attribute:?}");
            },
        }
    }

    fn set_cursor_shape(&mut self, shape: otty_escape::CursorShape) {
        // self.cursor_shape = Some(shape);
    }

    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        // self.cursor_icon = Some(icon);
    }

    fn set_cursor_style(&mut self, style: Option<otty_escape::CursorStyle>) {
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
        let (y_offset, max_y) = if self.mode.contains(TerminalMode::ORIGIN) {
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

    fn set_keypad_application_mode(&mut self, enabled: bool) {}

    fn push_keyboard_mode(&mut self) {}

    fn pop_keyboard_modes(&mut self, amount: u16) {}

    fn push_window_title(&mut self) {
        // if let Some(title) = &self.window_title {
        //     self.window_title_stack.push(title.clone());
        // }
    }

    fn pop_window_title(&mut self) {
        // self.window_title = self.window_title_stack.pop();
    }

    fn set_window_title(&mut self, title: String) {
        // self.window_title = Some(title);
    }

    fn scroll_display(&mut self, scroll: Scroll) {
        let old_display_offset = self.grid.display_offset();
        self.grid.scroll_display(scroll);
        // TODO: event emmiter
        // self.event_proxy.send_event(Event::MouseCursorDirty);

        // Clamp vi mode cursor to the viewport.
        // let viewport_start = -(self.grid.display_offset() as i32);
        // let viewport_end = viewport_start + self.bottommost_line().0;
        // let vi_cursor_line = &mut self.vi_mode_cursor.point.line.0;
        // *vi_cursor_line =
        //     cmp::min(viewport_end, cmp::max(viewport_start, *vi_cursor_line));
        // self.vi_mode_recompute_selection();

        // Damage everything if display offset changed.
        if old_display_offset != self.grid().display_offset() {
            self.mark_fully_damaged();
        }
    }

    fn swap_altscreen(&mut self, to_alter: bool) {
        if !to_alter {
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
        // self.mode ^= TerminalMode::ALT_SCREEN;
        // self.selection = None;
        self.mark_fully_damaged();
    }

    fn deccolm(&mut self) {
        // Setting 132 column font makes no sense, but run the other side effects.
        // Clear scrolling region.
        self.set_scrolling_region(1, self.screen_lines());

        // Clear grid.
        self.grid.reset_region(..);
        self.mark_fully_damaged();
    }

    fn set_mode(&mut self, mode: TerminalMode) {
        self.mode = mode;
    }

    fn begin_sync(&mut self) {}

    fn end_sync(&mut self) {}
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

/// Terminal version for escape sequence reports.
///
/// This returns the current terminal version as a unique number based on alacritty_terminal's
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardType {
    Clipboard,
    Selection,
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
            let is_tabstop = index % INITIAL_TABSTOPS == 0;
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

/// Terminal cursor rendering information.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct RenderableCursor {
    pub shape: CursorShape,
    pub point: Point,
}

impl RenderableCursor {
    fn new(term: &Surface) -> Self {
        // Cursor position.
        let mut point = term.grid.cursor.point;
        // let mut point = if vi_mode {
        //     term.vi_mode_cursor.point
        // } else {
        //     term.grid.cursor.point
        // };
        if term.grid[point].flags.contains(Flags::WIDE_CHAR_SPACER) {
            point.column -= 1;
        }

        // Cursor shape.
        let shape = if !term.mode().contains(TerminalMode::SHOW_CURSOR)
        {
            CursorShape::Hidden
        } else {
            term.cursor_style().shape
        };

        Self { shape, point }
    }
}

/// Visible terminal content.
///
/// This contains all content required to render the current terminal view.
pub struct SurfaceSnapshot<'a> {
    pub display_iter: GridIterator<'a, Cell>,
    // pub selection: Option<SelectionRange>,
    pub cursor: RenderableCursor,
    pub display_offset: usize,
    pub colors: &'a Colors,
    // pub mode: TerminalMode,
}

impl<'a> SurfaceSnapshot<'a> {
    fn new(term: &'a Surface) -> Self {
        Self {
            display_iter: term.grid().display_iter(),
            display_offset: term.grid().display_offset(),
            cursor: RenderableCursor::new(term),
            // selection: term.selection.as_ref().and_then(|s| s.to_range(term)),
            colors: &term.colors,
            // mode: *term.mode(),
        }
    }
}

pub trait SurfaceSnapshotSource {
    fn capture_snapshot(&self) -> SurfaceSnapshot;
}

impl SurfaceSnapshotSource for Surface {
    fn capture_snapshot(&self) -> SurfaceSnapshot {
        self.renderable_content()
    }
}

// /// Terminal test helpers.
// pub mod test {
//     use super::*;

//     #[cfg(feature = "serde")]
//     use serde::{Deserialize, Serialize};

//     use crate::event::VoidListener;

//     #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
//     pub struct TermSize {
//         pub columns: usize,
//         pub screen_lines: usize,
//     }

//     impl TermSize {
//         pub fn new(columns: usize, screen_lines: usize) -> Self {
//             Self {
//                 columns,
//                 screen_lines,
//             }
//         }
//     }

//     impl Dimensions for TermSize {
//         fn total_lines(&self) -> usize {
//             self.screen_lines()
//         }

//         fn screen_lines(&self) -> usize {
//             self.screen_lines
//         }

//         fn columns(&self) -> usize {
//             self.columns
//         }
//     }

//     /// Construct a terminal from its content as string.
//     ///
//     /// A `\n` will break line and `\r\n` will break line without wrapping.
//     ///
//     /// # Examples
//     ///
//     /// ```rust
//     /// use alacritty_terminal::term::test::mock_term;
//     ///
//     /// // Create a terminal with the following cells:
//     /// //
//     /// // [h][e][l][l][o] <- WRAPLINE flag set
//     /// // [:][)][ ][ ][ ]
//     /// // [t][e][s][t][ ]
//     /// mock_term(
//     ///     "\
//     ///     hello\n:)\r\ntest",
//     /// );
//     /// ```
//     pub fn mock_term(content: &str) -> Term<VoidListener> {
//         let lines: Vec<&str> = content.split('\n').collect();
//         let num_cols = lines
//             .iter()
//             .map(|line| {
//                 line.chars()
//                     .filter(|c| *c != '\r')
//                     .map(|c| c.width().unwrap())
//                     .sum()
//             })
//             .max()
//             .unwrap_or(0);

//         // Create terminal with the appropriate dimensions.
//         let size = TermSize::new(num_cols, lines.len());
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Fill terminal with content.
//         for (line, text) in lines.iter().enumerate() {
//             let line = Line(line as i32);
//             if !text.ends_with('\r') && line + 1 != lines.len() {
//                 term.grid[line][Column(num_cols - 1)]
//                     .flags
//                     .insert(Flags::WRAPLINE);
//             }

//             let mut index = 0;
//             for c in text.chars().take_while(|c| *c != '\r') {
//                 term.grid[line][Column(index)].c = c;

//                 // Handle fullwidth characters.
//                 let width = c.width().unwrap();
//                 if width == 2 {
//                     term.grid[line][Column(index)]
//                         .flags
//                         .insert(Flags::WIDE_CHAR);
//                     term.grid[line][Column(index + 1)]
//                         .flags
//                         .insert(Flags::WIDE_CHAR_SPACER);
//                 }

//                 index += width;
//             }
//         }

//         term
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     use std::mem;

//     use crate::event::VoidListener;
//     use crate::grid::{Grid, Scroll};
//     use crate::index::{Column, Point, Side};
//     use crate::selection::{Selection, SelectionType};
//     use crate::term::cell::{Cell, Flags};
//     use crate::term::test::TermSize;

//     #[test]
//     fn scroll_display_page_up() {
//         let size = TermSize::new(5, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 11 lines of scrollback.
//         for _ in 0..20 {
//             term.new_line();
//         }

//         // Scrollable amount to top is 11.
//         term.scroll_display(Scroll::PageUp);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(-1), Column(0)));
//         assert_eq!(term.grid.display_offset(), 10);

//         // Scrollable amount to top is 1.
//         term.scroll_display(Scroll::PageUp);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(-2), Column(0)));
//         assert_eq!(term.grid.display_offset(), 11);

//         // Scrollable amount to top is 0.
//         term.scroll_display(Scroll::PageUp);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(-2), Column(0)));
//         assert_eq!(term.grid.display_offset(), 11);
//     }

//     #[test]
//     fn scroll_display_page_down() {
//         let size = TermSize::new(5, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 11 lines of scrollback.
//         for _ in 0..20 {
//             term.new_line();
//         }

//         // Change display_offset to topmost.
//         term.grid_mut().scroll_display(Scroll::Top);
//         term.vi_mode_cursor =
//             ViModeCursor::new(Point::new(Line(-11), Column(0)));

//         // Scrollable amount to bottom is 11.
//         term.scroll_display(Scroll::PageDown);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(-1), Column(0)));
//         assert_eq!(term.grid.display_offset(), 1);

//         // Scrollable amount to bottom is 1.
//         term.scroll_display(Scroll::PageDown);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(0), Column(0)));
//         assert_eq!(term.grid.display_offset(), 0);

//         // Scrollable amount to bottom is 0.
//         term.scroll_display(Scroll::PageDown);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(0), Column(0)));
//         assert_eq!(term.grid.display_offset(), 0);
//     }

//     #[test]
//     fn simple_selection_works() {
//         let size = TermSize::new(5, 5);
//         let mut term = Term::new(Config::default(), &size, VoidListener);
//         let grid = term.grid_mut();
//         for i in 0..4 {
//             if i == 1 {
//                 continue;
//             }

//             grid[Line(i)][Column(0)].c = '"';

//             for j in 1..4 {
//                 grid[Line(i)][Column(j)].c = 'a';
//             }

//             grid[Line(i)][Column(4)].c = '"';
//         }
//         grid[Line(2)][Column(0)].c = ' ';
//         grid[Line(2)][Column(4)].c = ' ';
//         grid[Line(2)][Column(4)].flags.insert(Flags::WRAPLINE);
//         grid[Line(3)][Column(0)].c = ' ';

//         // Multiple lines contain an empty line.
//         term.selection = Some(Selection::new(
//             SelectionType::Simple,
//             Point {
//                 line: Line(0),
//                 column: Column(0),
//             },
//             Side::Left,
//         ));
//         if let Some(s) = term.selection.as_mut() {
//             s.update(
//                 Point {
//                     line: Line(2),
//                     column: Column(4),
//                 },
//                 Side::Right,
//             );
//         }
//         assert_eq!(
//             term.selection_to_string(),
//             Some(String::from("\"aaa\"\n\n aaa "))
//         );

//         // A wrapline.
//         term.selection = Some(Selection::new(
//             SelectionType::Simple,
//             Point {
//                 line: Line(2),
//                 column: Column(0),
//             },
//             Side::Left,
//         ));
//         if let Some(s) = term.selection.as_mut() {
//             s.update(
//                 Point {
//                     line: Line(3),
//                     column: Column(4),
//                 },
//                 Side::Right,
//             );
//         }
//         assert_eq!(
//             term.selection_to_string(),
//             Some(String::from(" aaa  aaa\""))
//         );
//     }

//     #[test]
//     fn semantic_selection_works() {
//         let size = TermSize::new(5, 3);
//         let mut term = Term::new(Config::default(), &size, VoidListener);
//         let mut grid: Grid<Cell> = Grid::new(3, 5, 0);
//         for i in 0..5 {
//             for j in 0..2 {
//                 grid[Line(j)][Column(i)].c = 'a';
//             }
//         }
//         grid[Line(0)][Column(0)].c = '"';
//         grid[Line(0)][Column(3)].c = '"';
//         grid[Line(1)][Column(2)].c = '"';
//         grid[Line(0)][Column(4)].flags.insert(Flags::WRAPLINE);

//         let mut escape_chars = String::from("\"");

//         mem::swap(&mut term.grid, &mut grid);
//         mem::swap(&mut term.config.semantic_escape_chars, &mut escape_chars);

//         {
//             term.selection = Some(Selection::new(
//                 SelectionType::Semantic,
//                 Point {
//                     line: Line(0),
//                     column: Column(1),
//                 },
//                 Side::Left,
//             ));
//             assert_eq!(term.selection_to_string(), Some(String::from("aa")));
//         }

//         {
//             term.selection = Some(Selection::new(
//                 SelectionType::Semantic,
//                 Point {
//                     line: Line(0),
//                     column: Column(4),
//                 },
//                 Side::Left,
//             ));
//             assert_eq!(term.selection_to_string(), Some(String::from("aaa")));
//         }

//         {
//             term.selection = Some(Selection::new(
//                 SelectionType::Semantic,
//                 Point {
//                     line: Line(1),
//                     column: Column(1),
//                 },
//                 Side::Left,
//             ));
//             assert_eq!(term.selection_to_string(), Some(String::from("aaa")));
//         }
//     }

//     #[test]
//     fn line_selection_works() {
//         let size = TermSize::new(5, 1);
//         let mut term = Term::new(Config::default(), &size, VoidListener);
//         let mut grid: Grid<Cell> = Grid::new(1, 5, 0);
//         for i in 0..5 {
//             grid[Line(0)][Column(i)].c = 'a';
//         }
//         grid[Line(0)][Column(0)].c = '"';
//         grid[Line(0)][Column(3)].c = '"';

//         mem::swap(&mut term.grid, &mut grid);

//         term.selection = Some(Selection::new(
//             SelectionType::Lines,
//             Point {
//                 line: Line(0),
//                 column: Column(3),
//             },
//             Side::Left,
//         ));
//         assert_eq!(term.selection_to_string(), Some(String::from("\"aa\"a\n")));
//     }

//     #[test]
//     fn block_selection_works() {
//         let size = TermSize::new(5, 5);
//         let mut term = Term::new(Config::default(), &size, VoidListener);
//         let grid = term.grid_mut();
//         for i in 1..4 {
//             grid[Line(i)][Column(0)].c = '"';

//             for j in 1..4 {
//                 grid[Line(i)][Column(j)].c = 'a';
//             }

//             grid[Line(i)][Column(4)].c = '"';
//         }
//         grid[Line(2)][Column(2)].c = ' ';
//         grid[Line(2)][Column(4)].flags.insert(Flags::WRAPLINE);
//         grid[Line(3)][Column(4)].c = ' ';

//         term.selection = Some(Selection::new(
//             SelectionType::Block,
//             Point {
//                 line: Line(0),
//                 column: Column(3),
//             },
//             Side::Left,
//         ));

//         // The same column.
//         if let Some(s) = term.selection.as_mut() {
//             s.update(
//                 Point {
//                     line: Line(3),
//                     column: Column(3),
//                 },
//                 Side::Right,
//             );
//         }
//         assert_eq!(term.selection_to_string(), Some(String::from("\na\na\na")));

//         // The first column.
//         if let Some(s) = term.selection.as_mut() {
//             s.update(
//                 Point {
//                     line: Line(3),
//                     column: Column(0),
//                 },
//                 Side::Left,
//             );
//         }
//         assert_eq!(
//             term.selection_to_string(),
//             Some(String::from("\n\"aa\n\"a\n\"aa"))
//         );

//         // The last column.
//         if let Some(s) = term.selection.as_mut() {
//             s.update(
//                 Point {
//                     line: Line(3),
//                     column: Column(4),
//                 },
//                 Side::Right,
//             );
//         }
//         assert_eq!(
//             term.selection_to_string(),
//             Some(String::from("\na\"\na\"\na"))
//         );
//     }

//     // /// Check that the grid can be serialized back and forth losslessly.
//     // ///
//     // /// This test is in the term module as opposed to the grid since we want to
//     // /// test this property with a T=Cell.
//     // #[test]
//     // #[cfg(feature = "serde")]
//     // fn grid_serde() {
//     //     let grid: Grid<Cell> = Grid::new(24, 80, 0);
//     //     let serialized = serde_json::to_string(&grid).expect("ser");
//     //     let deserialized =
//     //         serde_json::from_str::<Grid<Cell>>(&serialized).expect("de");

//     //     assert_eq!(deserialized, grid);
//     // }

//     // TODO:
//     // #[test]
//     // fn input_line_drawing_character() {
//     //     let size = TermSize::new(7, 17);
//     //     let mut term = Term::new(Config::default(), &size, VoidListener);
//     //     let cursor = Point::new(Line(0), Column(0));
//     //     term.configure_charset(
//     //         CharsetIndex::G0,
//     //         StandardCharset::SpecialCharacterAndLineDrawing,
//     //     );
//     //     term.input('a');

//     //     assert_eq!(term.grid()[cursor].c, '▒');
//     // }

//     #[test]
//     fn clearing_viewport_keeps_history_position() {
//         let size = TermSize::new(10, 20);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..29 {
//             term.new_line();
//         }

//         // Change the display area.
//         term.scroll_display(Scroll::Top);

//         assert_eq!(term.grid.display_offset(), 10);

//         // Clear the viewport.
//         term.clear_screen(ClearMode::All);

//         assert_eq!(term.grid.display_offset(), 10);
//     }

//     #[test]
//     fn clearing_viewport_with_vi_mode_keeps_history_position() {
//         let size = TermSize::new(10, 20);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..29 {
//             term.new_line();
//         }

//         // Enable vi mode.
//         term.toggle_vi_mode();

//         // Change the display area and the vi cursor position.
//         term.scroll_display(Scroll::Top);
//         term.vi_mode_cursor.point = Point::new(Line(-5), Column(3));

//         assert_eq!(term.grid.display_offset(), 10);

//         // Clear the viewport.
//         term.clear_screen(ClearMode::All);

//         assert_eq!(term.grid.display_offset(), 10);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(-5), Column(3)));
//     }

//     #[test]
//     fn clearing_scrollback_resets_display_offset() {
//         let size = TermSize::new(10, 20);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..29 {
//             term.new_line();
//         }

//         // Change the display area.
//         term.scroll_display(Scroll::Top);

//         assert_eq!(term.grid.display_offset(), 10);

//         // Clear the scrollback buffer.
//         term.clear_screen(ClearMode::Saved);

//         assert_eq!(term.grid.display_offset(), 0);
//     }

//     #[test]
//     fn clearing_scrollback_sets_vi_cursor_into_viewport() {
//         let size = TermSize::new(10, 20);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..29 {
//             term.new_line();
//         }

//         // Enable vi mode.
//         term.toggle_vi_mode();

//         // Change the display area and the vi cursor position.
//         term.scroll_display(Scroll::Top);
//         term.vi_mode_cursor.point = Point::new(Line(-5), Column(3));

//         assert_eq!(term.grid.display_offset(), 10);

//         // Clear the scrollback buffer.
//         term.clear_screen(ClearMode::Saved);

//         assert_eq!(term.grid.display_offset(), 0);
//         assert_eq!(term.vi_mode_cursor.point, Point::new(Line(0), Column(3)));
//     }

//     #[test]
//     fn clear_saved_lines() {
//         let size = TermSize::new(7, 17);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Add one line of scrollback.
//         term.grid.scroll_up(&(Line(0)..Line(1)), 1);

//         // Clear the history.
//         term.clear_screen(ClearMode::Saved);

//         // Make sure that scrolling does not change the grid.
//         let mut scrolled_grid = term.grid.clone();
//         scrolled_grid.scroll_display(Scroll::Top);

//         // Truncate grids for comparison.
//         scrolled_grid.truncate();
//         term.grid.truncate();

//         assert_eq!(term.grid, scrolled_grid);
//     }

//     #[test]
//     fn vi_cursor_keep_pos_on_scrollback_buffer() {
//         let size = TermSize::new(5, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 11 lines of scrollback.
//         for _ in 0..20 {
//             term.new_line();
//         }

//         // Enable vi mode.
//         term.toggle_vi_mode();

//         term.scroll_display(Scroll::Top);
//         term.vi_mode_cursor.point.line = Line(-11);

//         term.linefeed();
//         assert_eq!(term.vi_mode_cursor.point.line, Line(-12));
//     }

//     #[test]
//     fn grow_lines_updates_active_cursor_pos() {
//         let mut size = TermSize::new(100, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..19 {
//             term.new_line();
//         }
//         assert_eq!(term.history_size(), 10);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(9), Column(0)));

//         // Increase visible lines.
//         size.screen_lines = 30;
//         term.resize(size);

//         assert_eq!(term.history_size(), 0);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(19), Column(0)));
//     }

//     #[test]
//     fn grow_lines_updates_inactive_cursor_pos() {
//         let mut size = TermSize::new(100, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..19 {
//             term.new_line();
//         }
//         assert_eq!(term.history_size(), 10);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(9), Column(0)));

//         // Enter alt screen.
//         term.set_private_mode(
//             NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
//         );

//         // Increase visible lines.
//         size.screen_lines = 30;
//         term.resize(size);

//         // Leave alt screen.
//         term.unset_private_mode(
//             NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
//         );

//         assert_eq!(term.history_size(), 0);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(19), Column(0)));
//     }

//     #[test]
//     fn shrink_lines_updates_active_cursor_pos() {
//         let mut size = TermSize::new(100, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..19 {
//             term.new_line();
//         }
//         assert_eq!(term.history_size(), 10);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(9), Column(0)));

//         // Increase visible lines.
//         size.screen_lines = 5;
//         term.resize(size);

//         assert_eq!(term.history_size(), 15);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(4), Column(0)));
//     }

//     #[test]
//     fn shrink_lines_updates_inactive_cursor_pos() {
//         let mut size = TermSize::new(100, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Create 10 lines of scrollback.
//         for _ in 0..19 {
//             term.new_line();
//         }
//         assert_eq!(term.history_size(), 10);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(9), Column(0)));

//         // Enter alt screen.
//         term.set_private_mode(
//             NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
//         );

//         // Increase visible lines.
//         size.screen_lines = 5;
//         term.resize(size);

//         // Leave alt screen.
//         term.unset_private_mode(
//             NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
//         );

//         assert_eq!(term.history_size(), 15);
//         assert_eq!(term.grid.cursor.point, Point::new(Line(4), Column(0)));
//     }

//     #[test]
//     fn damage_public_usage() {
//         let size = TermSize::new(10, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);
//         // Reset terminal for partial damage tests since it's initialized as fully damaged.
//         term.reset_damage();

//         // Test that we damage input form [`Term::input`].

//         let left = term.grid.cursor.point.column.0;
//         term.print('d');
//         term.print('a');
//         term.print('m');
//         term.print('a');
//         term.print('g');
//         term.print('e');
//         let right = term.grid.cursor.point.column.0;

//         let mut damaged_lines = match term.damage() {
//             TermDamage::Full => {
//                 panic!("Expected partial damage, however got Full")
//             },
//             TermDamage::Partial(damaged_lines) => damaged_lines,
//         };
//         assert_eq!(
//             damaged_lines.next(),
//             Some(LineDamageBounds {
//                 line: 0,
//                 left,
//                 right
//             })
//         );
//         assert_eq!(damaged_lines.next(), None);
//         term.reset_damage();

//         // Create scrollback.
//         for _ in 0..20 {
//             term.new_line();
//         }

//         match term.damage() {
//             TermDamage::Full => (),
//             TermDamage::Partial(_) => {
//                 panic!("Expected Full damage, however got Partial ")
//             },
//         };
//         term.reset_damage();

//         term.scroll_display(Scroll::Delta(10));
//         term.reset_damage();

//         // No damage when scrolled into viewport.
//         for idx in 0..term.columns() {
//             term.goto(idx as i32, idx);
//         }
//         let mut damaged_lines = match term.damage() {
//             TermDamage::Full => {
//                 panic!("Expected partial damage, however got Full")
//             },
//             TermDamage::Partial(damaged_lines) => damaged_lines,
//         };
//         assert_eq!(damaged_lines.next(), None);

//         // Scroll back into the viewport, so we have 2 visible lines which terminal can write
//         // to.
//         term.scroll_display(Scroll::Delta(-2));
//         term.reset_damage();

//         term.goto(0, 0);
//         term.goto(1, 0);
//         term.goto(2, 0);
//         let display_offset = term.grid().display_offset();
//         let mut damaged_lines = match term.damage() {
//             TermDamage::Full => {
//                 panic!("Expected partial damage, however got Full")
//             },
//             TermDamage::Partial(damaged_lines) => damaged_lines,
//         };
//         assert_eq!(
//             damaged_lines.next(),
//             Some(LineDamageBounds {
//                 line: display_offset,
//                 left: 0,
//                 right: 0
//             })
//         );
//         assert_eq!(
//             damaged_lines.next(),
//             Some(LineDamageBounds {
//                 line: display_offset + 1,
//                 left: 0,
//                 right: 0
//             })
//         );
//         assert_eq!(damaged_lines.next(), None);
//     }

//     #[test]
//     fn damage_cursor_movements() {
//         let size = TermSize::new(10, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);
//         let num_cols = term.columns();
//         // Reset terminal for partial damage tests since it's initialized as fully damaged.
//         term.reset_damage();

//         term.goto(1, 1);

//         // NOTE While we can use `[Term::damage]` to access terminal damage information, in the
//         // following tests we will be accessing `term.damage.lines` directly to avoid adding extra
//         // damage information (like cursor and Vi cursor), which we're not testing.

//         assert_eq!(
//             term.damage.lines[0],
//             LineDamageBounds {
//                 line: 0,
//                 left: 0,
//                 right: 0
//             }
//         );
//         assert_eq!(
//             term.damage.lines[1],
//             LineDamageBounds {
//                 line: 1,
//                 left: 1,
//                 right: 1
//             }
//         );
//         term.damage.reset(num_cols);

//         term.move_forward(3);
//         assert_eq!(
//             term.damage.lines[1],
//             LineDamageBounds {
//                 line: 1,
//                 left: 1,
//                 right: 4
//             }
//         );
//         term.damage.reset(num_cols);

//         term.move_backward(8);
//         assert_eq!(
//             term.damage.lines[1],
//             LineDamageBounds {
//                 line: 1,
//                 left: 0,
//                 right: 4
//             }
//         );
//         term.goto(5, 5);
//         term.damage.reset(num_cols);

//         term.backspace();
//         term.backspace();
//         assert_eq!(
//             term.damage.lines[5],
//             LineDamageBounds {
//                 line: 5,
//                 left: 3,
//                 right: 5
//             }
//         );
//         term.damage.reset(num_cols);

//         term.move_up(1, false);
//         assert_eq!(
//             term.damage.lines[5],
//             LineDamageBounds {
//                 line: 5,
//                 left: 3,
//                 right: 3
//             }
//         );
//         assert_eq!(
//             term.damage.lines[4],
//             LineDamageBounds {
//                 line: 4,
//                 left: 3,
//                 right: 3
//             }
//         );
//         term.damage.reset(num_cols);

//         term.move_up(1, false);
//         term.move_up(1, false);
//         assert_eq!(
//             term.damage.lines[4],
//             LineDamageBounds {
//                 line: 4,
//                 left: 3,
//                 right: 3
//             }
//         );
//         assert_eq!(
//             term.damage.lines[5],
//             LineDamageBounds {
//                 line: 5,
//                 left: 3,
//                 right: 3
//             }
//         );
//         assert_eq!(
//             term.damage.lines[6],
//             LineDamageBounds {
//                 line: 6,
//                 left: 3,
//                 right: 3
//             }
//         );
//         term.damage.reset(num_cols);

//         term.wrapline();
//         assert_eq!(
//             term.damage.lines[6],
//             LineDamageBounds {
//                 line: 6,
//                 left: 3,
//                 right: 3
//             }
//         );
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 0,
//                 right: 0
//             }
//         );
//         term.move_forward(3);
//         term.move_up(1, false);
//         term.damage.reset(num_cols);

//         term.linefeed();
//         assert_eq!(
//             term.damage.lines[6],
//             LineDamageBounds {
//                 line: 6,
//                 left: 3,
//                 right: 3
//             }
//         );
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 3,
//                 right: 3
//             }
//         );
//         term.damage.reset(num_cols);

//         term.carriage_return();
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 0,
//                 right: 3
//             }
//         );
//         term.damage.reset(num_cols);

//         term.erase_chars(5);
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 0,
//                 right: 5
//             }
//         );
//         term.damage.reset(num_cols);

//         term.delete_chars(3);
//         let right = term.columns() - 1;
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 0,
//                 right
//             }
//         );
//         term.move_forward(term.columns());
//         term.damage.reset(num_cols);

//         term.move_backward_tabs(1);
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 8,
//                 right
//             }
//         );
//         term.save_cursor_position();
//         term.goto(1, 1);
//         term.damage.reset(num_cols);

//         term.restore_cursor_position();
//         assert_eq!(
//             term.damage.lines[1],
//             LineDamageBounds {
//                 line: 1,
//                 left: 1,
//                 right: 1
//             }
//         );
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 8,
//                 right: 8
//             }
//         );
//         term.damage.reset(num_cols);

//         term.clear_line(LineClearMode::All);
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 0,
//                 right
//             }
//         );
//         term.damage.reset(num_cols);

//         term.clear_line(LineClearMode::Left);
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 0,
//                 right: 8
//             }
//         );
//         term.damage.reset(num_cols);

//         term.clear_line(LineClearMode::Right);
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 8,
//                 right
//             }
//         );
//         term.damage.reset(num_cols);

//         term.reverse_index();
//         assert_eq!(
//             term.damage.lines[7],
//             LineDamageBounds {
//                 line: 7,
//                 left: 8,
//                 right: 8
//             }
//         );
//         assert_eq!(
//             term.damage.lines[6],
//             LineDamageBounds {
//                 line: 6,
//                 left: 8,
//                 right: 8
//             }
//         );
//     }

//     #[test]
//     fn full_damage() {
//         let size = TermSize::new(100, 10);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         assert!(term.damage.full);
//         for _ in 0..20 {
//             term.new_line();
//         }
//         term.reset_damage();

//         term.clear_screen(ClearMode::Above);
//         assert!(term.damage.full);
//         term.reset_damage();

//         term.scroll_display(Scroll::Top);
//         assert!(term.damage.full);
//         term.reset_damage();

//         // Sequential call to scroll display without doing anything shouldn't damage.
//         term.scroll_display(Scroll::Top);
//         assert!(!term.damage.full);
//         term.reset_damage();

//         term.set_options(Config::default());
//         assert!(term.damage.full);
//         term.reset_damage();

//         term.scroll_down_relative(Line(5), 2);
//         assert!(term.damage.full);
//         term.reset_damage();

//         term.scroll_up_relative(Line(3), 2);
//         assert!(term.damage.full);
//         term.reset_damage();

//         term.deccolm();
//         assert!(term.damage.full);
//         term.reset_damage();

//         term.screen_alignment_display();
//         assert!(term.damage.full);
//         term.reset_damage();

//         term.set_mode(NamedMode::Insert.into());
//         // Just setting `Insert` mode shouldn't mark terminal as damaged.
//         assert!(!term.damage.full);
//         term.reset_damage();

//         let color_index = 257;
//         term.set_color(color_index, Rgb::default());
//         assert!(term.damage.full);
//         term.reset_damage();

//         // Setting the same color once again shouldn't trigger full damage.
//         term.set_color(color_index, Rgb::default());
//         assert!(!term.damage.full);

//         term.reset_color(color_index);
//         assert!(term.damage.full);
//         term.reset_damage();

//         // We shouldn't trigger fully damage when cursor gets update.
//         term.set_color(StdColor::Cursor as usize, Rgb::default());
//         assert!(!term.damage.full);

//         // However requesting terminal damage should mark terminal as fully damaged in `Insert`
//         // mode.
//         let _ = term.damage();
//         assert!(term.damage.full);
//         term.reset_damage();

//         term.unset_mode(NamedMode::Insert.into());
//         assert!(term.damage.full);
//         term.reset_damage();

//         // Keep this as a last check, so we don't have to deal with restoring from alt-screen.
//         term.swap_alt();
//         assert!(term.damage.full);
//         term.reset_damage();

//         let size = TermSize::new(10, 10);
//         term.resize(size);
//         assert!(term.damage.full);
//     }

//     #[test]
//     fn window_title() {
//         let size = TermSize::new(7, 17);
//         let mut term = Term::new(Config::default(), &size, VoidListener);

//         // Title None by default.
//         assert_eq!(term.title, None);

//         // Title can be set.
//         term.set_window_title(Some("Test".into()));
//         assert_eq!(term.title, Some("Test".into()));

//         // Title can be pushed onto stack.
//         term.push_window_title();
//         term.set_window_title(Some("Next".into()));
//         assert_eq!(term.title, Some("Next".into()));
//         assert_eq!(term.title_stack.first().unwrap(), &Some("Test".into()));

//         // Title can be popped from stack and set as the window title.
//         term.pop_window_title();
//         assert_eq!(term.title, Some("Test".into()));
//         assert!(term.title_stack.is_empty());

//         // Title stack doesn't grow infinitely.
//         for _ in 0..4097 {
//             term.push_window_title();
//         }
//         assert_eq!(term.title_stack.len(), 4096);

//         // Title and title stack reset when terminal state is reset.
//         term.push_window_title();
//         term.reset_state();
//         assert_eq!(term.title, None);
//         assert!(term.title_stack.is_empty());

//         // Title stack pops back to default.
//         term.title = None;
//         term.push_window_title();
//         term.set_window_title(Some("Test".into()));
//         term.pop_window_title();
//         assert_eq!(term.title, None);

//         // Title can be reset to default.
//         term.title = Some("Test".into());
//         term.set_window_title(None);
//         assert_eq!(term.title, None);
//     }

//     #[test]
//     fn parse_cargo_version() {
//         assert!(version_number(env!("CARGO_PKG_VERSION")) >= 10_01);
//         assert_eq!(version_number("0.0.1-dev"), 1);
//         assert_eq!(version_number("0.1.2-dev"), 1_02);
//         assert_eq!(version_number("1.2.3-dev"), 1_02_03);
//         assert_eq!(version_number("999.99.99"), 9_99_99_99);
//     }
// }
