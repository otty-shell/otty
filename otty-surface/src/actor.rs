//! Abstraction over operations that can be applied to a terminal surface.
//!
//! The [`SurfaceActor`] trait specifies the high‑level actions needed to
//! manipulate terminal content: printing characters, scrolling, clearing,
//! changing modes, updating colors, reporting state, and so on.

use std::collections::VecDeque;

use crate::escape::{
    CharacterAttribute, Charset, CharsetIndex, ClearMode, Hyperlink,
    KeyboardMode, LineClearMode, Mode, PrivateMode, Rgb, TabClearMode,
};
use crate::escape::{CursorShape, CursorStyle, KeyboardModeApplyBehavior};
use crate::grid::{Dimensions, Scroll};
use crate::index::Point;
use crate::{SelectionType, Side};

/// Consumer of semantic terminal actions.
///
/// Each method corresponds to a high‑level terminal operation (for example:
/// print a character, scroll a region, clear part of the screen, toggle a
/// mode, report status).
///
/// The default implementations are no‑ops so that embedders can override only
/// the subset they care about. The main in‑tree implementation
/// ([`crate::surface::Surface`]) overrides all relevant methods to maintain
/// the terminal content model.
pub trait SurfaceActor {
    /// Render a single Unicode scalar value at the current cursor position.
    fn print(&mut self, _: char) {}

    /// Resize the underlying surface to the given dimensions.
    ///
    /// Implementations typically reflow scrollback and viewport content to
    /// mirror xterm/Alacritty behavior.
    fn resize<S: Dimensions>(&mut self, _: S) {}

    /// Insert blank cells at the cursor column, shifting existing content
    /// to the right.
    fn insert_blank(&mut self, _count: usize) {}

    /// Insert blank lines at the cursor line within the scroll region.
    fn insert_blank_lines(&mut self, _count: usize) {}

    /// Delete lines starting at the cursor line within the scroll region.
    fn delete_lines(&mut self, _count: usize) {}

    /// Delete character cells starting at the cursor column.
    fn delete_chars(&mut self, _count: usize) {}

    /// Erase character cells starting at the cursor column, replacing them
    /// with the current background color.
    fn erase_chars(&mut self, _count: usize) {}

    /// Move the cursor one cell to the left, if possible.
    fn backspace(&mut self) {}

    /// Move the cursor to the first column of the current line.
    fn carriage_return(&mut self) {}

    /// Move the cursor down one line, scrolling if necessary.
    fn line_feed(&mut self) {}

    /// Combined line feed and optional carriage return depending on mode.
    fn new_line(&mut self) {}

    /// Set a horizontal tabstop at the current cursor column.
    fn set_horizontal_tab(&mut self) {}

    /// Reverse index (RI): scroll the content down when the cursor is on
    /// the top margin, otherwise move the cursor one line up.
    fn reverse_index(&mut self) {}

    /// Reset terminal state to its power‑on defaults.
    fn reset(&mut self) {}

    /// Clear the screen according to the provided clear mode.
    fn clear_screen(&mut self, _: ClearMode) {}

    /// Clear the current line according to the provided clear mode.
    fn clear_line(&mut self, _: LineClearMode) {}

    /// Insert horizontal tabs, shifting text to the right.
    fn insert_tabs(&mut self, _count: usize) {}

    /// Clear tabstops according to the provided clear mode.
    fn clear_tabs(&mut self, _: TabClearMode) {}

    /// Trigger the "screen alignment display" (DECALN) test pattern.
    fn screen_alignment_display(&mut self) {}

    /// Move the cursor forward by a number of tab positions.
    fn move_forward_tabs(&mut self, _count: usize) {}

    /// Move the cursor backward by a number of tab positions.
    fn move_backward_tabs(&mut self, _count: usize) {}

    /// Select the active character set index (G0–G3).
    fn set_active_charset_index(&mut self, _: CharsetIndex) {}

    /// Configure the character set mapped at a given index.
    fn configure_charset(&mut self, _: Charset, _: CharsetIndex) {}

    /// Set a color register to a specific RGB value.
    fn set_color(&mut self, _index: usize, _: Rgb) {}

    /// Query the RGB value stored in a color register.
    fn query_color(&mut self, _index: usize) {}

    /// Reset a color register back to its default value.
    fn reset_color(&mut self, _index: usize) {}

    /// Restrict scrolling to the given region.
    fn set_scrolling_region(&mut self, _top: usize, _bottom: usize) {}

    /// Scroll the content up within the current scroll region.
    fn scroll_up(&mut self, _count: usize) {}

    /// Scroll the content down within the current scroll region.
    fn scroll_down(&mut self, _count: usize) {}

