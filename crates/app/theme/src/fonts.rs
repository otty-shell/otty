//! The font the terminal is drawn with.
//!
//! The font files, the `Font` values and the language of the text come from
//! `otty_ui_kit::fonts`; this module only says which of them the terminal
//! uses.
//!
//! # Example
//!
//! ```
//! use otty_app_theme::fonts::FontsConfig;
//!
//! let config = FontsConfig::default();
//!
//! assert_eq!(config.terminal().font(), otty_ui_kit::fonts::MONO);
//! ```

use iced::Font;
use otty_ui_kit::{fonts, typography};

/// Terminal font configuration.
#[derive(Debug, Clone)]
pub struct TerminalFonts {
    font_type: Font,
    size: f32,
}

impl TerminalFonts {
    /// Font of the terminal text.
    pub fn font(&self) -> Font {
        self.font_type
    }

    /// Size of the terminal text in pixels.
    pub fn size(&self) -> f32 {
        self.size
    }
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
pub struct FontsConfig {
    terminal: TerminalFonts,
}

impl FontsConfig {
    /// Font of the terminal.
    pub fn terminal(&self) -> &TerminalFonts {
        &self.terminal
    }
}

#[cfg(test)]
mod tests {
    use otty_ui_kit::{fonts, typography};

    use super::FontsConfig;

    #[test]
    fn given_default_config_when_read_then_terminal_uses_kit_mono_at_term_size()
    {
        let config = FontsConfig::default();

        assert_eq!(config.terminal().font(), fonts::MONO);
        assert_eq!(config.terminal().size(), typography::TERM.size());
    }
}
