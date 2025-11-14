use egui::Color32;
use otty_libterm::escape::{Color, Rgb, StdColor};
use std::collections::HashMap;

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
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalTheme {
    palette: Box<ColorPalette>,
    ansi256_colors: HashMap<u8, Color32>,
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self {
            palette: Box::<ColorPalette>::default(),
            ansi256_colors: build_256_palette(),
        }
    }
}

impl TerminalTheme {
    pub fn new(palette: Box<ColorPalette>) -> Self {
        Self {
            palette,
            ansi256_colors: build_256_palette(),
        }
    }

    pub fn resolve(&self, color: Color) -> Color32 {
        match color {
            Color::TrueColor(Rgb { r, g, b }) => Color32::from_rgb(r, g, b),
            Color::Indexed(idx) => self
                .ansi256_colors
                .get(&idx)
                .copied()
                .unwrap_or(Color32::from_rgb(0, 0, 0)),
            Color::Std(std) => match std {
                StdColor::Foreground => hex_to_color(&self.palette.foreground),
                StdColor::Background => hex_to_color(&self.palette.background),
                StdColor::Black => hex_to_color(&self.palette.black),
                StdColor::Red => hex_to_color(&self.palette.red),
                StdColor::Green => hex_to_color(&self.palette.green),
                StdColor::Yellow => hex_to_color(&self.palette.yellow),
                StdColor::Blue => hex_to_color(&self.palette.blue),
                StdColor::Magenta => hex_to_color(&self.palette.magenta),
                StdColor::Cyan => hex_to_color(&self.palette.cyan),
                StdColor::White => hex_to_color(&self.palette.white),
                StdColor::BrightBlack => {
                    hex_to_color(&self.palette.bright_black)
                },
                StdColor::BrightRed => hex_to_color(&self.palette.bright_red),
                StdColor::BrightGreen => {
                    hex_to_color(&self.palette.bright_green)
                },
                StdColor::BrightYellow => {
                    hex_to_color(&self.palette.bright_yellow)
                },
                StdColor::BrightBlue => hex_to_color(&self.palette.bright_blue),
                StdColor::BrightMagenta => {
                    hex_to_color(&self.palette.bright_magenta)
                },
                StdColor::BrightCyan => hex_to_color(&self.palette.bright_cyan),
                StdColor::BrightWhite => {
                    hex_to_color(&self.palette.bright_white)
                },
                StdColor::Cursor => hex_to_color(&self.palette.foreground),
                StdColor::DimBlack => dim(hex_to_color(&self.palette.black)),
                StdColor::DimRed => dim(hex_to_color(&self.palette.red)),
                StdColor::DimGreen => dim(hex_to_color(&self.palette.green)),
                StdColor::DimYellow => dim(hex_to_color(&self.palette.yellow)),
                StdColor::DimBlue => dim(hex_to_color(&self.palette.blue)),
                StdColor::DimMagenta => {
                    dim(hex_to_color(&self.palette.magenta))
                },
                StdColor::DimCyan => dim(hex_to_color(&self.palette.cyan)),
                StdColor::DimWhite => dim(hex_to_color(&self.palette.white)),
                StdColor::BrightForeground => {
                    hex_to_color(&self.palette.foreground)
                },
                StdColor::DimForeground => {
                    dim(hex_to_color(&self.palette.foreground))
                },
            },
        }
    }
}

fn build_256_palette() -> HashMap<u8, Color32> {
    let mut m = HashMap::new();

    // 6x6x6 cube at 16..231
    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                let idx = 16 + r * 36 + g * 6 + b;
                let r = if r == 0 { 0 } else { r * 40 + 55 };
                let g = if g == 0 { 0 } else { g * 40 + 55 };
                let b = if b == 0 { 0 } else { b * 40 + 55 };
                m.insert(idx, Color32::from_rgb(r, g, b));
            }
        }
    }

    // grayscale 232..255
    for i in 0..24 {
        let v = 8 + i * 10;
        m.insert(232 + i, Color32::from_rgb(v, v, v));
    }

    m
}

fn hex_to_color(hex: &str) -> Color32 {
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
    Color32::from_rgb(r, g, b)
}

fn dim(c: Color32) -> Color32 {
    let [r, g, b, a] = c.to_array();
    Color32::from_rgba_premultiplied(
        (r as f32 * 0.7) as u8,
        (g as f32 * 0.7) as u8,
        (b as f32 * 0.7) as u8,
        a,
    )
}
