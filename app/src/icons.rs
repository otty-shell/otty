/// Title bar button that maximizes the window.
pub(crate) const WINDOW_FULLSCREEN: &[u8] =
    include_bytes!("../../assets/svg/fullscreen.svg");
/// Title bar button that minimizes the window.
pub(crate) const WINDOW_TRAY: &[u8] =
    include_bytes!("../../assets/svg/tray.svg");
/// Title bar button that closes the window.
pub(crate) const WINDOW_CLOSE: &[u8] =
    include_bytes!("../../assets/svg/window_close.svg");
/// Plus on the New session and New launch buttons.
pub(crate) const ADD_TAB_HEADER: &[u8] =
    include_bytes!("../../assets/svg/add-tab.svg");
/// A folder in the Launches tree.
pub(crate) const FOLDER_CLOSED: &[u8] =
    include_bytes!("../../assets/svg/folder-closed.svg");
/// A file in the Files tree.
pub(crate) const FILE: &[u8] = include_bytes!("../../assets/svg/file.svg");
/// Run mark on the empty state's New launch action.
pub(crate) const PLAY: &[u8] = include_bytes!("../../assets/svg/play.svg");
/// Title bar button that opens Settings.
pub(crate) const SETTINGS: &[u8] =
    include_bytes!("../../assets/svg/sidebar-settings.svg");
/// Title bar button that opens the notifications list: a bell.
pub(crate) const BELL: &[u8] = include_bytes!("../../assets/svg/bell.svg");
/// Title bar toggle for the left panel.
pub(crate) const PANEL_LEFT: &[u8] =
    include_bytes!("../../assets/svg/panel-left.svg");
/// Title bar toggle for the right panel.
pub(crate) const PANEL_RIGHT: &[u8] =
    include_bytes!("../../assets/svg/panel-right.svg");
/// Title bar toggle for focus mode.
pub(crate) const FOCUS_MODE: &[u8] =
    include_bytes!("../../assets/svg/focus-mode.svg");
/// Magnifier in the title search and the command palette input.
pub(crate) const SEARCH: &[u8] = include_bytes!("../../assets/svg/search.svg");
/// Terminal mode in the mode switcher and the All modes menu: a window
/// with a prompt.
pub(crate) const MODE_TERMINAL: &[u8] =
    include_bytes!("../../assets/svg/mode-terminal.svg");
/// Containers mode in the mode switcher and the All modes menu: stacked boxes
/// on a base.
pub(crate) const MODE_CONTAINERS: &[u8] =
    include_bytes!("../../assets/svg/mode-containers.svg");
/// Containers mode button that loads every list again: two turning arrows.
pub(crate) const REFRESH: &[u8] =
    include_bytes!("../../assets/svg/refresh.svg");
/// Containers create buttons (Run container, Create volume, Create network): a
/// plus.
pub(crate) const PLUS: &[u8] = include_bytes!("../../assets/svg/plus.svg");
/// Containers Pull image and Pull again: an arrow down into a tray.
pub(crate) const PULL: &[u8] = include_bytes!("../../assets/svg/pull.svg");
/// Containers Prune button: a broom.
pub(crate) const PRUNE: &[u8] = include_bytes!("../../assets/svg/prune.svg");
/// Containers Start and Run actions: a play triangle.
pub(crate) const START: &[u8] = include_bytes!("../../assets/svg/start.svg");
/// Containers Stop action: a square.
pub(crate) const STOP: &[u8] = include_bytes!("../../assets/svg/stop.svg");
/// Containers Restart action: one round arrow.
pub(crate) const RESTART: &[u8] =
    include_bytes!("../../assets/svg/restart.svg");
/// Containers Pause action: two bars.
pub(crate) const PAUSE: &[u8] = include_bytes!("../../assets/svg/pause.svg");
/// Containers Unpause action: a bar and a play triangle.
pub(crate) const RESUME: &[u8] = include_bytes!("../../assets/svg/resume.svg");
/// Containers Remove action: a trash can.
pub(crate) const TRASH: &[u8] = include_bytes!("../../assets/svg/trash.svg");
/// Containers Shell action: a prompt.
pub(crate) const SHELL: &[u8] = include_bytes!("../../assets/svg/shell.svg");
/// Copy to the clipboard: one sheet over another.
pub(crate) const COPY: &[u8] = include_bytes!("../../assets/svg/copy.svg");
/// Link that opens outside the app: an arrow leaving a square.
pub(crate) const EXTERNAL_LINK: &[u8] =
    include_bytes!("../../assets/svg/external-link.svg");
/// Error mark of a notice: an exclamation mark in a circle.
pub(crate) const NOTICE_ERROR: &[u8] =
    include_bytes!("../../assets/svg/notice-error.svg");
/// Product logo shown on the empty state. Drawn in its own colors.
///
/// A PNG, not the SVG: `logo-small.svg` only wraps a large raster image,
/// and resvg shrinks it without averaging pixels, so it looks grainy.
pub(crate) const LOGO: &[u8] =
    include_bytes!("../../assets/logo/logo-small.png");
/// Window icon of the app.
pub(crate) const APP_ICON_DATA: &[u8] =
    include_bytes!("../../assets/logo/logo-small.png");
