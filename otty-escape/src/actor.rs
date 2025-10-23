//! High-level escape sequence consumer interface.
//!
//! The [`Parser`](crate::Parser) translates the raw byte stream into semantic
//! events and relays them to an [`Actor`] implementation.  Downstream crates
//! can implement this trait to mutate their terminal model, update UI state or
//! collect metrics without re-implementing the escape sequence finite state
//! machine.

use cursor_icon::CursorIcon;

use crate::{
    attributes::Attr,
    charset::{Charset, CharsetIndex},
    color::Rgb,
    cursor::{CursorShape, CursorStyle},
    hyperlink::Hyperlink,
    mode::{
        ClearMode, KeyboardModes, KeyboardModesApplyBehavior, LineClearMode,
        Mode, ModifyOtherKeys, PrivateMode, ScpCharPath, ScpUpdateMode,
        TabClearMode,
    },
};

/// Trait implemented by consumers of the escape sequence parser.
///
/// All methods have a default empty implementation so that downstream crates
/// only need to override the variants they actually care about.  The parser
/// will invoke these callbacks synchronously while it walks through the input
/// byte stream.
pub trait Actor {
    fn set_title(&mut self, _: Option<String>) {}

    fn print(&mut self, _: char) {}

    fn put_tab(&mut self, _: u16) {}

    fn backspace(&mut self) {}

    fn bell(&mut self) {}

    fn substitute(&mut self) {}

    fn set_active_charset(&mut self, _: CharsetIndex) {}

    fn linefeed(&mut self) {}

    fn carriage_return(&mut self) {}

    fn set_horizontal_tab(&mut self) {}

    fn reverse_index(&mut self) {}

    fn identify_terminal(&mut self, _: Option<char>) {}

    fn reset_state(&mut self) {}

    fn save_cursor_position(&mut self) {}

    fn restore_cursor_position(&mut self) {}

    fn screen_alignment_display(&mut self) {}

    fn set_keypad_application_mode(&mut self) {}

    fn unset_keypad_application_mode(&mut self) {}

    fn configure_charset(&mut self, _: Charset, _: CharsetIndex) {}

    fn reset_color(&mut self, _: usize) {}

    fn set_color(&mut self, _: usize, _: Rgb) {}

    fn color_query(&mut self, _: usize, _: &str) {}

    fn clipboard_load(&mut self, _: u8, _: &str) {}

    fn clipboard_store(&mut self, _: u8, _: &[u8]) {}

    fn set_cursor_shape(&mut self, _: CursorShape) {}

    fn set_mouse_cursor_icon(&mut self, _: CursorIcon) {}

    fn set_hyperlink(&mut self, _: Option<Hyperlink>) {}

    fn insert_blank(&mut self, _: usize) {}

    fn move_up(&mut self, _: usize) {}

    fn move_down(&mut self, _: usize) {}

    fn move_forward(&mut self, _: usize) {}

    fn move_backward(&mut self, _col: usize) {}

    fn goto_line(&mut self, _: i32) {}

    fn move_down_and_cr(&mut self, _: usize) {}

    fn move_up_and_cr(&mut self, _: usize) {}

    fn goto_col(&mut self, _: usize) {}

    fn set_tabs(&mut self, _: u16) {}

    fn clear_tabs(&mut self, _: TabClearMode) {}

    fn goto(&mut self, _: i32, _: usize) {}

    fn set_mode(&mut self, _: Mode) {}

    fn set_private_mode(&mut self, _: PrivateMode) {}

    fn unset_mode(&mut self, _: Mode) {}

    fn unset_private_mode(&mut self, _: PrivateMode) {}

    fn move_forward_tabs(&mut self, _: u16) {}

    fn clear_screen(&mut self, _: ClearMode) {}

    fn clear_line(&mut self, _: LineClearMode) {}

    fn set_scp(&mut self, _: ScpCharPath, _: ScpUpdateMode) {}

    fn insert_blank_lines(&mut self, _: usize) {}

    fn delete_lines(&mut self, _: usize) {}

    fn terminal_attribute(&mut self, _: Attr) {}

    fn set_modify_other_keys(&mut self, _: ModifyOtherKeys) {}

    fn report_modify_other_keys(&mut self) {}

    fn device_status(&mut self, _: usize) {}

    fn delete_chars(&mut self, _: usize) {}

    fn report_mode(&mut self, _: Mode) {}

    fn report_private_mode(&mut self, _: PrivateMode) {}

    fn set_cursor_style(&mut self, _: Option<CursorStyle>) {}

    fn set_scrolling_region(&mut self, _: usize, _: Option<usize>) {}

    fn scroll_up(&mut self, _: usize) {}

    fn scroll_down(&mut self, _: usize) {}

    fn text_area_size_pixels(&mut self) {}

    fn text_area_size_chars(&mut self) {}

    fn push_title(&mut self) {}

    fn pop_title(&mut self) {}

    fn report_keyboard_mode(&mut self) {}

    fn set_keyboard_mode(
        &mut self,
        _mode: KeyboardModes,
        _behavior: KeyboardModesApplyBehavior,
    ) {
    }

    fn push_keyboard_mode(&mut self, _mode: KeyboardModes) {}

    fn pop_keyboard_modes(&mut self, _to_pop: u16) {}

    fn newline(&mut self) {}

    fn erase_chars(&mut self, _: usize) {}

    fn move_backward_tabs(&mut self, _count: u16) {}
}
