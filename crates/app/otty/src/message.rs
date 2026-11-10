//! The only message that goes through iced, and the events it carries.
//!
//! A frame widget or a feature gets its own variant with its `Intent`. What
//! its `reduce` returns never comes back here: `dispatch/` reads the outcomes
//! in the same update. Only input, window events, ticks and I/O answers
//! arrive as a [`Message`].

use iced::window::Direction;
use otty_app_domain::commands::AppCommand;
use otty_app_explorer::ExplorerIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_settings::SettingsIntent;

use crate::chrome::ChromeIntent;
use crate::modes::ModesIntent;
use crate::shell_session::ShellSession;
use crate::sidebar::SidebarIntent;

/// What iced delivers to the app: an event and where it came from.
///
/// Only `app.rs` and `effects/` mark events. The view gives
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
    Chrome(ChromeIntent),
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
    /// Run an app command, from a shortcut or a header button.
    Command(AppCommand),
    /// The session new terminal tabs start, built after the shell
    /// integration files were written. It has no integration when the files
    /// could not be written or the shell has none.
    ShellIntegrated(ShellSession),
    /// This computer's host name; `None` when it could not be read.
    HostNameRead(Option<String>),
    /// A key press or release no widget captured.
    Keyboard(iced::keyboard::Event),
    /// An event of the window, such as a new size.
    #[expect(
        dead_code,
        reason = "the terminal feature (issue 11) resizes its pane grids to a new window size"
    )]
    Window(iced::window::Event),
    /// A grip at the window edge was pressed: resize the window that way.
    ResizeWindow(Direction),
}
