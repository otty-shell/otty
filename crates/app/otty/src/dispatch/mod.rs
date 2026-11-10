//! Delivery: hands each [`AppEvent`] to the one that handles it, and reads
//! what comes back.
//!
//! `dispatch/x.rs` gives the intents of widget `x` to its `reduce` and reads
//! each outcome. There are three kinds of outcome, and each arm is one call:
//!
//! 1. news for another widget or feature: its `reduce` runs right away, in
//!    the same update;
//! 2. I/O: `effects/` starts it, and the answer comes back later as an
//!    [`AppEvent`];
//! 3. a window operation, such as moving the keyboard focus: an iced task.
//!
//! Delivery makes no decisions: a decision about a widget's state lives in
//! its `reduce`.

pub(crate) mod chrome;
pub(crate) mod command;
pub(crate) mod containers;
pub(crate) mod explorer;
pub(crate) mod modes;
pub(crate) mod notifications;
pub(crate) mod palette;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod terminal;

use iced::Task;
use iced::widget::operation;
use otty_app_slots::focus;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

use crate::app::App;
use crate::layout;
use crate::message::AppEvent;

/// Hand an app event to the one that handles it: a widget or feature intent
/// to its delivery, a command to [`command::deliver`]. The I/O answers of
/// the start and the events of the window itself are handled here.
pub(crate) fn dispatch(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::Chrome(intent) => chrome::deliver(app, intent),
        AppEvent::Sidebar(intent) => sidebar::deliver(app, intent),
        AppEvent::Modes(intent) => modes::deliver(app, intent),
        AppEvent::Notifications(intent) => notifications::deliver(app, intent),
        AppEvent::Explorer(intent) => explorer::deliver(app, intent),
        AppEvent::Palette(intent) => palette::deliver(app, intent),
        AppEvent::Settings(intent) => settings::deliver(app, intent),
        AppEvent::Tabs(intent) => tabs::deliver(app, intent),
        AppEvent::Terminal(intent) => terminal::deliver(app, intent),
        AppEvent::QuickLaunch(intent) => quick_launch::deliver(app, intent),
        AppEvent::Containers(intent) => containers::deliver(app, intent),
        AppEvent::Command(command) => command::deliver(app, command),
        AppEvent::ShellIntegrated(shell_session) => {
            app.shell_session = shell_session;

            Task::none()
        },
        AppEvent::HostNameRead(host_name) => {
            tabs::deliver(app, TabsIntent::HostNameRead(host_name))
        },
        AppEvent::ResizeWindow(direction) => resize_window(direction),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            app.screen_size = layout::screen_size_from_window(size);

            // The pane grids take the new main area.
            terminal::deliver(app, TerminalIntent::SyncPaneGridSize)
        },
        // A shortcut key arrives as its command (see `update_input`); other
        // keys no widget took and other window events do nothing.
        AppEvent::Keyboard(_) | AppEvent::Window(_) => Task::none(),
    }
}

/// Take the keyboard focus from every widget, the focused terminal too: an
/// open menu takes the keys, so typed keys reach no shell under it and Esc
/// reaches the frame.
pub(crate) fn take_keyboard() -> Task<AppEvent> {
    operation::focus(focus::NOTHING)
}

/// Let the window manager resize the window from the grip at `direction`.
#[cfg(not(target_os = "macos"))]
fn resize_window(direction: iced::window::Direction) -> Task<AppEvent> {
    iced::window::latest()
        .and_then(move |id| iced::window::drag_resize(id, direction))
}

/// The system resizes the window on macOS.
#[cfg(target_os = "macos")]
fn resize_window(_direction: iced::window::Direction) -> Task<AppEvent> {
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::window::app;

    /// The host name reaches the tabs, which name this computer with it.
    #[test]
    fn a_read_host_name_reaches_the_tabs() {
        let mut app = app();

        let _none = dispatch(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );

        assert_eq!(app.tabs.active_machine_name(), "build-box");
    }

    /// The header menus are kept inside the main area, which follows the
    /// window size.
    #[test]
    fn a_new_window_size_gives_the_main_area_its_size() {
        let mut app = app();

        let _none = dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(iced::Size::new(
                1200.0, 900.0,
            ))),
        );

        assert_eq!(
            app.screen_size,
            layout::screen_size_from_window(iced::Size::new(1200.0, 900.0))
        );
    }

    #[test]
    fn an_integrated_shell_session_is_written_to_the_app() {
        let mut app = app();
        let session = crate::shell_session::ShellSession::without_integration(
            "/opt/otty-test/otty-test-shell",
        );

        let _none = dispatch(&mut app, AppEvent::ShellIntegrated(session));

        assert_eq!(app.shell_session.name(), "otty-test-shell");
    }
}
