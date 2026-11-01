use iced::Font;
use otty_ui_kit::{fonts, typography};

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
            size: typography::TERM.size(),
        }
    }
}

/// Font configuration for terminal rendering.
#[derive(Debug, Default, Clone)]
pub(crate) struct FontsConfig {
    pub(crate) terminal: TerminalFonts,
}
