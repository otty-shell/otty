//! Small structural shapes drawn by rows and fields themselves.

use iced::widget::svg;

/// Chevron pointing right (closed tree node).
const CARET_CLOSED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 2.5 8 6l-3.5 3.5"/></svg>"#;
/// Chevron pointing down (open tree node).
const CARET_OPEN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 4.5 6 8l3.5-3.5"/></svg>"#;
/// Cross used to remove a token or close a row.
const CROSS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7"/></svg>"#;
/// Three dots in a row, the "more" button of the mode switcher.
const DOTS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black"><circle cx="5" cy="12" r="1.8"/><circle cx="12" cy="12" r="1.8"/><circle cx="19" cy="12" r="1.8"/></svg>"#;
/// Pin outline: a mode that is not pinned.
const PIN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/></svg>"#;
/// Filled pin: a pinned mode.
const PIN_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black" stroke="black" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/></svg>"#;
/// Arrow pointing up: a table column sorted in ascending order.
const SORT_UP: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 10V2.5M2.75 5.5 6 2.25 9.25 5.5"/></svg>"#;
/// Arrow pointing down: a table column sorted in descending order.
const SORT_DOWN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 2v7.5M2.75 6.5 6 9.75 9.25 6.5"/></svg>"#;
/// Plus, a ready-made icon for component tests.
#[cfg(test)]
const PLUS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M8 3v10M3 8h10"/></svg>"#;

/// Returns the tree caret for an open or closed node.
pub(super) fn caret(is_open: bool) -> svg::Handle {
    svg::Handle::from_memory(if is_open { CARET_OPEN } else { CARET_CLOSED })
}

/// Returns the remove cross.
pub(super) fn cross() -> svg::Handle {
    svg::Handle::from_memory(CROSS)
}

/// Returns the plus test icon.
#[cfg(test)]
pub(super) fn plus() -> svg::Handle {
    svg::Handle::from_memory(PLUS)
}

/// Returns the "more" dots.
pub(super) fn dots() -> svg::Handle {
    svg::Handle::from_memory(DOTS)
}

/// Returns the sort arrow: up when `is_ascending`, down otherwise.
pub(super) fn sort_arrow(is_ascending: bool) -> svg::Handle {
    svg::Handle::from_memory(if is_ascending { SORT_UP } else { SORT_DOWN })
}

/// Returns the pin, filled when `is_pinned`.
pub(super) fn pin(is_pinned: bool) -> svg::Handle {
    svg::Handle::from_memory(if is_pinned { PIN_FILLED } else { PIN })
}
