/// Return whether `value` is a complete 7-char hex color (`#RRGGBB`).
pub(super) fn is_valid_hex_color(value: &str) -> bool {
    let mut chars = value.chars();
    if chars.next() != Some('#') || value.len() != 7 {
        return false;
    }
    chars.all(|ch| ch.is_ascii_hexdigit())
}

/// Return whether `value` is a valid prefix of a hex color.
pub(super) fn is_hex_color_prefix(value: &str) -> bool {
    let mut chars = value.chars();
    if chars.next() != Some('#') || value.len() > 7 {
        return false;
    }
    chars.all(|ch| ch.is_ascii_hexdigit())
}
