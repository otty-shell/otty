use iced::theme::Palette;
use iced::{Color, Theme};
use otty_ui_term::{ColorPalette as TerminalColorPalette, parse_hex_color};

#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub foreground: String,
    pub background: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
    pub bright_foreground: String,
    pub dim_black: String,
    pub dim_red: String,
    pub dim_green: String,
    pub dim_yellow: String,
    pub dim_blue: String,
    pub dim_magenta: String,
    pub dim_cyan: String,
    pub dim_white: String,
    pub dim_foreground: String,
    pub overlay: String,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            foreground: String::from("#C0C5CE"),
            background: String::from("#161822"),
            black: String::from("#161822"),
            red: String::from("#E06C75"),
            green: String::from("#98C379"),
            yellow: String::from("#E5C07B"),
            blue: String::from("#4FA6ED"),
            magenta: String::from("#C678DD"),
            cyan: String::from("#56B6C2"),
            white: String::from("#D1D5DB"),
            // BRIGHT COLORS
            bright_black: String::from("#4F5666"),
            bright_red: String::from("#FF5C8D"),
            bright_green: String::from("#5AF78E"),
            bright_yellow: String::from("#F3E488"),
            bright_blue: String::from("#5FD8FF"),
            bright_magenta: String::from("#FF4081"),
            bright_cyan: String::from("#2CD4C8"),
            bright_white: String::from("#FFFFFF"),
            bright_foreground: String::from("#ECEFF4"),
            // DIM COLORS
            dim_foreground: String::from("#6B7280"),
            dim_black: String::from("#0F1115"),
            dim_red: String::from("#8F3F4A"),
            dim_green: String::from("#587545"),
            dim_yellow: String::from("#8A734A"),
            dim_blue: String::from("#2F638F"),
            dim_magenta: String::from("#784885"),
            dim_cyan: String::from("#326B73"),
            dim_white: String::from("#6C7385"),
            overlay: String::from("#232530"),
        }
    }
}

