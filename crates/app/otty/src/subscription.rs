//! The events the app listens to on its own: window events, the
//! subscriptions of the frame widgets, the timers of the features and their
//! I/O streams. The explorer's file watch runs here; Docker events and logs
//! (issue 13) and the terminal sessions (issue 11) start here when they move
//! in. A feature decides what runs (a timer period, the folders to watch);
//! this module only runs it.

#![allow(
    clippy::disallowed_methods,
    reason = "subscriptions start the I/O streams of the features"
)]

mod changed_folders;
mod explorer_watch;

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

    // The folders of the Files section, watched for changes on disk.
    let explorer_watch =
        explorer_watch::subscription(app.explorer.watched_folders());

    Subscription::batch([
        window_events,
        sidebar,
        toast_timer(app),
        explorer_watch,
    ])
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_app_explorer::ExplorerIntent;

    use super::*;
    use crate::dispatch;
    use crate::testing::window::app;

    /// Once the explorer follows a folder, the app watches it for changes.
    #[test]
    fn a_followed_folder_starts_the_file_watch() {
        let mut app = app();
        let without = subscription(&app).units();

        let _read = dispatch::dispatch(
            &mut app,
            AppEvent::Explorer(ExplorerIntent::SyncRoot {
                cwd: PathBuf::from("/tmp/otty-explorer-watch"),
            }),
        );

        assert_eq!(subscription(&app).units(), without + 1);
    }
}
