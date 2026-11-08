//! Keyboard focus that belongs to no widget.
//!
//! An open menu takes the keyboard from every widget, the focused terminal
//! too: typed keys then reach no shell under the menu, and Esc reaches the
//! app, which closes the menu.

use iced::Task;
use iced::widget::{Id, operation};

/// The id of no widget. Focusing it unfocuses every widget.
pub(crate) const NOTHING: Id = Id::new("focus_nothing");

/// Take the keyboard focus from every widget, the focused terminal too.
///
/// # Example
///
/// ```ignore
/// SidebarIntent::HeaderMenuOpen(menu) => {
///     state.open_header_menu(menu);
///     focus::nothing()
/// },
/// ```
pub(crate) fn nothing<T>() -> Task<T> {
    operation::focus(NOTHING)
}
