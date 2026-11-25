//! The only message that goes through iced, and the events it carries.
//!
//! A frame widget or a feature gets its own variant with its `Intent`. What
//! its `reduce` returns never comes back here: `dispatch/` reads the outcomes
//! in the same update. Only input, window events, ticks and I/O answers
//! arrive as a [`Message`].

#[cfg(not(target_os = "macos"))]
use iced::window::Direction;
use otty_app_agents::AgentsIntent;
use otty_app_agents_io::LoadedReaders;
use otty_app_containers::ContainersIntent;
use otty_app_domain::commands::AppCommand;
use otty_app_explorer::ExplorerIntent;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_plugins::PluginsIntent;
use otty_app_quick_launch::QuickLaunchIntent;
use otty_app_settings::SettingsIntent;
use otty_app_shell::ShellSession;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

use crate::frame::sidebar::SidebarIntent;
use crate::frame::title_bar::TitleBarIntent;

/// What iced delivers to the app: an event and where it came from.
///
/// Only `app.rs` marks events. The view gives
/// [`Message::Input`]; the keyboard subscription too, since a key press is
/// input. Everything else (I/O answers, ticks, window events) comes back as
/// [`Message::Internal`].
#[derive(Debug, Clone)]
pub(crate) enum Message {
    /// A press or typed text in the interface, or a key press. The input
    /// checks in `update.rs` decide what happens to it.
    Input(AppEvent),
    /// An I/O answer, a tick or a window event. It follows from input that
    /// already passed the checks, or arrives on its own, so nothing stops it.
    Internal(AppEvent),
}

/// An event the app loop carries: an intent of a frame widget or of a
/// feature, a command or an event of the window itself.
#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    /// The header: the window buttons and the panel toggles.
    TitleBar(TitleBarIntent),
    /// The side panels, focus mode and the section header menus.
    Sidebar(SidebarIntent),
    /// The mode switcher and the All modes menu.
    Modes(ModesIntent),
    /// The error toasts and the notifications list under the bell.
    Notifications(NotificationsIntent),
    /// The Files section of the right panel in Terminal mode.
    Explorer(ExplorerIntent),
    /// The command palette.
    Palette(PaletteIntent),
    /// The Settings dialog, over every mode.
    Settings(SettingsIntent),
    /// The open tabs and the Sessions section of the left panel.
    Tabs(TabsIntent),
    /// The panes of the terminal tabs, their pane menus and what their
    /// terminals report.
    Terminal(TerminalIntent),
    /// The Launches section, the launch editor, the context menu of the
    /// launches and the SSH secret prompt, and the answers of their I/O.
    QuickLaunch(QuickLaunchIntent),
    /// Containers mode: the engine tree, the lists and their search, and the
    /// answers of the engine work and the engine events.
    Containers(ContainersIntent),
    /// Agents mode: the profiles, the projects and their sessions, and the
    /// answers of their reads.
    Agents(AgentsIntent),
    /// Plugins mode, its dialogs, and the answers of its I/O.
    Plugins(PluginsIntent),
    /// The parts that are on of the installed plugins were loaded: Agents
    /// mode reads with them from now on.
    PluginsLoaded(LoadedReaders),
    /// Run an app command, from a shortcut or a header button.
    Command(AppCommand),
    /// The session new terminal tabs start, built after the shell
    /// integration files were written. It has no integration when the files
    /// could not be written or the shell has none.
    ShellIntegrated(ShellSession),
    /// This computer's host name; `None` when it could not be read. The
    /// tabs name this computer's machine group with it.
    HostNameRead(Option<String>),
    /// A key press or release no widget captured.
    Keyboard(iced::keyboard::Event),
    /// An event of the window, such as a new size.
    Window(iced::window::Event),
    /// A grip at the window edge was pressed: resize the window that way.
    /// The system resizes the window on macOS, so it has no grips.
    #[cfg(not(target_os = "macos"))]
    ResizeWindow(Direction),
    /// An event sent from inside an open menu: one of its items, or a
    /// hover or a scroll on it. The view marks every message of a menu
    /// so, and the input checks let it reach the menu without knowing
    /// its items.
    FromMenu(Box<AppEvent>),
    /// A press of any mouse button that no widget took, such as one on an
    /// empty part of the window: it closes the open menus.
    PressedAway,
}

impl AppEvent {
    /// Marks `event` as sent from inside an open menu.
    pub(crate) fn from_menu(event: AppEvent) -> Self {
        Self::FromMenu(Box::new(event))
    }

    /// The event without the mark of a menu, as its feature gets it.
    #[cfg(test)]
    pub(crate) fn unmarked(self) -> AppEvent {
        match self {
            Self::FromMenu(event) => *event,
            event => event,
        }
    }
}
