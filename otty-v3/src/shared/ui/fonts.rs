use iced::Font;
use iced::font::{Family, Weight};

/// Embedded JetBrains Mono Nerd Font for terminal rendering.
pub(crate) const TERM_FONT_JET_BRAINS_BYTES: &[u8] = include_bytes!(
    "../../../../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Bold.ttf"
);

/// UI font configuration.
#[derive(Debug, Clone)]
pub(crate) struct UiFonts {
    pub(crate) font_type: Font,
    pub(crate) size: f32,
}

impl Default for UiFonts {
    fn default() -> Self {
        Self {
            font_type: Font::default(),
            size: 14.0,
        }
    }
}

/// Terminal font configuration.
#[derive(Debug, Clone)]
pub(crate) struct TerminalFonts {
    pub(crate) font_type: Font,
    pub(crate) size: f32,
}

impl Default for TerminalFonts {
    fn default() -> Self {
        Self {
            font_type: Font {
                weight: Weight::Bold,
                family: Family::Name("JetBrainsMono Nerd Font Mono"),
                ..Font::default()
            },
            size: 14.0,
        }
    }
}

/// Combined font configuration for UI and terminal rendering.
#[derive(Debug, Default, Clone)]
pub(crate) struct FontsConfig {
    pub(crate) ui: UiFonts,
    pub(crate) terminal: TerminalFonts,
}
