use iced::Font;
use otty_ui_kit::{fonts, typography};

/// Embedded JetBrains Mono Nerd Font Bold, used for bold terminal text.
pub(crate) const TERM_FONT_JET_BRAINS_BYTES: &[u8] = include_bytes!(
    "../../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Bold.ttf"
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
            font_type: fonts::UI,
            size: typography::BODY.size(),
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
            font_type: fonts::MONO,
            size: typography::BODY.size(),
        }
    }
}

/// Combined font configuration for UI and terminal rendering.
#[derive(Debug, Default, Clone)]
pub(crate) struct FontsConfig {
    pub(crate) ui: UiFonts,
    pub(crate) terminal: TerminalFonts,
}
