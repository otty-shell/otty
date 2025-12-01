use std::{collections::HashMap, str::FromStr};

use iced::{Color, widget::container};
use otty_libterm::escape::{self, Rgb, StdColor};

use crate::settings::ThemeSettings;

pub(crate) trait TerminalStyle {
    fn container_style(&self) -> container::Style;
}

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
    pub bright_foreground: Option<String>,
    pub dim_foreground: String,
    pub dim_black: String,
    pub dim_red: String,
    pub dim_green: String,
    pub dim_yellow: String,
    pub dim_blue: String,
    pub dim_magenta: String,
    pub dim_cyan: String,
    pub dim_white: String,
    pub block_highlight: String,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            foreground: String::from("#d8d8d8"),
            background: String::from("#181818"),
            black: String::from("#181818"),
            red: String::from("#ac4242"),
            green: String::from("#90a959"),
            yellow: String::from("#f4bf75"),
            blue: String::from("#6a9fb5"),
            magenta: String::from("#aa759f"),
            cyan: String::from("#75b5aa"),
            white: String::from("#d8d8d8"),
            bright_black: String::from("#6b6b6b"),
            bright_red: String::from("#c55555"),
            bright_green: String::from("#aac474"),
            bright_yellow: String::from("#feca88"),
            bright_blue: String::from("#82b8c8"),
            bright_magenta: String::from("#c28cb8"),
            bright_cyan: String::from("#93d3c3"),
            bright_white: String::from("#f8f8f8"),
            bright_foreground: None,
            dim_foreground: String::from("#828482"),
            dim_black: String::from("#0f0f0f"),
            dim_red: String::from("#712b2b"),
            dim_green: String::from("#5f6f3a"),
            dim_yellow: String::from("#a17e4d"),
            dim_blue: String::from("#456877"),
            dim_magenta: String::from("#704d68"),
            dim_cyan: String::from("#4d7770"),
            dim_white: String::from("#8e8e8e"),
            block_highlight: String::from("#ffffff"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    palette: Box<ColorPalette>,
    ansi256_colors: HashMap<u8, Color>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            palette: Box::<ColorPalette>::default(),
            ansi256_colors: build_ansi256_colors(),
        }
    }
}

impl Theme {
    pub fn new(settings: ThemeSettings) -> Self {
        Self {
            palette: settings.color_pallete,
            ansi256_colors: build_ansi256_colors(),
        }
    }

    pub fn block_highlight_color(&self) -> Color {
        parse_hex_color(&self.palette.block_highlight)
    }

    pub fn get_color(&self, c: escape::Color) -> Color {
        match c {
            escape::Color::TrueColor(rgb) => {
                Color::from_rgb8(rgb.r, rgb.g, rgb.b)
            },
            escape::Color::Indexed(index) => {
                if index <= 15 {
                    let color = match index {
                        // Normal terminal colors
                        0 => &self.palette.black,
                        1 => &self.palette.red,
                        2 => &self.palette.green,
                        3 => &self.palette.yellow,
                        4 => &self.palette.blue,
                        5 => &self.palette.magenta,
                        6 => &self.palette.cyan,
                        7 => &self.palette.white,
                        // Bright terminal colors
                        8 => &self.palette.bright_black,
                        9 => &self.palette.bright_red,
                        10 => &self.palette.bright_green,
                        11 => &self.palette.bright_yellow,
                        12 => &self.palette.bright_blue,
                        13 => &self.palette.bright_magenta,
                        14 => &self.palette.bright_cyan,
                        15 => &self.palette.bright_white,
                        _ => &self.palette.background,
                    };

                    return parse_hex_color(color);
                }

                // Other colors
                match self.ansi256_colors.get(&index) {
                    Some(color) => *color,
                    None => Color::from_rgb8(0, 0, 0),
                }
            },
            escape::Color::Std(c) => {
                let color = match c {
                    StdColor::Foreground => &self.palette.foreground,
                    StdColor::Background => &self.palette.background,
                    // Normal terminal colors
                    StdColor::Black => &self.palette.black,
                    StdColor::Red => &self.palette.red,
                    StdColor::Green => &self.palette.green,
                    StdColor::Yellow => &self.palette.yellow,
                    StdColor::Blue => &self.palette.blue,
                    StdColor::Magenta => &self.palette.magenta,
                    StdColor::Cyan => &self.palette.cyan,
                    StdColor::White => &self.palette.white,
                    // Bright terminal colors
                    StdColor::BrightBlack => &self.palette.bright_black,
                    StdColor::BrightRed => &self.palette.bright_red,
                    StdColor::BrightGreen => &self.palette.bright_green,
                    StdColor::BrightYellow => &self.palette.bright_yellow,
                    StdColor::BrightBlue => &self.palette.bright_blue,
                    StdColor::BrightMagenta => &self.palette.bright_magenta,
                    StdColor::BrightCyan => &self.palette.bright_cyan,
                    StdColor::BrightWhite => &self.palette.bright_white,
                    StdColor::BrightForeground => {
                        match &self.palette.bright_foreground {
                            Some(color) => color,
                            None => &self.palette.foreground,
                        }
                    },
                    // Dim terminal colors
                    StdColor::DimForeground => &self.palette.dim_foreground,
                    StdColor::DimBlack => &self.palette.dim_black,
                    StdColor::DimRed => &self.palette.dim_red,
                    StdColor::DimGreen => &self.palette.dim_green,
                    StdColor::DimYellow => &self.palette.dim_yellow,
                    StdColor::DimBlue => &self.palette.dim_blue,
                    StdColor::DimMagenta => &self.palette.dim_magenta,
                    StdColor::DimCyan => &self.palette.dim_cyan,
                    StdColor::DimWhite => &self.palette.dim_white,
                    _ => &self.palette.background,
                };

                parse_hex_color(color)
            },
        }
    }
}

