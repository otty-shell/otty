//! Small structural shapes drawn by rows and fields themselves.
//!
//! The caret, the cross, the "⋯" dots, the arrow and the check are also
//! public, so the app draws the same shapes in its own views instead of
//! bundling copies of them. They are drawn in black; an svg style with a
//! color paints them in that color.
//!
//! # Example
//!
//! ```
//! use iced::Color;
//! use iced::widget::{Svg, svg};
//! use otty_ui_kit::components::glyph;
//!
//! let _chevron: Svg<'_> =
//!     svg(glyph::caret(true))
//!         .width(10)
//!         .height(10)
//!         .style(|_, _| svg::Style {
//!             color: Some(Color::WHITE),
//!         });
//! ```

use iced::widget::svg;

/// Chevron pointing right (closed tree node).
const CARET_CLOSED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 2.5 8 6l-3.5 3.5"/></svg>"#;
/// Chevron pointing down (open tree node).
const CARET_OPEN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 4.5 6 8l3.5-3.5"/></svg>"#;
/// Cross used to remove a token or close a row.
const CROSS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7"/></svg>"#;
/// Three dots in a row, the "⋯" button that opens a menu of more actions.
const DOTS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black"><circle cx="5" cy="12" r="1.8"/><circle cx="12" cy="12" r="1.8"/><circle cx="19" cy="12" r="1.8"/></svg>"#;
/// Pin outline: a mode that is not pinned.
const PIN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/></svg>"#;
/// Filled pin: a pinned mode.
const PIN_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black" stroke="black" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/></svg>"#;
/// Arrow pointing up: a table column sorted in ascending order, the button
/// that goes to the start of a command block.
const SORT_UP: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 10V2.5M2.75 5.5 6 2.25 9.25 5.5"/></svg>"#;
/// Arrow pointing down: a table column sorted in descending order.
const SORT_DOWN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 2v7.5M2.75 6.5 6 9.75 9.25 6.5"/></svg>"#;
/// Check: a command that ended with exit code 0.
const CHECK: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>"#;
/// Ring with a gap, the shape the spinner turns.
const RING: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round"><path d="M12 3a9 9 0 1 0 9 9"/></svg>"#;
/// Plus, a ready-made icon for component tests.
#[cfg(test)]
const PLUS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M8 3v10M3 8h10"/></svg>"#;

/// Returns the caret of something open or closed: a chevron pointing down
/// when `is_open`, as an open tree node or an unfolded block, and right
/// otherwise.
pub fn caret(is_open: bool) -> svg::Handle {
    svg::Handle::from_memory(if is_open { CARET_OPEN } else { CARET_CLOSED })
}

/// Returns the cross: remove, close, or a command that failed.
pub fn cross() -> svg::Handle {
    svg::Handle::from_memory(CROSS)
}

/// Returns the check: a command that ended with exit code 0.
///
/// # Example
///
/// ```
/// use iced::widget::{Svg, svg};
/// use otty_ui_kit::components::glyph;
///
/// let _done: Svg<'_> = svg(glyph::check()).width(12).height(12);
/// ```
pub fn check() -> svg::Handle {
    svg::Handle::from_memory(CHECK)
}

/// Returns the ring with a gap that the spinner turns.
pub(super) fn ring() -> svg::Handle {
    svg::Handle::from_memory(RING)
}

/// Returns the plus test icon.
#[cfg(test)]
pub(super) fn plus() -> svg::Handle {
    svg::Handle::from_memory(PLUS)
}

/// Returns the "⋯" dots of a button that opens a menu of more actions.
pub fn dots() -> svg::Handle {
    svg::Handle::from_memory(DOTS)
}

/// Returns the arrow: up when `is_up`, down otherwise. A table column sorted
/// in ascending order shows it up.
pub fn arrow(is_up: bool) -> svg::Handle {
    svg::Handle::from_memory(if is_up { SORT_UP } else { SORT_DOWN })
}

/// Returns the pin, filled when `is_pinned`.
pub(super) fn pin(is_pinned: bool) -> svg::Handle {
    svg::Handle::from_memory(if is_pinned { PIN_FILLED } else { PIN })
}
