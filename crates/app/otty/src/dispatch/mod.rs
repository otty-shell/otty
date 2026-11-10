//! Delivery: hands each [`AppEvent`] to the one that handles it, and reads
//! what comes back.
//!
//! Every component, a feature or a frame widget, goes through one function,
//! [`deliver`]: the component reduces the intent once, then every outcome
//! it reports is read, in order, in the same update. The [`Delivery`] trait
//! fixes that shape, so no component has a delivery of its own.
//!
//! `dispatch/x.rs` implements [`Delivery`] for the intent of component `x`
//! and holds only two things: `parts`, which takes the component and the
//! ctx it reads from the fields of `App`, and `read`, which turns one
//! outcome into one call. There are three kinds of outcome:
//!
//! 1. news for another widget or feature: its `deliver` runs right away, in
//!    the same update;
//! 2. I/O: the feature's `io` crate or a helper crate starts it, and the
//!    answer comes back later as an [`AppEvent`];
//! 3. a window operation, such as moving the keyboard focus: an iced task.
//!
//! `read` makes one call per arm and no decisions: a decision about a
//! component's state lives in its `reduce`. The compiler checks the shape;
//! review checks that `read` stays this thin.
//!
//! A command (`dispatch/command.rs`) and the window events below are not
//! components: they are plain functions that call [`deliver`].

mod command;
mod containers;
mod delivery;
mod explorer;
mod modes;
mod notifications;
mod palette;
mod quick_launch;
mod settings;
mod sidebar;
mod tabs;
mod terminal;
mod title_bar;

use iced::Task;
use iced::widget::operation;
use otty_app_containers::ContainersIntent;
use otty_app_slots::focus;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

pub(crate) use self::delivery::{Delivery, deliver};
use crate::app::App;
use crate::layout;
use crate::message::AppEvent;

/// Hand an app event to the one that handles it: a widget or feature intent
/// to [`deliver`], a command to [`command::deliver`]. The I/O answers of the
/// start and the events of the window itself are handled here.
pub(crate) fn dispatch(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::TitleBar(intent) => deliver(app, intent),
        AppEvent::Sidebar(intent) => deliver(app, intent),
        AppEvent::Modes(intent) => deliver(app, intent),
        AppEvent::Notifications(intent) => deliver(app, intent),
        AppEvent::Explorer(intent) => deliver(app, intent),
        AppEvent::Palette(intent) => deliver(app, intent),
        AppEvent::Settings(intent) => deliver(app, intent),
        AppEvent::Tabs(intent) => deliver(app, intent),
        AppEvent::Terminal(intent) => deliver(app, intent),
        AppEvent::QuickLaunch(intent) => deliver(app, intent),
        AppEvent::Containers(intent) => deliver(app, intent),
        AppEvent::Command(command) => command::deliver(app, command),
        AppEvent::ShellIntegrated(shell_session) => {
            app.shell_session = shell_session;

            Task::none()
        },
        AppEvent::HostNameRead(host_name) => {
            deliver(app, TabsIntent::HostNameRead(host_name))
        },
        AppEvent::ResizeWindow(direction) => resize_window(direction),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            app.screen_size = layout::screen_size_from_window(size);

            // The pane grids take the new main area, and the containers row
            // menu is kept inside the new window.
            let panes = deliver(app, TerminalIntent::SyncPaneGridSize);
            let containers =
                deliver(app, ContainersIntent::WindowResized(size));

            Task::batch([panes, containers])
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

    /// The containers row menu opens at the pointer and is kept inside the
    /// window, which the containers feature is told about.
    #[test]
    fn a_new_window_size_keeps_the_containers_row_menu_inside_it() {
        use otty_app_slots::{Component, Overlays};
        use otty_app_theme::theme::ThemeProps;

        let window = iced::Size::new(600.0, 400.0);
        let mut app = crate::testing::window::containers_app_with_lists();

        let _none = dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(window)),
        );
        let _keyboard = app.containers.reduce(
            ContainersIntent::OpenRowMenu {
                index: 0,
                position: iced::Point::new(590.0, 390.0),
            },
            (),
        );
        let theme = app.theme_manager.current().clone();
        let menu = app
            .containers
            .overlay(ThemeProps::new(&theme))
            .expect("the row menu")
            .into_content();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            window,
            menu,
        );
        let item = ui
            .find(otty_app_i18n::t(otty_app_i18n::Key::ContainersActionRemove))
            .expect("the menu should offer Remove")
            .bounds();

        assert!(item.x + item.width <= window.width, "{item:?}");
        assert!(item.y + item.height <= window.height, "{item:?}");
    }

    #[test]
    fn an_integrated_shell_session_is_written_to_the_app() {
        let mut app = app();
        let session = otty_app_shell::ShellSession::without_integration(
            "/opt/otty-test/otty-test-shell",
        );

        let _none = dispatch(&mut app, AppEvent::ShellIntegrated(session));

        assert_eq!(app.shell_session.name(), "otty-test-shell");
    }
}
