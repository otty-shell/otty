use iced::Font;
use otty_ui_kit::{fonts, typography};

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
            size: typography::TERM.size(),
        }
    }
}

/// Combined font configuration for UI and terminal rendering.
#[derive(Debug, Default, Clone)]
pub(crate) struct FontsConfig {
    pub(crate) ui: UiFonts,
    pub(crate) terminal: TerminalFonts,
}
