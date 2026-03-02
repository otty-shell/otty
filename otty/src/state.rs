use iced::Size;

/// Tab bar height for pane grid calculations.
const TAB_BAR_HEIGHT: f32 = 25.0;

/// Window and screen geometry state.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) window_size: Size,
    pub(crate) screen_size: Size,
}

impl State {
    pub(crate) fn new(window_size: Size, screen_size: Size) -> Self {
        Self {
            window_size,
            screen_size,
        }
    }

    pub(crate) fn set_screen_size(&mut self, size: Size) {
        self.screen_size = size;
    }
}
