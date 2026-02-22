pub(crate) use super::model::is_valid_hex_color;

/// Return the human-readable label for a palette entry by index.
pub(crate) fn palette_label(index: usize) -> Option<&'static str> {
    PALETTE_LABELS.get(index).copied()
}

const PALETTE_LABELS: [&str; 29] = [
    "Foreground",
    "Background",
    "Black",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "White",
    "Bright Black",
    "Bright Red",
    "Bright Green",
    "Bright Yellow",
    "Bright Blue",
    "Bright Magenta",
    "Bright Cyan",
    "Bright White",
    "Bright Foreground",
    "Dim Black",
    "Dim Red",
    "Dim Green",
    "Dim Yellow",
    "Dim Blue",
    "Dim Magenta",
    "Dim Cyan",
    "Dim White",
    "Dim Foreground",
    "Overlay",
];
