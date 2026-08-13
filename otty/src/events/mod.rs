use iced::Task;
use iced::keyboard::{Key, Modifiers};
use iced::widget::pane_grid;
#[cfg(not(target_os = "macos"))]
use iced::window;
use iced::window::Direction;

use crate::app::App;
use crate::layout::screen_size_from_window;
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::explorer::ExplorerEvent;
use crate::widgets::quick_launch::QuickLaunchEvent;
use crate::widgets::settings::SettingsEvent;
use crate::widgets::sidebar::SidebarEvent;
use crate::widgets::tabs::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(crate) mod chrome;
pub(crate) mod explorer;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;

#[derive(Clone)]
pub(crate) enum AppEvent {
    Sidebar(SidebarEvent),
    Chrome(ChromeEvent),
    Tabs(TabsEvent),
    QuickLaunch(QuickLaunchEvent),
    TerminalWorkspace(TerminalWorkspaceEvent),
    Explorer(ExplorerEvent),
    Settings(SettingsEvent),
    IcedReady,
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

pub(crate) fn handle(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenTerminalTab {
                title: app.shell_session.name().to_string(),
            },
        ))),
        AppEvent::Sidebar(event) => sidebar::handle(app, event),
        AppEvent::Chrome(event) => chrome::handle(app, event),
        AppEvent::Tabs(event) => tabs::handle(app, event),
        AppEvent::QuickLaunch(event) => quick_launch::handle(app, event),
        AppEvent::TerminalWorkspace(event) => {
            terminal_workspace::handle(app, event)
        },
        AppEvent::Explorer(event) => explorer::handle(app, event),
        AppEvent::Settings(event) => settings::handle(app, event),
        AppEvent::SyncTerminalGridSizes => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )),
        ),
        AppEvent::Keyboard(event) => handle_keyboard(app, &event),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            app.window_size = size;
            app.state.window_size = size;
            app.state.set_screen_size(screen_size_from_window(size));

            terminal_workspace::handle(
                app,
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::SyncPaneGridSize,
                ),
            )
        },
        AppEvent::ResizeWindow(dir) => {
            #[cfg(target_os = "macos")]
            {
                let _ = dir;
                Task::none()
            }

            #[cfg(not(target_os = "macos"))]
            {
                window::latest()
                    .and_then(move |id| window::drag_resize(id, dir))
            }
        },
        AppEvent::Window(_) => Task::none(),
    }
}

/// Turn a keyboard shortcut into the event it triggers.
///
/// Only Cmd+D on macOS is bound today: it splits the focused pane of the
/// active tab side by side. Every other key is left to the focused
/// widget.
fn handle_keyboard(app: &App, event: &iced::keyboard::Event) -> Task<AppEvent> {
    let iced::keyboard::Event::KeyPressed {
        key,
        modifiers,
        repeat,
        ..
    } = event
    else {
        return Task::none();
    };

    if !is_split_pane_shortcut(key, modifiers, *repeat) {
        return Task::none();
    }

    let Some(tab_id) = app.widgets.tabs.active_tab_id() else {
        return Task::none();
    };
    let Some(pane) = app
        .widgets
        .terminal_workspace
        .tab(tab_id)
        .and_then(|tab| tab.focus())
    else {
        return Task::none();
    };

    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::SplitPane {
            tab_id,
            pane,
            axis: pane_grid::Axis::Vertical,
        },
    )))
}

/// Return whether this key press is the split-pane shortcut, Cmd+D.
///
/// Bound on macOS only. Elsewhere `Modifiers::COMMAND` resolves to
/// `CTRL`, and the terminal already binds Ctrl+D to EOF. The terminal
/// widget never marks keyboard events as captured, so claiming that
/// combination here would both send the control character and split the
/// pane. Binding this on other platforms needs a key that does not
/// collide with terminal control characters.
///
/// Auto-repeat is rejected: holding the shortcut down would otherwise
/// spawn one pane, terminal and shell process per repeat event.
///
/// The modifier set must match exactly, so Cmd+Shift+D and friends stay
/// free for other bindings.
fn is_split_pane_shortcut(
    key: &Key,
    modifiers: &Modifiers,
    repeat: bool,
) -> bool {
    if !cfg!(target_os = "macos") || repeat {
        return false;
    }

    if *modifiers != Modifiers::COMMAND {
        return false;
    }

    matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("d"))
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;

    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn given_command_d_on_macos_when_checked_then_it_is_the_shortcut() {
        let key = Key::Character("d".into());

        assert!(is_split_pane_shortcut(&key, &Modifiers::COMMAND, false));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn given_command_d_off_macos_when_checked_then_it_is_not_the_shortcut() {
        // Off macOS `Modifiers::COMMAND` is `CTRL`, and the terminal
        // binds Ctrl+D to EOF. Claiming it here would both send the
        // control character and split the pane.
        let key = Key::Character("d".into());

        assert!(!is_split_pane_shortcut(&key, &Modifiers::COMMAND, false));
    }

    #[test]
    fn given_repeated_command_d_when_checked_then_it_is_not_the_shortcut() {
        let key = Key::Character("d".into());

        assert!(!is_split_pane_shortcut(&key, &Modifiers::COMMAND, true));
    }

    #[test]
    fn given_command_shift_d_when_checked_then_it_is_not_the_shortcut() {
        let key = Key::Character("D".into());
        let modifiers = Modifiers::COMMAND | Modifiers::SHIFT;

        assert!(!is_split_pane_shortcut(&key, &modifiers, false));
    }

    #[test]
    fn given_bare_d_when_checked_then_it_is_not_the_shortcut() {
        let key = Key::Character("d".into());

        assert!(!is_split_pane_shortcut(&key, &Modifiers::empty(), false));
    }

    #[test]
    fn given_command_c_when_checked_then_it_is_not_the_shortcut() {
        let key = Key::Character("c".into());

        assert!(!is_split_pane_shortcut(&key, &Modifiers::COMMAND, false));
    }

    #[test]
    fn given_named_key_when_checked_then_it_is_not_the_shortcut() {
        let key = Key::Named(Named::Enter);

        assert!(!is_split_pane_shortcut(&key, &Modifiers::COMMAND, false));
    }
}
