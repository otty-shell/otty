//! The events the app listens to on its own: window events, the
//! subscriptions of the frame widgets and of the features, the timers of
//! the features and their I/O streams. The explorer's file watch runs here;
//! Docker events and logs (issue 13) start here when they move in. A
//! feature decides what runs (a timer period, the folders to watch); this
//! module only runs it. The terminal owns its live sessions (see the spec,
//! "Exceptions"), so the events of its terminals come from its own
//! subscription.

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

    // The output, titles and ends of every open terminal.
    let terminals = app.terminal.subscription().map(AppEvent::Terminal);

    Subscription::batch([
        window_events,
        sidebar,
        toast_timer(app),
        explorer_watch,
        terminals,
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
    use crate::testing::window::{app, open_terminal_tab};

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

    /// The events of each open terminal come in through the terminal's own
    /// subscription, one per pane.
    #[test]
    fn each_open_terminal_is_listened_to() {
        let mut app = app();
        let without = subscription(&app).units();

        open_terminal_tab(&mut app);
        let _split = dispatch::dispatch(
            &mut app,
            AppEvent::Command(
                otty_app_domain::commands::AppCommand::SplitRight,
            ),
        );

        assert_eq!(subscription(&app).units(), without + 2);
    }
}
