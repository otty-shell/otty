/// State of the title bar: whether the left button is held on it.
#[derive(Default)]
pub(super) struct TitleBarState {
    is_title_bar_pressed: bool,
}

impl TitleBarState {
    /// Return whether the left button is held on the empty title bar and
    /// the window drag has not started yet.
    pub(crate) fn is_title_bar_pressed(&self) -> bool {
        self.is_title_bar_pressed
    }

    /// Remember a press on the empty title bar.
    pub(crate) fn press_title_bar(&mut self) {
        self.is_title_bar_pressed = true;
    }

    /// Forget the title bar press.
    pub(crate) fn release_title_bar(&mut self) {
        self.is_title_bar_pressed = false;
    }
}
