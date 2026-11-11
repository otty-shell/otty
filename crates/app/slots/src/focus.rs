//! Ids of the keyboard focus that belong to no feature.
//!
//! An open menu takes the keyboard from every widget, the focused terminal
//! too: typed keys then reach no shell under the menu, and Esc reaches the
//! frame, which closes the menu. A feature asks for this with an outcome, and
//! the binary moves the focus.

use iced::widget::Id;

/// The id of no widget. Focusing it takes the keyboard focus from every
/// widget.
///
/// # Example
///
/// ```
/// use iced::widget::operation;
/// use otty_app_slots::focus;
///
/// let take_the_keyboard = operation::focus::<()>(focus::NOTHING);
/// ```
pub const NOTHING: Id = Id::new("focus_nothing");
