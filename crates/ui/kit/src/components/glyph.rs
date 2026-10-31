//! Small structural shapes drawn by rows, tabs and fields themselves.

use iced::widget::svg;

/// Chevron pointing right (closed tree node).
const CARET_CLOSED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 2.5 8 6l-3.5 3.5"/></svg>"#;
/// Chevron pointing down (open tree node).
const CARET_OPEN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 4.5 6 8l3.5-3.5"/></svg>"#;
/// Cross used to remove a token, row or tab.
const CROSS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7"/></svg>"#;
/// Plus used to add a tab.
const PLUS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M8 3v10M3 8h10"/></svg>"#;

/// Returns the tree caret for an open or closed node.
pub(super) fn caret(is_open: bool) -> svg::Handle {
    svg::Handle::from_memory(if is_open { CARET_OPEN } else { CARET_CLOSED })
}

/// Returns the remove cross.
pub(super) fn cross() -> svg::Handle {
    svg::Handle::from_memory(CROSS)
}

/// Returns the add plus.
pub(super) fn plus() -> svg::Handle {
    svg::Handle::from_memory(PLUS)
}
