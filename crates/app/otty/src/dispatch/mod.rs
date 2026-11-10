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
pub(crate) mod modes;
pub(crate) mod sidebar;

use iced::Task;
use iced::widget::operation;
use otty_app_slots::focus;

use crate::app::App;
use crate::message::AppEvent;

/// Hand an app event to the one that handles it: a widget intent to the
/// widget's delivery, a command to [`command::deliver`]. The I/O answers of
/// the start and the events of the window itself are handled here.
pub(crate) fn dispatch(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::Chrome(intent) => chrome::deliver(app, intent),
        AppEvent::Sidebar(intent) => sidebar::deliver(app, intent),
        AppEvent::Modes(intent) => modes::deliver(app, intent),
        AppEvent::Command(command) => command::deliver(app, command),
        AppEvent::ShellIntegrated(shell_session) => {
            app.shell_session = shell_session;

            Task::none()
        },
        AppEvent::HostNameRead(host_name) => {
            app.host_name = host_name;

            Task::none()
        },
        AppEvent::ResizeWindow(direction) => resize_window(direction),
        // A new window size resizes the terminal pane grids; the terminal
        // feature (issue 11) is told here. A shortcut key arrives as its
        // command (see `update_input`); other keys no widget took do
        // nothing.
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

    #[test]
    fn a_read_host_name_is_written_to_the_app() {
        let mut app = app();

        let _none = dispatch(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );

        assert_eq!(app.host_name.as_deref(), Some("build-box"));
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
