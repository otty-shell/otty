//! The events the app listens to on its own: window events, the
//! subscriptions of the frame widgets and of the features, and the I/O
//! streams of the features. A feature gives what needs no I/O (its timers,
//! the events of its own terminals) through its own `subscription()`; an
//! I/O stream comes from the feature's `io` crate, which watches what the
//! feature asks for (the folders, the engines, the logs to follow). This
//! module only runs them and wraps them into [`AppEvent`].

use iced::{Subscription, window};
use otty_app_containers_io as containers_io;
use otty_app_explorer_io as explorer_io;
use otty_app_slots::Component;

use crate::app::App;
use crate::message::AppEvent;

/// Build the active subscription set from current app state.
pub(crate) fn subscription(app: &App) -> Subscription<AppEvent> {
    let window_events =
        window::events().map(|(_id, event)| AppEvent::Window(event));

    // Frames for the focus mode note while it stays and fades out.
    let sidebar = app.sidebar.subscription().map(AppEvent::Sidebar);

    // The timers of the features: the toasts, the quick launch tick, the
    // engine search and the Run form's folder recheck.
    let notifications = app
        .notifications
        .subscription()
        .map(AppEvent::Notifications);
    let quick_launch =
        app.quick_launch.subscription().map(AppEvent::QuickLaunch);
    let containers = app.containers.subscription().map(AppEvent::Containers);

    // The folders of the Files section, watched for changes on disk.
    let explorer_watch =
        explorer_io::watch_folders(app.explorer.watched_folders())
            .map(AppEvent::Explorer);

    // The output, titles and ends of every open terminal.
    let terminals = app.terminal.subscription().map(AppEvent::Terminal);

    // Containers mode only: engine events keep the tables fresh, and the
    // Logs tab of the inspector follows the logs of its container.
    let engine_watch =
        containers_io::watch_engines(app.containers.watched_engines())
            .map(AppEvent::Containers);
    let followed_logs =
        containers_io::follow_logs(app.containers.followed_logs())
            .map(AppEvent::Containers);

    Subscription::batch([
        window_events,
        sidebar,
        notifications,
        quick_launch,
        containers,
        explorer_watch,
        terminals,
        engine_watch,
        followed_logs,
    ])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_app_containers::{ContainersIntent, RunIntent};
    use otty_app_explorer::ExplorerIntent;
    use otty_app_quick_launch::QuickLaunchIntent;

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
