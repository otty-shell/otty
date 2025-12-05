use iced::theme::Palette;
use iced::{Color, Theme};
use otty_ui_term::ColorPalette;

/// Identifier of a built-in application theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppThemeId {
    DefaultDark,
    Ubuntu,
}

/// Global application theme shared between UI and terminal.
#[derive(Debug, Clone)]
pub struct AppTheme {
    pub id: AppThemeId,
    pub name: String,

    pub font_family: String,
    pub font_size_ui: f32,
    pub font_size_terminal: f32,

    pub background: Color,
    pub sidebar_background: Color,
    pub tab_bar_background: Color,
    pub accent: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub border: Color,

    pub terminal_background: Color,
    pub terminal_foreground: Color,
    pub terminal_cursor: Color,
    pub terminal_selection: Color,

    pub terminal_palette: ColorPalette,
}

impl AppTheme {
    pub fn default() -> Self {
        // Atom One Dark inspired palette
        let palette = ColorPalette {
            foreground: String::from("#ABB2BF"),
            background: String::from("#282C34"),
            black: String::from("#282C34"),
            red: String::from("#E06C75"),
            green: String::from("#98C379"),
            yellow: String::from("#E5C07B"),
            blue: String::from("#61AFEF"),
            magenta: String::from("#C678DD"),
            cyan: String::from("#56B6C2"),
            white: String::from("#ABB2BF"),
            bright_black: String::from("#5C6370"),
            bright_red: String::from("#E06C75"),
            bright_green: String::from("#98C379"),
            bright_yellow: String::from("#E5C07B"),
            bright_blue: String::from("#61AFEF"),
            bright_magenta: String::from("#C678DD"),
            bright_cyan: String::from("#56B6C2"),
            bright_white: String::from("#FFFFFF"),
            bright_foreground: None,
            dim_foreground: String::from("#828997"),
            dim_black: String::from("#21252B"),
            dim_red: String::from("#BE5046"),
            dim_green: String::from("#7E8A4E"),
            dim_yellow: String::from("#D19A66"),
            dim_blue: String::from("#4B6EAF"),
            dim_magenta: String::from("#8A4F8D"),
            dim_cyan: String::from("#3A8088"),
            dim_white: String::from("#9DA5B4"),
            block_highlight: String::from("#3E4451"),
        };

        Self::from_palette(
            AppThemeId::DefaultDark,
            String::from("Atom One Dark"),
            palette,
        )
    }

    pub fn ubuntu() -> Self {
        let palette = ColorPalette {
            background: String::from("#300A24"),
            foreground: String::from("#FFFFFF"),
            black: String::from("#2E3436"),
            red: String::from("#CC0000"),
            green: String::from("#4E9A06"),
            yellow: String::from("#C4A000"),
            blue: String::from("#3465A4"),
            magenta: String::from("#75507B"),
            cyan: String::from("#06989A"),
            white: String::from("#D3D7CF"),
            bright_black: String::from("#555753"),
            bright_red: String::from("#EF2929"),
            bright_green: String::from("#8AE234"),
            bright_yellow: String::from("#FCE94F"),
            bright_blue: String::from("#729FCF"),
            bright_magenta: String::from("#AD7FA8"),
            bright_cyan: String::from("#34E2E2"),
            bright_white: String::from("#EEEEEC"),
            ..ColorPalette::default()
        };

        Self::from_palette(AppThemeId::Ubuntu, String::from("Ubuntu"), palette)
    }

    pub fn from_id(id: AppThemeId) -> Self {
        match id {
            AppThemeId::DefaultDark => Self::default(),
            AppThemeId::Ubuntu => Self::ubuntu(),
        }
    }

    pub fn to_iced_theme(&self) -> Theme {
        let palette = Palette {
            background: self.background,
            text: self.text_primary,
            primary: self.accent,
            success: self.accent,
            danger: Color::from_rgb(0.76, 0.26, 0.32),
        };

        Theme::custom(self.name.clone(), palette)
    }

    fn from_palette(
        id: AppThemeId,
        name: String,
        palette: ColorPalette,
    ) -> Self {
        let background = color_from_hex(&palette.background);
        let foreground = color_from_hex(&palette.foreground);
        let dim_foreground = color_from_hex(&palette.dim_foreground);
        let accent = color_from_hex(&palette.blue);
        let border = color_from_hex(&palette.bright_black);
        let selection = color_from_hex(&palette.block_highlight);

        Self {
            id,
            name,
            font_family: String::from("monospace"),
            font_size_ui: 14.0,
            font_size_terminal: 14.0,
            background,
            sidebar_background: background,
            tab_bar_background: background,
            accent,
            text_primary: foreground,
            text_muted: dim_foreground,
            border,
            terminal_background: background,
            terminal_foreground: foreground,
            terminal_cursor: foreground,
            terminal_selection: selection,
            terminal_palette: palette,
        }
    }
}

/// Manages the current global theme and presets.
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current: AppTheme,
    presets: Vec<AppTheme>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let default = AppTheme::default();
        let ubuntu = AppTheme::ubuntu();

        Self {
            current: default.clone(),
            presets: vec![default, ubuntu],
        }
    }

    pub fn current(&self) -> &AppTheme {
        &self.current
    }

    pub fn presets(&self) -> &[AppTheme] {
        &self.presets
    }

    pub fn set_current(&mut self, id: AppThemeId) {
        if self.current.id == id {
            return;
        }

        if let Some(found) = self.presets.iter().find(|theme| theme.id == id) {
            self.current = found.clone();
        } else {
            self.current = AppTheme::from_id(id);
        }
    }

    pub fn set_font_size_ui(&mut self, size: f32) {
        let clamped = size.clamp(10.0, 24.0);
        self.current.font_size_ui = clamped;
    }

    pub fn set_font_size_terminal(&mut self, size: f32) {
        let clamped = size.clamp(8.0, 32.0);
        self.current.font_size_terminal = clamped;
    }

    pub fn iced_theme(&self) -> Theme {
        self.current.to_iced_theme()
    }

    pub fn terminal_palette(&self) -> ColorPalette {
        self.current.terminal_palette.clone()
    }
}

fn color_from_hex(value: &str) -> Color {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);

    if hex.len() != 6 {
        panic!("invalid hex color {}", value);
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .unwrap_or_else(|_| panic!("invalid hex color {}", value));
    let g = u8::from_str_radix(&hex[2..4], 16)
        .unwrap_or_else(|_| panic!("invalid hex color {}", value));
    let b = u8::from_str_radix(&hex[4..6], 16)
        .unwrap_or_else(|_| panic!("invalid hex color {}", value));

    Color::from_rgb8(r, g, b)
}
