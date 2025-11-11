use crate::grid::ScrollDirection;
use cursor_icon::CursorIcon;
use otty_escape::{
    CharacterAttribute, Charset, CharsetIndex, ClearMode, Hyperlink,
    LineClearMode, Mode, PrivateMode, Rgb, TabClearMode,
};

pub trait SurfaceActor {
    fn print(&mut self, _ch: char) {}

    fn resize(&mut self, _columns: usize, _rows: usize) {}

    fn insert_blank(&mut self, _count: usize) {}

    fn insert_blank_lines(&mut self, _count: usize) {}

    fn delete_lines(&mut self, _count: usize) {}

    fn delete_chars(&mut self, _count: usize) {}

    fn erase_chars(&mut self, _count: usize) {}

    fn backspace(&mut self) {}

    fn carriage_return(&mut self) {}

    fn line_feed(&mut self) {}

    fn new_line(&mut self) {}

    fn set_horizontal_tab(&mut self) {}

    fn reverse_index(&mut self) {}

    fn reset(&mut self) {}

    fn clear_screen(&mut self, _mode: ClearMode) {}

    fn clear_line(&mut self, _mode: LineClearMode) {}

    fn insert_tabs(&mut self, _count: usize) {}

    fn set_tabs(&mut self, _mask: u16) {}

    fn clear_tabs(&mut self, _mode: TabClearMode) {}

    fn screen_alignment_display(&mut self) {}

    fn move_forward_tabs(&mut self, _count: usize) {}

    fn move_backward_tabs(&mut self, _count: usize) {}

    fn set_active_charset_index(&mut self, _index: CharsetIndex) {}

    fn configure_charset(&mut self, _charset: Charset, _index: CharsetIndex) {}

    fn set_color(&mut self, _index: usize, _color: Rgb) {}

    fn query_color(&mut self, _index: usize) {}

    fn reset_color(&mut self, _index: usize) {}

    fn set_scrolling_region(&mut self, _top: usize, _bottom: usize) {}

    fn scroll_up(&mut self, _count: usize) {}

    fn scroll_down(&mut self, _count: usize) {}

    fn set_hyperlink(&mut self, _link: Option<Hyperlink>) {}

    fn sgr(&mut self, _attribute: CharacterAttribute) {}

    fn set_cursor_shape(&mut self, _shape: otty_escape::CursorShape) {}

    fn set_cursor_icon(&mut self, _icon: CursorIcon) {}

    fn set_cursor_style(&mut self, _style: Option<otty_escape::CursorStyle>) {}

    fn save_cursor(&mut self) {}

    fn restore_cursor(&mut self) {}

    fn move_up(&mut self, _rows: usize, _carriage_return: bool) {}

    fn move_down(&mut self, _rows: usize, _carriage_return: bool) {}

    fn move_forward(&mut self, _cols: usize) {}

    fn move_backward(&mut self, _cols: usize) {}

    fn goto(&mut self, _row: i32, _col: usize) {}

    fn goto_row(&mut self, _row: i32) {}

    fn goto_column(&mut self, _col: usize) {}

    fn set_keypad_application_mode(&mut self, _enabled: bool) {}

    fn push_keyboard_mode(&mut self) {}

    fn pop_keyboard_modes(&mut self, _amount: u16) {}

    fn set_mode(&mut self, _mode: Mode, _enabled: bool) {}

    fn set_private_mode(&mut self, _mode: PrivateMode, _enabled: bool) {}

    fn push_window_title(&mut self) {}

    fn pop_window_title(&mut self) {}

    fn set_window_title(&mut self, _title: String) {}

    fn begin_sync(&mut self) {}

    fn end_sync(&mut self) {}

    fn scroll_display(&mut self, _direction: ScrollDirection) {}

    fn enter_altscreem(&mut self) {}

    fn exit_altscreem(&mut self) {}

    fn decolumn(&mut self) {}
}
