use crate::{TerminalMode, escape::KeyboardMode};
use otty_surface::Surface;
pub use otty_surface::SurfaceSnapshot;

// TODO: to otty-surface
pub trait SurfaceSnapshotSource {
    fn capture_snapshot(&self) -> SurfaceSnapshot;
}

impl SurfaceSnapshotSource for Surface {
    fn capture_snapshot(&self) -> SurfaceSnapshot {
        self.snapshot()
    }
}

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