fn build_ansi256_colors() -> HashMap<u8, Color> {
    let mut ansi256_colors = HashMap::new();

    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                // Reserve the first 16 colors for config.
                let index = 16 + r * 36 + g * 6 + b;
                let color = Color::from_rgb8(
                    if r == 0 { 0 } else { r * 40 + 55 },
                    if g == 0 { 0 } else { g * 40 + 55 },
                    if b == 0 { 0 } else { b * 40 + 55 },
                );
                ansi256_colors.insert(index, color);
            }
        }
    }

    let index: u8 = 232;
    for i in 0..24 {
        let value = i * 10 + 8;
        ansi256_colors.insert(index + i, Color::from_rgb8(value, value, value));
    }

    ansi256_colors
}

impl TerminalStyle for Theme {
    fn container_style(&self) -> container::Style {
        container::Style {
            background: Some(
                Rgb::from_str(&self.palette.background)
                    .map(|c| Color::from_rgb8(c.r, c.g, c.b))
                    .unwrap_or_else(|_| {
                        panic!(
                            "invalid background color {}",
                            self.palette.background
                        )
                    })
                    .into(),
            ),
            ..container::Style::default()
        }
    }
}

pub fn parse_hex_color(value: &str) -> Color {
    Rgb::from_str(value)
        .map(|c| Color::from_rgb8(c.r, c.g, c.b))
        .unwrap_or_else(|_| panic!("invalid color {}", value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use otty_libterm::escape;
    use std::collections::HashMap;

    #[test]
    fn get_basic_indexed_colors() {
        let default_theme = Theme::default();
        let basic_indexed_colors_map: HashMap<u8, String> = HashMap::from([
            (0, default_theme.palette.black.clone()),
            (1, default_theme.palette.red.clone()),
            (2, default_theme.palette.green.clone()),
            (3, default_theme.palette.yellow.clone()),
            (4, default_theme.palette.blue.clone()),
            (5, default_theme.palette.magenta.clone()),
            (6, default_theme.palette.cyan.clone()),
            (7, default_theme.palette.white.clone()),
            (8, default_theme.palette.bright_black.clone()),
            (9, default_theme.palette.bright_red.clone()),
            (10, default_theme.palette.bright_green.clone()),
            (11, default_theme.palette.bright_yellow.clone()),
            (12, default_theme.palette.bright_blue.clone()),
            (13, default_theme.palette.bright_magenta.clone()),
            (14, default_theme.palette.bright_cyan.clone()),
            (15, default_theme.palette.bright_white.clone()),
        ]);

        for index in 0..16 {
            let color = default_theme.get_color(escape::Color::Indexed(index));
            let expected_color = basic_indexed_colors_map.get(&index).unwrap();
            assert_eq!(
                color,
                Rgb::from_str(expected_color)
                    .map(|c| Color::from_rgb8(c.r, c.g, c.b))
                    .unwrap()
            )
        }
    }
}
