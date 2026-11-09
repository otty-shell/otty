//! The event type of the whole app, part of the App layer. `app.rs` wraps
//! each [`AppEvent`] in a `Message` that says where it came from, and
//! `dispatch` delivers it to the one that handles it.

use iced::window::Direction;

use crate::commands::AppCommand;
use crate::shell_session::ShellSession;
use crate::theme::BuiltinTheme;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::containers::event::ContainersEvent;
use crate::widgets::explorer::event::ExplorerEvent;
use crate::widgets::modes::event::ModesEvent;
use crate::widgets::notifications::event::NotificationsEvent;
use crate::widgets::palette::event::PaletteEvent;
use crate::widgets::quick_launch::event::QuickLaunchEvent;
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::sidebar::event::SidebarEvent;
use crate::widgets::tabs::event::TabsEvent;
use crate::widgets::terminal_workspace::event::TerminalWorkspaceEvent;

/// An event the app loop carries: a widget event, a command or an event of
/// the window itself.
#[derive(Clone)]
pub(crate) enum AppEvent {
    Sidebar(SidebarEvent),
    Chrome(ChromeEvent),
    Tabs(TabsEvent),
    QuickLaunch(QuickLaunchEvent),
    TerminalWorkspace(TerminalWorkspaceEvent),
    Explorer(ExplorerEvent),
    Settings(SettingsEvent),
    Palette(PaletteEvent),
    Modes(ModesEvent),
    Containers(ContainersEvent),
    Notifications(NotificationsEvent),
    /// Run an app command, from a shortcut, a palette entry or a header
    /// button.
    Command(AppCommand),
    /// The window is ready; the startup effects run.
    IcedReady,
    /// The effects built the session new terminal tabs start, after they
    /// wrote the shell integration files. It has no integration when the
    /// files could not be written or the shell has none.
    ShellIntegrated(ShellSession),
    /// The effects read this computer's host name; `None` when it could not
    /// be read.
    HostNameRead(Option<String>),
    /// Saved settings chose this theme for the window and new terminals.
    ThemeChosen(BuiltinTheme),
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}
