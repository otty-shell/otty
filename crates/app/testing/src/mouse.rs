//! Mouse input for view tests that `iced_test` has no helper for.

use iced::Event;
use iced::mouse::{self, Button};

/// A press and a release of the middle button where the pointer is.
///
/// # Example
///
/// ```
/// use otty_app_testing::mouse::middle_click;
///
/// let [press, release] = middle_click();
///
/// assert!(matches!(
///     press,
///     iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
///         iced::mouse::Button::Middle
///     ))
/// ));
/// assert!(matches!(
///     release,
///     iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
///         iced::mouse::Button::Middle
///     ))
/// ));
/// ```
pub fn middle_click() -> [Event; 2] {
    click_of(Button::Middle)
}

/// A press and a release of the right button where the pointer is.
///
/// # Example
///
/// ```
/// use otty_app_testing::mouse::right_click;
///
/// let [press, _release] = right_click();
///
/// assert!(matches!(
///     press,
///     iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
///         iced::mouse::Button::Right
///     ))
/// ));
/// ```
pub fn right_click() -> [Event; 2] {
    click_of(Button::Right)
}

fn click_of(button: Button) -> [Event; 2] {
    [
        Event::Mouse(mouse::Event::ButtonPressed(button)),
        Event::Mouse(mouse::Event::ButtonReleased(button)),
    ]
}
