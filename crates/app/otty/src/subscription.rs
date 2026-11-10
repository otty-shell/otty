//! The events the app listens to on its own: window events and the
//! subscriptions of the frame widgets. The I/O streams of the features, such
//! as file watching (issue 07), Docker events and logs (issue 13) and the
//! terminal sessions (issue 11), start here when they move in.

#![allow(
    clippy::disallowed_methods,
    reason = "subscriptions start the I/O streams of the features"
)]

use iced::{Subscription, window};
use otty_app_slots::Feature;

use crate::app::App;
use crate::message::AppEvent;

/// Build the active subscription set from current app state.
pub(crate) fn subscription(app: &App) -> Subscription<AppEvent> {
    let window_events =
        window::events().map(|(_id, event)| AppEvent::Window(event));

    // Frames for the focus mode note while it stays and fades out.
    let sidebar = app.sidebar.subscription().map(AppEvent::Sidebar);

    Subscription::batch([window_events, sidebar])
}
