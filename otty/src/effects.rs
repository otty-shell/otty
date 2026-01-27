use iced::Task;

use crate::app::Event as AppEvent;

/// Close the active application window.
pub(crate) fn close_window() -> Task<AppEvent> {
    iced::window::latest().and_then(iced::window::close)
}
