//! The events the app listens to on its own: window events, the
//! subscriptions of the frame widgets and of the features, and the I/O
//! streams of the features. A feature gives what needs no I/O (its timers,
//! the events of its own terminals) through its own `subscription()`; an
//! I/O stream comes from the feature's `io` crate, which watches what the
//! feature asks for (the folders, the engines, the logs to follow). This
//! module runs them and wraps them into [`AppEvent`]. The one timer it
//! starts itself is the clock of the terminal's block statuses: the
//! terminal feature builds iced without an async runtime, so it has no
//! timer of its own.

use std::time::Duration;

use iced::{Subscription, window};
use otty_app_containers_io as containers_io;
use otty_app_domain::mode::Mode;
use otty_app_explorer_io as explorer_io;
use otty_app_slots::Component;
use otty_app_terminal::TerminalIntent;

use crate::app::App;
use crate::message::AppEvent;

/// How often the status of a running command shows its new time.
const BLOCK_CLOCK_PERIOD: Duration = Duration::from_secs(1);

/// Build the active subscription set from current app state.
pub(crate) fn subscription(app: &App) -> Subscription<AppEvent> {
    let window_events =
        window::events().map(|(_id, event)| AppEvent::Window(event));

    // Frames for the focus mode note while it stays and fades out.
    let sidebar = app.sidebar.subscription().map(AppEvent::Sidebar);

    // The timers of the features: the toasts, the quick launch tick, the
    // engine search, the Run form's folder recheck and the limits of the
    // agents.
    let notifications = app
        .notifications
        .subscription()
        .map(AppEvent::Notifications);
    let quick_launch =
        app.quick_launch.subscription().map(AppEvent::QuickLaunch);
    let containers = app.containers.subscription().map(AppEvent::Containers);
    let agents = app.agents.subscription().map(AppEvent::Agents);

    // The folders of the Files section, watched for changes on disk.
    let explorer_watch =
        explorer_io::watch_folders(app.explorer.watched_folders())
            .map(AppEvent::Explorer);

    // The output, titles and ends of every open terminal.
    let terminals = app.terminal.subscription().map(AppEvent::Terminal);

    // Terminal mode only: while a command runs in a pane on screen, its
    // status counts the seconds, so the window is drawn once a second.
    // The terminal feature can not start a timer of its own: its iced has
    // no async runtime.
    let block_clock = if app.modes.active() == Mode::Terminal
        && app.terminal.has_running_command_in_front()
    {
        iced::time::every(BLOCK_CLOCK_PERIOD)
            .map(|_| AppEvent::Terminal(TerminalIntent::BlockClockTick))
    } else {
        Subscription::none()
    };

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
        agents,
        explorer_watch,
        terminals,
        block_clock,
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

    /// Make the focused terminal of the tab in front show one command
    /// block that `is_finished` or still runs.
    fn report_command(
        app: &mut App,
        terminals: &otty_app_testing::terminal::ScriptedTerminals,
        is_finished: bool,
    ) {
        use otty_app_testing::blocks::{command_block, finished};
        use otty_ui_term::SnapshotOwned;

        let terminal_id = app
            .terminal
            .focused_terminal_id()
            .expect("the tab in front has a terminal");
        let mut block = command_block("cmd-1", 1_000);
        if is_finished {
            block = finished(block, Some(0), 1_000);
        }
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks.push(block);
        terminals.report(terminal_id, snapshot);

        let _ = dispatch::dispatch(
            app,
            AppEvent::Terminal(TerminalIntent::Widget(
                otty_ui_term::Event::ContentSync { id: terminal_id },
            )),
        );
    }

    /// While a command runs in a pane of Terminal mode, its status counts
    /// the seconds, so the app ticks once a second; the tick stops with the
    /// command and outside Terminal mode.
    #[test]
    fn a_running_command_starts_the_block_clock_until_it_ends() {
        use otty_app_modes::ModesIntent;

        use crate::testing::window::app_with_scripted_terminals;

        let (mut app, terminals) = app_with_scripted_terminals();
        open_terminal_tab(&mut app);
        let at_prompt = subscription(&app).units();

        report_command(&mut app, &terminals, false);
        let running = subscription(&app).units();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let in_containers = subscription(&app).units();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Terminal), ());
        report_command(&mut app, &terminals, true);

        assert_eq!(running, at_prompt + 1);
        assert_eq!(in_containers, at_prompt);
        assert_eq!(subscription(&app).units(), at_prompt);
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

    /// Agents mode reads the limits again on a timer while it is on
    /// screen; another mode stops it.
    #[test]
    fn agents_mode_starts_the_limits_timer_until_another_mode_shows() {
        use otty_app_modes::ModesIntent;

        let mut app = app();
        let without = subscription(&app).units();

        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Agents)),
        );
        let in_agents = subscription(&app).units();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
        );

        assert_eq!(in_agents, without + 1);
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
