use std::{collections::HashMap, path::PathBuf};

use iced::Font;
use otty_libterm::{TerminalSize, pty::SSHAuth};

use crate::theme::ColorPalette;

#[cfg(target_os = "windows")]
const DEFAULT_SHELL: &str = "cmd.exe";

#[cfg(unix)]
const DEFAULT_SHELL: &str = "/bin/bash";

#[derive(Default, Clone)]
pub struct Settings {
    pub font: FontSettings,
    pub theme: ThemeSettings,
    pub backend: BackendSettings,
}

#[derive(Default, Debug, Clone)]
pub struct BackendSettings {
    pub session: SessionKind,
    pub size: TerminalSize,
}

impl BackendSettings {
    pub fn with_session(mut self, session: SessionKind) -> Self {
        self.session = session;
        self
    }

    pub fn with_size(mut self, size: TerminalSize) -> Self {
        self.size = size;
        self
    }
}

#[derive(Debug, Clone)]
pub enum SessionKind {
    Local {
        program: String,
        args: Vec<String>,
        envs: HashMap<String, String>,
        working_directory: Option<PathBuf>,
    },
    SSH {
        host: String,
        user: String,
        auth: SSHAuth,
    }
}

impl Default for SessionKind {
    fn default() -> Self {
        Self::Local {
            program: DEFAULT_SHELL.to_string(),
            args: vec![],
            envs: HashMap::new(),
            working_directory: None,
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

impl FontSettings {
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn with_scale_factor(mut self, scale_factor: f32) -> Self {
        self.scale_factor = scale_factor;
        self
    }

    pub fn with_font_type(mut self, font_type: Font) -> Self {
        self.font_type = font_type;
        self
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
