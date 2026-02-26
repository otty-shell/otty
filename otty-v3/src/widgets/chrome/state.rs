/// Chrome window decoration state.
#[derive(Default)]
pub(crate) struct ChromeState {
    is_fullscreen: bool,
}

impl ChromeState {
    /// Return whether the window is fullscreen.
    pub(crate) fn is_fullscreen(&self) -> bool {
        self.is_fullscreen
    }

    /// Toggle fullscreen state and return the new value.
    pub(crate) fn toggle_fullscreen(&mut self) -> bool {
        self.is_fullscreen = !self.is_fullscreen;
        self.is_fullscreen
    }
}
