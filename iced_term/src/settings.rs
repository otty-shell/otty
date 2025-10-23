use crate::ColorPalette;
use iced::Font;

#[cfg(target_os = "windows")]
const DEFAULT_SHELL: &str = "wsl.exe";

#[cfg(not(target_os = "windows"))]
const DEFAULT_SHELL: &str = "/bin/bash";

#[derive(Default, Clone)]
pub struct Settings {
    pub font: FontSettings,
    pub theme: ThemeSettings,
    pub backend: BackendSettings,
}

#[derive(Debug, Clone)]
pub struct BackendSettings {
    pub program: String,
    pub args: Vec<String>,
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            program: DEFAULT_SHELL.to_string(),
            args: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontSettings {
    pub size: f32,
    pub scale_factor: f32,
    pub font_type: Font,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            size: 14.0,
            scale_factor: 1.3,
            font_type: Font::MONOSPACE,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ThemeSettings {
    pub color_pallete: Box<ColorPalette>,
}

impl ThemeSettings {
    pub fn new(color_pallete: Box<ColorPalette>) -> Self {
        Self { color_pallete }
    }
}