impl From<ColorPalette> for TerminalColorPalette {
    fn from(p: ColorPalette) -> Self {
        Self {
            foreground: p.foreground,
            background: p.background,
            black: p.black,
            red: p.red,
            green: p.green,
            yellow: p.yellow,
            blue: p.blue,
            magenta: p.magenta,
            cyan: p.cyan,
            white: p.white,
            bright_black: p.bright_black,
            bright_red: p.bright_red,
            bright_green: p.bright_green,
            bright_yellow: p.bright_yellow,
            bright_blue: p.bright_blue,
            bright_magenta: p.bright_magenta,
            bright_cyan: p.bright_cyan,
            bright_white: p.bright_white,
            bright_foreground: Some(p.bright_foreground.clone()),
            dim_foreground: p.dim_foreground,
            dim_black: p.dim_black,
            dim_red: p.dim_red,
            dim_green: p.dim_green,
            dim_yellow: p.dim_yellow,
            dim_blue: p.dim_blue,
            dim_magenta: p.dim_magenta,
            dim_cyan: p.dim_cyan,
            dim_white: p.dim_white,
            block_highlight: p.bright_foreground,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IcedColorPalette {
    pub foreground: Color,
    pub background: Color,
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub bright_black: Color,
    pub bright_red: Color,
    pub bright_green: Color,
    pub bright_yellow: Color,
    pub bright_blue: Color,
    pub bright_magenta: Color,
    pub bright_cyan: Color,
    pub bright_white: Color,
    pub bright_foreground: Color,
    pub dim_black: Color,
    pub dim_red: Color,
    pub dim_green: Color,
    pub dim_yellow: Color,
    pub dim_blue: Color,
    pub dim_magenta: Color,
    pub dim_cyan: Color,
    pub dim_white: Color,
    pub dim_foreground: Color,
    pub overlay: Color,
}

impl From<&ColorPalette> for IcedColorPalette {
    fn from(p: &ColorPalette) -> Self {
        Self {
            foreground: parse_hex_color(&p.foreground),
            background: parse_hex_color(&p.background),
            black: parse_hex_color(&p.black),
            red: parse_hex_color(&p.red),
            green: parse_hex_color(&p.green),
            yellow: parse_hex_color(&p.yellow),
            blue: parse_hex_color(&p.blue),
            magenta: parse_hex_color(&p.magenta),
            cyan: parse_hex_color(&p.cyan),
            white: parse_hex_color(&p.white),
            bright_black: parse_hex_color(&p.bright_black),
            bright_red: parse_hex_color(&p.bright_red),
            bright_green: parse_hex_color(&p.bright_green),
            bright_yellow: parse_hex_color(&p.bright_yellow),
            bright_blue: parse_hex_color(&p.bright_blue),
            bright_magenta: parse_hex_color(&p.bright_magenta),
            bright_cyan: parse_hex_color(&p.bright_cyan),
            bright_white: parse_hex_color(&p.bright_white),
            bright_foreground: parse_hex_color(&p.bright_foreground),
            dim_black: parse_hex_color(&p.dim_black),
            dim_red: parse_hex_color(&p.dim_red),
            dim_green: parse_hex_color(&p.dim_green),
            dim_yellow: parse_hex_color(&p.dim_yellow),
            dim_blue: parse_hex_color(&p.dim_blue),
            dim_magenta: parse_hex_color(&p.dim_magenta),
            dim_cyan: parse_hex_color(&p.dim_cyan),
            dim_white: parse_hex_color(&p.dim_white),
            dim_foreground: parse_hex_color(&p.dim_foreground),
            overlay: parse_hex_color(&p.overlay),
        }
    }
}

/// Optional overrides for widget/component styling.
#[derive(Debug, Default, Clone, Copy)]
pub struct StyleOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub border_radius: Option<f32>,
}

/// Global application theme shared between UI and terminal.
#[derive(Debug, Clone)]
pub struct AppTheme {
    id: String,
    raw_palette: ColorPalette,
    iced_palette: IcedColorPalette,
}

impl Default for AppTheme {
    fn default() -> Self {
        let raw_palette = ColorPalette::default();
        let iced_palette = IcedColorPalette::from(&raw_palette);

        Self {
            id: String::from("default"),
            raw_palette,
            iced_palette,
        }
    }
}

impl From<&AppTheme> for Theme {
    fn from(value: &AppTheme) -> Self {
        let palette = &value.iced_palette;
        let palette = Palette {
            background: palette.background,
            text: palette.foreground,
            primary: palette.background,
            success: palette.green,
            danger: palette.red,
            warning: palette.yellow,
        };

        Theme::custom(value.id.clone(), palette)
    }
}

impl AppTheme {
    /// Build an application theme from a custom palette.
    pub fn from_palette(id: String, raw_palette: ColorPalette) -> Self {
        let iced_palette = IcedColorPalette::from(&raw_palette);
        Self {
            id,
            raw_palette,
            iced_palette,
        }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn terminal_palette(&self) -> TerminalColorPalette {
        TerminalColorPalette::from(self.raw_palette.clone())
    }

    pub fn iced_palette(&self) -> &IcedColorPalette {
        &self.iced_palette
    }
}

/// Theme props passed through App -> Screen -> Widget -> Component.
#[derive(Debug, Clone, Copy)]
pub struct ThemeProps<'a> {
    pub theme: &'a AppTheme,
    pub overrides: Option<StyleOverrides>,
}

impl<'a> ThemeProps<'a> {
    pub fn new(theme: &'a AppTheme) -> Self {
        Self {
            theme,
            overrides: None,
        }
    }
}

/// Manages the current global theme and presets.
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current: AppTheme,
}

impl ThemeManager {
    pub fn new() -> Self {
        let default = AppTheme::default();

        Self {
            current: default.clone(),
        }
    }

    pub fn current(&self) -> &AppTheme {
        &self.current
    }

    pub fn iced_theme(&self) -> Theme {
        Theme::from(&self.current)
    }

    /// Replace the current theme with a custom palette.
    pub fn set_custom_palette(&mut self, palette: ColorPalette) {
        self.current = AppTheme::from_palette(String::from("custom"), palette);
    }
}
