use crate::surface::SurfaceSnapshot;
use crate::{TerminalMode, escape::KeyboardMode};

#[derive(Debug, Clone)]
pub struct TerminalSnapshot {
    pub surface: SurfaceSnapshot,
    pub terminal_mode: TerminalMode,
    pub keyboard_mode: KeyboardMode,
}

impl TerminalSnapshot {
    pub fn new(
        surface: SurfaceSnapshot,
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
