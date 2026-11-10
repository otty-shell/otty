//! The events the app listens to on its own: window events, the
//! subscriptions of the frame widgets and the timers of the features. The
//! I/O streams of the features, such as file watching (issue 07), Docker
//! events and logs (issue 13) and the terminal sessions (issue 11), start
//! here when they move in.

#![allow(
    clippy::disallowed_methods,
    reason = "subscriptions start the I/O streams of the features"
)]

use iced::{Subscription, window};
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::Feature;

use crate::app::App;
use crate::message::AppEvent;

/// Build the active subscription set from current app state.
pub(crate) fn subscription(app: &App) -> Subscription<AppEvent> {
    let window_events =
        window::events().map(|(_id, event)| AppEvent::Window(event));

    // Frames for the focus mode note while it stays and fades out.
    let sidebar = app.sidebar.subscription().map(AppEvent::Sidebar);

    Subscription::batch([window_events, sidebar, toast_timer(app)])
}

/// The toast timer, while the notifications feature asks for it. It runs
/// here because the iced timer needs the async runtime of the binary.
fn toast_timer(app: &App) -> Subscription<AppEvent> {
    let Some(every) = app.notifications.toast_timer() else {
        return Subscription::none();
    };

    iced::time::every(every)
        .map(|_| AppEvent::Notifications(NotificationsIntent::Tick))
}
