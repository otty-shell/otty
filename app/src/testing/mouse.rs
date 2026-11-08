//! Mouse input for view tests that `iced_test` has no helper for.

use iced::Event;
use iced::mouse::{self, Button};

/// A press and a release of the middle button where the pointer is.
pub(crate) fn middle_click() -> [Event; 2] {
    [
        Event::Mouse(mouse::Event::ButtonPressed(Button::Middle)),
        Event::Mouse(mouse::Event::ButtonReleased(Button::Middle)),
    ]
}
