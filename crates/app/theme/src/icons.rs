//! The app's icons as file bytes: SVG for buttons and modes, PNG for the logo
//! and the window icon. They are read from `assets/` when the crate is built.
//!
//! # Example
//!
//! ```
//! use otty_app_domain::mode::Mode;
//! use otty_app_theme::icons;
//!
//! assert_eq!(icons::mode_icon(Mode::Containers), icons::MODE_CONTAINERS);
//! ```

use otty_app_domain::mode::Mode;

/// Title bar button that maximizes the window.
pub const WINDOW_FULLSCREEN: &[u8] =
    include_bytes!("../../../../assets/svg/fullscreen.svg");
/// Title bar button that minimizes the window.
pub const WINDOW_TRAY: &[u8] =
    include_bytes!("../../../../assets/svg/tray.svg");
/// Title bar button that closes the window.
pub const WINDOW_CLOSE: &[u8] =
    include_bytes!("../../../../assets/svg/window_close.svg");
/// Plus on the New session and New launch buttons.
pub const ADD_TAB_HEADER: &[u8] =
    include_bytes!("../../../../assets/svg/add-tab.svg");
/// A folder in the Launches tree.
pub const FOLDER_CLOSED: &[u8] =
    include_bytes!("../../../../assets/svg/folder-closed.svg");
/// A file in the Files tree.
pub const FILE: &[u8] = include_bytes!("../../../../assets/svg/file.svg");
/// Run mark on the empty state's New launch action.
pub const PLAY: &[u8] = include_bytes!("../../../../assets/svg/play.svg");
/// Title bar button that opens Settings.
pub const SETTINGS: &[u8] =
    include_bytes!("../../../../assets/svg/sidebar-settings.svg");
/// Title bar button that opens the notifications list: a bell.
pub const BELL: &[u8] = include_bytes!("../../../../assets/svg/bell.svg");
/// Title bar toggle for the left panel.
pub const PANEL_LEFT: &[u8] =
    include_bytes!("../../../../assets/svg/panel-left.svg");
/// Title bar toggle for the right panel.
pub const PANEL_RIGHT: &[u8] =
    include_bytes!("../../../../assets/svg/panel-right.svg");
/// Title bar toggle for focus mode.
pub const FOCUS_MODE: &[u8] =
    include_bytes!("../../../../assets/svg/focus-mode.svg");
/// Magnifier in the title search and the command palette input.
pub const SEARCH: &[u8] = include_bytes!("../../../../assets/svg/search.svg");
/// Terminal mode in the mode switcher and the All modes menu: a window
/// with a prompt.
pub const MODE_TERMINAL: &[u8] =
    include_bytes!("../../../../assets/svg/mode-terminal.svg");
/// Containers mode in the mode switcher and the All modes menu: stacked boxes
/// on a base.
pub const MODE_CONTAINERS: &[u8] =
    include_bytes!("../../../../assets/svg/mode-containers.svg");
/// Containers mode button that loads every list again: two turning arrows.
pub const REFRESH: &[u8] = include_bytes!("../../../../assets/svg/refresh.svg");
/// Loading spinner of Containers mode while the engines are looked for: a
/// ring with a gap, turned by the kit spinner.
pub const SPINNER: &[u8] = include_bytes!("../../../../assets/svg/spinner.svg");
/// Containers create buttons (Run container, Create volume, Create network): a
/// plus.
pub const PLUS: &[u8] = include_bytes!("../../../../assets/svg/plus.svg");
/// Containers Pull image and Pull again: an arrow down into a tray.
pub const PULL: &[u8] = include_bytes!("../../../../assets/svg/pull.svg");
/// A broom for clearing: the Containers Prune button and the Clear all button
/// of the notifications list.
pub const BROOM: &[u8] = include_bytes!("../../../../assets/svg/broom.svg");
/// Containers Start and Run actions: a play triangle.
pub const START: &[u8] = include_bytes!("../../../../assets/svg/start.svg");
/// Containers Stop action: a square.
pub const STOP: &[u8] = include_bytes!("../../../../assets/svg/stop.svg");
/// Containers Restart action: one round arrow.
pub const RESTART: &[u8] = include_bytes!("../../../../assets/svg/restart.svg");
/// Containers Pause action: two bars.
pub const PAUSE: &[u8] = include_bytes!("../../../../assets/svg/pause.svg");
/// Containers Unpause action: a bar and a play triangle.
pub const RESUME: &[u8] = include_bytes!("../../../../assets/svg/resume.svg");
/// Containers Remove action: a trash can.
pub const TRASH: &[u8] = include_bytes!("../../../../assets/svg/trash.svg");
/// Containers Shell action: a prompt.
pub const SHELL: &[u8] = include_bytes!("../../../../assets/svg/shell.svg");
/// Copy to the clipboard: one sheet over another.
pub const COPY: &[u8] = include_bytes!("../../../../assets/svg/copy.svg");
/// Link that opens outside the app: an arrow leaving a square.
pub const EXTERNAL_LINK: &[u8] =
    include_bytes!("../../../../assets/svg/external-link.svg");
/// Error mark of a notice: an exclamation mark in a circle.
pub const NOTICE_ERROR: &[u8] =
    include_bytes!("../../../../assets/svg/notice-error.svg");
/// Product logo shown on the empty state. Drawn in its own colors.
///
/// A PNG, not the SVG: `logo-small.svg` only wraps a large raster image,
/// and resvg shrinks it without averaging pixels, so it looks grainy.
pub const LOGO: &[u8] =
    include_bytes!("../../../../assets/logo/logo-small.png");
/// Window icon of the app: the logo on a square canvas, since window
/// managers draw window icons as squares.
pub const APP_ICON_DATA: &[u8] =
    include_bytes!("../../../../assets/logo/logo-square.png");

/// SVG icon of the mode's switcher tab and menu row.
pub fn mode_icon(mode: Mode) -> &'static [u8] {
    match mode {
        Mode::Containers => MODE_CONTAINERS,
        _ => MODE_TERMINAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_each_mode_when_icon_asked_then_it_is_the_mode_switcher_icon() {
        assert_eq!(mode_icon(Mode::Terminal), MODE_TERMINAL);
        assert_eq!(mode_icon(Mode::Containers), MODE_CONTAINERS);
    }

    #[test]
    fn given_two_modes_when_icons_compared_then_they_differ() {
        assert_ne!(mode_icon(Mode::Terminal), mode_icon(Mode::Containers));
    }
}
