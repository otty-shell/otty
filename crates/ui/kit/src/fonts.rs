//! Bundled fonts of the design system: Noto Sans for the interface and
//! JetBrainsMono Nerd Font Mono for terminal text. Both are licensed under
//! the SIL Open Font License 1.1 (see `assets/fonts`).
//!
//! Load every file from [`all_bytes`] into the `iced` application before
//! using the [`Font`] values below.

use iced::Font;
use iced::font::{Family, Weight};

/// Family name of the interface font as stored in the font file.
const UI_FAMILY: &str = "Noto Sans";

/// Family name of the terminal font as stored in the font file.
const MONO_FAMILY: &str = "JetBrainsMono Nerd Font Mono";

/// Noto Sans Regular (400) font file.
pub const NOTO_SANS_REGULAR_BYTES: &[u8] =
    include_bytes!("../../../../assets/fonts/NotoSans/NotoSans-Regular.ttf");

/// Noto Sans Medium (500) font file.
pub const NOTO_SANS_MEDIUM_BYTES: &[u8] =
    include_bytes!("../../../../assets/fonts/NotoSans/NotoSans-Medium.ttf");

/// Noto Sans SemiBold (600) font file.
pub const NOTO_SANS_SEMIBOLD_BYTES: &[u8] =
    include_bytes!("../../../../assets/fonts/NotoSans/NotoSans-SemiBold.ttf");

/// JetBrainsMono Nerd Font Mono Regular (400) font file.
pub const JETBRAINS_MONO_REGULAR_BYTES: &[u8] = include_bytes!(
    "../../../../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Regular.ttf"
);

/// Interface font, Noto Sans Regular: default text of rows, buttons and menus.
pub const UI: Font = Font {
    family: Family::Name(UI_FAMILY),
    weight: Weight::Normal,
    ..Font::DEFAULT
};

/// Interface font, Noto Sans Medium: session titles in pane headers.
pub const UI_MEDIUM: Font = Font {
    family: Family::Name(UI_FAMILY),
    weight: Weight::Medium,
    ..Font::DEFAULT
};

/// Interface font, Noto Sans SemiBold: titles and headings.
pub const UI_SEMIBOLD: Font = Font {
    family: Family::Name(UI_FAMILY),
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

/// Terminal and code font, JetBrainsMono Nerd Font Mono Regular.
pub const MONO: Font = Font {
    family: Family::Name(MONO_FAMILY),
    weight: Weight::Normal,
    ..Font::DEFAULT
};

/// Returns the bytes of every bundled font file the app must load at startup.
pub fn all_bytes() -> [&'static [u8]; 4] {
    [
        NOTO_SANS_REGULAR_BYTES,
        NOTO_SANS_MEDIUM_BYTES,
        NOTO_SANS_SEMIBOLD_BYTES,
        JETBRAINS_MONO_REGULAR_BYTES,
    ]
}
