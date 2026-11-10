//! The events the app listens to on its own: window events, the
//! subscriptions of the frame widgets and of the features, the timers of
//! the features and their I/O streams. The explorer's file watch, the events
//! watch of the container engines and the followed logs of the containers
//! inspector run here. A feature decides what runs (a timer period, the
//! folders to watch, the logs to follow); this module only runs it. The terminal owns its live sessions (see the spec,
//! "Exceptions"), so the events of its terminals come from its own
//! subscription.

#![allow(
    clippy::disallowed_methods,
    reason = "subscriptions start the I/O streams of the features"
)]

mod changed_folders;
mod engine_watch;
mod explorer_watch;
mod followed_logs;

use iced::{Subscription, window};
use otty_app_containers::{ContainersIntent, RunIntent};
use otty_app_notifications::NotificationsIntent;
use otty_app_quick_launch::QuickLaunchIntent;
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

    // Containers mode only: engine events keep the tables fresh, and the
    // Logs tab of the inspector follows the logs of its container.
    let engine_watch =
        engine_watch::subscription(app.containers.watched_engines());
    let followed_logs =
        followed_logs::subscription(app.containers.followed_logs());

    Subscription::batch([
        window_events,
        sidebar,
        toast_timer(app),
        launches_timer(app),
        explorer_watch,
        terminals,
        engine_watch,
        followed_logs,
        engine_search_timer(app),
        folder_recheck_timer(app),
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

/// The quick launch tick, while the feature asks for it: a running launch
/// blinks its row and a changed tree is saved.
fn launches_timer(app: &App) -> Subscription<AppEvent> {
    let Some(every) = app.quick_launch.tick_timer() else {
        return Subscription::none();
    };

    iced::time::every(every)
        .map(|_| AppEvent::QuickLaunch(QuickLaunchIntent::Tick))
}

/// The timer of the no-engine card, while the containers feature asks for
/// it: an engine that starts takes over by itself.
fn engine_search_timer(app: &App) -> Subscription<AppEvent> {
    let Some(every) = app.containers.engine_search_timer() else {
        return Subscription::none();
    };

    iced::time::every(every)
        .map(|_| AppEvent::Containers(ContainersIntent::EngineSearchTick))
}

/// The timer of the Run form's folders, while the containers feature asks
/// for it: a folder created elsewhere clears its error by itself.
fn folder_recheck_timer(app: &App) -> Subscription<AppEvent> {
    let Some(every) = app.containers.folder_recheck_timer() else {
        return Subscription::none();
    };

    iced::time::every(every).map(|_| {
        AppEvent::Containers(ContainersIntent::Run(RunIntent::RecheckFolders))
    })
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

    /// While the launch tree waits to be saved, quick launch asks for its
    /// tick, and the app runs it.
    #[test]
    fn a_changed_launch_tree_starts_the_quick_launch_tick() {
        let mut app = app();
        let without = subscription(&app).units();

        let _folder = dispatch::dispatch(
            &mut app,
            AppEvent::QuickLaunch(QuickLaunchIntent::HeaderCreateFolder),
        );
        let _named = dispatch::dispatch(
            &mut app,
            AppEvent::QuickLaunch(QuickLaunchIntent::InlineEditChanged(
                String::from("servers"),
            )),
        );
        let _saved = dispatch::dispatch(
            &mut app,
            AppEvent::QuickLaunch(QuickLaunchIntent::InlineEditSubmit),
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

    /// Containers mode watches the events of each found engine; another
    /// mode watches none.
    #[test]
    fn found_engines_are_watched_in_containers_mode_only() {
        use otty_containers::{EngineKind, fixtures};

        let mut app = app();
        let without = subscription(&app).units();

        for intent in [
            ContainersIntent::Enter,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
        ] {
            let _ = dispatch::dispatch(&mut app, AppEvent::Containers(intent));
        }
        let in_containers = subscription(&app).units();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Containers(ContainersIntent::Leave),
        );

        assert_eq!(in_containers, without + 2);
        assert_eq!(subscription(&app).units(), without);
    }

    /// While the no-engine card shows, the app looks for engines on a
    /// timer.
    #[test]
    fn the_no_engine_card_starts_the_engine_search_timer() {
        let mut app = app();
        let without = subscription(&app).units();

        for intent in [
            ContainersIntent::Enter,
            ContainersIntent::DiscoveryFinished(
                otty_containers::fixtures::discovery(&[]),
            ),
        ] {
            let _ = dispatch::dispatch(&mut app, AppEvent::Containers(intent));
        }

        assert_eq!(subscription(&app).units(), without + 1);
    }

    /// The Logs tab of the selected container follows its logs in one
    /// stream; another tab stops it.
    #[test]
    fn the_logs_tab_starts_one_logs_stream_and_another_tab_stops_it() {
        use otty_app_containers::InspectorTab;

        use crate::testing::window::containers_app_with_lists;

        let mut app = containers_app_with_lists();
        let without = subscription(&app).units();

        let _ = crate::update::update_input(
            &mut app,
            AppEvent::Containers(ContainersIntent::SelectInspectorTab(
                InspectorTab::Logs,
            )),
        );
        let on_logs = subscription(&app).units();
        let _ = crate::update::update_input(
            &mut app,
            AppEvent::Containers(ContainersIntent::SelectInspectorTab(
                InspectorTab::Inspect,
            )),
        );

        assert_eq!(on_logs, without + 1);
        assert_eq!(subscription(&app).units(), without);
    }

    /// A folder row in the Run form asks the disk again on a timer, so a
    /// folder created elsewhere clears its error by itself.
    #[test]
    fn a_folder_row_of_the_run_form_starts_the_folder_timer() {
        use otty_app_containers::RunEdit;

        use crate::testing::window::containers_app_with_lists;

        let mut app = containers_app_with_lists();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Containers(ContainersIntent::OpenCreate),
        );
        let without = subscription(&app).units();

        for edit in [
            RunEdit::AddVolume,
            RunEdit::VolumeSource(0, String::from("/srv/site")),
        ] {
            let _ = dispatch::dispatch(
                &mut app,
                AppEvent::Containers(ContainersIntent::Run(RunIntent::Edited(
                    edit,
                ))),
            );
        }
        let with_row = subscription(&app).units();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Containers(ContainersIntent::Leave),
        );

        assert_eq!(with_row, without + 1);
        assert_eq!(subscription(&app).units(), without - 1);
    }
}