    /// Scroll the display viewport relative to the scrollback history.
    fn scroll_display(&mut self, _: Scroll) {}

    /// Set the hyperlink associated with subsequent printed cells.
    fn set_hyperlink(&mut self, _: Option<Hyperlink>) {}

    /// Apply a single Select Graphic Rendition (SGR) attribute.
    fn sgr(&mut self, _: CharacterAttribute) {}

    /// Configure the cursor shape (block, underline, bar, hidden).
    fn set_cursor_shape(&mut self, _: CursorShape) {}

    /// Configure the cursor style (shape + blink).
    fn set_cursor_style(&mut self, _: Option<CursorStyle>) {}

    /// Save the current cursor position and attributes.
    fn save_cursor(&mut self) {}

    /// Restore the cursor position and attributes from the last saved state.
    fn restore_cursor(&mut self) {}

    /// Move the cursor up by `rows` lines, optionally performing carriage
    /// return.
    fn move_up(&mut self, _rows: usize, _carriage_return: bool) {}

    /// Move the cursor down by `rows` lines, optionally performing carriage
    /// return.
    fn move_down(&mut self, _rows: usize, _carriage_return: bool) {}

    /// Move the cursor forward (right) by `cols` columns.
    fn move_forward(&mut self, _cols: usize) {}

    /// Move the cursor backward (left) by `cols` columns.
    fn move_backward(&mut self, _cols: usize) {}

    /// Move the cursor to the given row/column in the current origin mode.
    fn goto(&mut self, _row: i32, _col: usize) {}

    /// Move the cursor vertically to an absolute row.
    fn goto_row(&mut self, _row: i32) {}

    /// Move the cursor horizontally to an absolute column.
    fn goto_column(&mut self, _col: usize) {}

    /// Enable or disable the keypad application mode.
    fn set_keypad_application_mode(&mut self, _enabled: bool) {}

    /// Update the terminal keyboard mode using the given apply behavior.
    fn set_keyboard_mode(
        &mut self,
        _: KeyboardMode,
        _: KeyboardModeApplyBehavior,
    ) {
    }

    /// Push a keyboard mode onto the mode stack.
    fn push_keyboard_mode(&mut self, _: KeyboardMode) {}

    /// Pop a number of keyboard modes from the mode stack.
    fn pop_keyboard_modes(&mut self, _count: u16) {}

    /// Report the current keyboard mode via the given report channel.
    fn report_keyboard_mode(&mut self, _report_channel: &mut VecDeque<u8>) {}

    /// Save the current window title on an internal stack.
    fn push_window_title(&mut self) {}

    /// Restore the last window title from the internal stack.
    fn pop_window_title(&mut self) -> Option<String> {
        None
    }

    /// Set the active window title.
    fn set_window_title(&mut self, _title: Option<String>) {}

    /// Handle DEC column mode (DECCOLM) changes.
    fn deccolm(&mut self) {}

    /// Enable a DEC private mode flag.
    fn set_private_mode(&mut self, _: PrivateMode) {}

    /// Disable a DEC private mode flag.
    fn unset_private_mode(&mut self, _: PrivateMode) {}

    /// Report the state of a DEC private mode flag.
    fn report_private_mode(
        &mut self,
        _: PrivateMode,
        _report_channel: &mut VecDeque<u8>,
    ) {
    }

    /// Enable a public (non‑private) terminal mode.
    fn set_mode(&mut self, _: Mode) {}

    /// Disable a public (non‑private) terminal mode.
    fn unset_mode(&mut self, _: Mode) {}

    /// Report the state of a public terminal mode.
    fn report_mode(&mut self, _: Mode, _report_channel: &mut VecDeque<u8>) {}

    /// Report terminal identity and capabilities (DA/DECID).
    fn identify_terminal(
        &mut self,
        _attr: Option<char>,
        _report_channel: &mut VecDeque<u8>,
    ) {
    }

    /// Report device status codes through the provided report channel.
    fn report_device_status(
        &mut self,
        _status: usize,
        _report_channel: &mut VecDeque<u8>,
    ) {
    }

    /// Report or request the text area size in pixels.
    fn request_text_area_by_pixels(
        &mut self,
        _report_channel: &mut VecDeque<u8>,
    ) {
    }

    /// Report or request the text area size in character cells.
    fn request_text_area_by_chars(
        &mut self,
        _report_channel: &mut VecDeque<u8>,
    ) {
    }

    /// Init the selection range
    fn start_selection(&mut self, _: SelectionType, _: Point, _: Side) {}

    /// Update the selection range
    fn update_selection(&mut self, _: Point, _: Side) {}

    /// Handle high‑level block lifecycle events coming from the parser.
    fn handle_block_event(&mut self, _: crate::escape::BlockEvent) {}
}
