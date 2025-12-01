use iced::{
    Font,
    font::{Family, Weight},
};

pub(crate) const TERM_FONT_JET_BRAINS_BYTES: &[u8] = include_bytes!(
    "../../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Bold.ttf"
);

#[derive(Debug, Clone)]
pub struct UiFonts {
    pub _font_type: Font,
    pub size: f32,
}

impl Default for UiFonts {
    fn default() -> Self {
        Self {
            _font_type: Font::default(),
            size: 14.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalFonts {
    pub font_type: Font,
    pub size: f32,
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

#[derive(Debug, Default, Clone)]
pub struct FontsConfig {
    pub ui: UiFonts,
    pub terminal: TerminalFonts,
}
