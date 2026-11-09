//! The toast timer: a tick four times a second while a toast shows, so a
//! toast goes away about five seconds after it came.

use std::time::Duration;

use iced::Subscription;

use super::event::{NotificationsEvent, NotificationsIntent};

/// Time between two ticks of the toast timer.
const TICK: Duration = Duration::from_millis(250);

/// Tick while `has_toasts`; with no toast on screen nothing runs.
pub(super) fn subscription(
    has_toasts: bool,
) -> Subscription<NotificationsEvent> {
    if !has_toasts {
        return Subscription::none();
    }

    iced::time::every(TICK)
        .map(|_| NotificationsEvent::Intent(NotificationsIntent::Tick))
}
