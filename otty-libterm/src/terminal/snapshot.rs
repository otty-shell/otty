use crate::{TerminalSize, surface::SurfaceSnapshot};

pub struct TerminalSnapshot<'a> {
    pub surface: SurfaceSnapshot<'a>,
    pub size: TerminalSize,
}

impl<'a> TerminalSnapshot<'a> {
    pub fn new(surface: SurfaceSnapshot<'a>, size: TerminalSize) -> Self {
        Self { surface, size }
    }
}
