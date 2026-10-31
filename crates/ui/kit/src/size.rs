//! Fixed heights and widths so every widget lines up, in logical pixels.

/// `control-sm`: compact icon buttons in headers.
pub const CONTROL_SM: f32 = 24.0;
/// `control-md`: toolbar buttons, small buttons.
pub const CONTROL_MD: f32 = 28.0;
/// `control-lg`: form inputs and form buttons.
pub const CONTROL_LG: f32 = 34.0;
/// `row`: list, tree and session rows; menu items.
pub const ROW: f32 = 30.0;
/// `tab`: top tab strip height.
pub const TAB: f32 = 34.0;
/// `titlebar`: window title bar.
pub const TITLEBAR: f32 = 38.0;
/// `statusbar`: status bar.
pub const STATUSBAR: f32 = 24.0;
/// Diameter of a target dot (`.ot-dot`).
pub const DOT: f32 = 7.0;
/// Height of a session group label (`.ot-group { height: 28px }`).
pub const GROUP: f32 = 28.0;
/// Left padding of a session row under its group label, so its title lines
/// up with the group name: padding, dot and gap (`padding-left: 23px`).
pub const SESSION_INDENT: f32 =
    crate::spacing::SPACE_3 + DOT + crate::spacing::SPACE_3;
/// Width of the chevron half of a split button (`width: 18px`).
pub const SPLIT_CHEVRON: f32 = 18.0;
/// Gap between items of the status bar (`.ot-statusbar { gap: 18px }`).
pub const STATUS_GAP: f32 = 18.0;
/// Height of a button inside the status bar (`.ot-status-btn`).
pub const STATUS_BUTTON: f32 = 20.0;
/// `rail`: widget rail width.
pub const RAIL: f32 = 48.0;
/// `rail-button`: rail button height.
pub const RAIL_BUTTON: f32 = 44.0;
/// `panel`: default left panel width; resizable.
pub const PANEL: f32 = 260.0;
/// Tree indent: extra left offset for each nesting level in a tree, as in
/// VS Code (`workbench.tree.indent`).
pub const TREE_INDENT: f32 = 8.0;
/// Logo height in the title bar.
pub const LOGO: f32 = 20.0;
/// Maximum width of a form page.
pub const FORM: f32 = 720.0;
/// Width of the 1px `line` rule between panes, panels and under the title bar.
pub const LINE: f32 = 1.0;
/// Width of the invisible zone around a split line that the pointer can drag.
pub const SPLIT_GRAB: f32 = 8.0;
/// Width of a modal dialog.
pub const DIALOG: f32 = 420.0;
/// Height of a pane header (`.pane-head { height: 30px }`).
pub const PANE_HEADER: f32 = 30.0;
/// Width of the title bar search button (`.tb-search { width: 480px }`).
pub const TITLE_SEARCH_WIDTH: f32 = 480.0;
/// Height of the title bar search button (`.tb-search { height: 26px }`).
pub const TITLE_SEARCH_HEIGHT: f32 = 26.0;
/// Height of an action tile on the empty card (`.big-action`).
pub const ACTION_TILE: f32 = 44.0;
/// Height of the floating note pill (`height: 30px` in the focus screen).
pub const FLOATING_NOTE: f32 = 30.0;
/// Width of the empty card (`.empty-card { width: 460px }`).
pub const EMPTY_CARD: f32 = 460.0;
/// Width of the logo slot on the empty card (`width: 72px`).
pub const EMPTY_LOGO_WIDTH: f32 = 72.0;
/// Height of the logo slot on the empty card (`height: 52px`).
pub const EMPTY_LOGO_HEIGHT: f32 = 52.0;
