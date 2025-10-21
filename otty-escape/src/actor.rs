//! High-level escape sequence consumer interface.
//!
//! The [`Parser`](crate::Parser) translates the raw byte stream into semantic
//! events and relays them to an [`Actor`] implementation.  Downstream crates
//! can implement this trait to mutate their terminal model, update UI state or
//! collect metrics without re-implementing the escape sequence finite state
//! machine.

use crate::{
    charset::{Charset, CharsetIndex},
};

/// Trait implemented by consumers of the escape sequence parser.
///
/// All methods have a default empty implementation so that downstream crates
/// only need to override the variants they actually care about.  The parser
/// will invoke these callbacks synchronously while it walks through the input
/// byte stream.
pub trait Actor {
    /// Emits a printable Unicode scalar value.
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

    fn configure_charset(&mut self, _: Charset) {}

    fn reset_color(&mut self, _: usize) {}
}
