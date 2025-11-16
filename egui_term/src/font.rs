use egui::{Context, FontId};

use crate::types::Size;

#[derive(Debug, Clone)]
pub struct FontSettings {
    pub font_type: FontId,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            font_type: FontId::monospace(14.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalFont {
    font_type: FontId,
}

impl Default for TerminalFont {
    fn default() -> Self {
        Self {
            font_type: FontSettings::default().font_type,
        }
    }
}

impl TerminalFont {
    pub fn new(settings: FontSettings) -> Self {
        Self {
            font_type: settings.font_type,
        }
    }

    pub fn font_type(&self) -> FontId {
        self.font_type.clone()
    }

    pub fn font_measure(&self, ctx: &Context) -> Size {
        let (width, height) = ctx.fonts_mut(|f| {
            (
                f.glyph_width(&self.font_type, 'm'),
                f.row_height(&self.font_type),
            )
        });

        Size::new(width, height)
    }
}
