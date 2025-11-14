use otty_escape::KeyboardMode;

use crate::terminal::{mode::TerminalMode, surface::SurfaceSnapshot};


pub struct TerminalSnapshot<'a> {
    pub surface: SurfaceSnapshot<'a>,
    pub terminal_mode: TerminalMode,
    pub keyboard_mode: KeyboardMode,
}

impl<'a> TerminalSnapshot<'a> {
    pub fn new(
        surface: SurfaceSnapshot<'a>,
        terminal_mode: TerminalMode,
        keyboard_mode: KeyboardMode,
    ) -> Self {
        Self {
            surface,
            terminal_mode,
            keyboard_mode,
        }
    }
}
