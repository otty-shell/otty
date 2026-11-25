use std::collections::BTreeMap;

use iced::{Point, Size};

/// What Plugins mode reads from outside while it reduces: where the
/// pointer is and how big the main area is, so the menu of a plugin in the
/// left panel opens at the pointer and stays inside the main area, and how
/// many of OTTY's own logins each plugin made, which the Remove dialog
/// names. The binary builds it from the frame and Agents mode before every
/// intent.
///
/// # Example
///
/// ```
/// use std::collections::BTreeMap;
///
/// use iced::{Point, Size};
/// use otty_app_plugins::PluginsCtx;
///
/// let ctx = PluginsCtx {
///     cursor: Point::new(120.0, 300.0),
///     screen_size: Size::new(1200.0, 800.0),
///     own_logins: BTreeMap::from([(String::from("openai"), 2)]),
/// };
///
/// assert_eq!(ctx.cursor.x, 120.0);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PluginsCtx {
    /// The last pointer position over the window content.
    pub cursor: Point,
    /// The size of the main area.
    pub screen_size: Size,
    /// How many of OTTY's own logins each plugin made, by plugin id: they
    /// live in the plugin's folder and go when it is removed (issue 13 of
    /// spec 2026-11-23-accounts-apart-from-profiles).
    pub own_logins: BTreeMap<String, usize>,
}
