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
/// A folder in a tree.
pub(crate) const FOLDER: &[u8] = include_bytes!("../../assets/svg/folder.svg");
/// An expanded folder in a tree.
pub(crate) const FOLDER_OPENED: &[u8] =
    include_bytes!("../../assets/svg/folder-opened.svg");
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
/// Product logo shown on the empty state. Drawn in its own colors.
///
/// A PNG, not the SVG: `logo-small.svg` only wraps a large raster image,
/// and resvg shrinks it without averaging pixels, so it looks grainy.
pub(crate) const LOGO: &[u8] =
    include_bytes!("../../assets/logo/logo-small.png");
/// Window icon of the app.
pub(crate) const APP_ICON_DATA: &[u8] =
    include_bytes!("../../assets/logo/logo-small.png");
