use gpui::{Hsla, Rgba};
use otty_libterm::escape::{Color, Rgb, StdColor};
use otty_libterm::surface::Colors;

use crate::ConfigError;

/// Validated opaque RGB color used by terminal themes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl TerminalColor {
    /// Return the raw RGB channels.
    pub const fn rgb(&self) -> (u8, u8, u8) {
        (self.red, self.green, self.blue)
    }

    /// Convert to the color type consumed by GPUI paint operations.
    pub fn hsla(&self) -> Hsla {
        Rgba {
            r: f32::from(self.red) / 255.0,
            g: f32::from(self.green) / 255.0,
            b: f32::from(self.blue) / 255.0,
            a: 1.0,
        }
        .into()
    }

    /// Construct a color from RGB channels.
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// Parse a color in strict `#rrggbb` form.
    pub fn from_hex(value: &str) -> Result<Self, ConfigError> {
        let Some(hex) = value.strip_prefix('#').filter(|hex| hex.len() == 6)
        else {
            return Err(ConfigError::InvalidColor(value.to_string()));
        };
        let red = parse_channel(&hex[0..2], value)?;
        let green = parse_channel(&hex[2..4], value)?;
        let blue = parse_channel(&hex[4..6], value)?;

        Ok(Self { red, green, blue })
    }
}

impl From<Rgb> for TerminalColor {
    fn from(value: Rgb) -> Self {
        Self::from_rgb(value.r, value.g, value.b)
    }
}

/// The configurable ANSI colors used by a terminal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColorPalette {
    standard: [TerminalColor; 16],
    foreground: TerminalColor,
    background: TerminalColor,
    cursor: TerminalColor,
    dim: [TerminalColor; 9],
}

impl ColorPalette {
    /// Default foreground.
    pub fn foreground(&self) -> TerminalColor {
        self.foreground
    }

    /// Default background.
    pub fn background(&self) -> TerminalColor {
        self.background
    }

    /// Default cursor color.
    pub fn cursor(&self) -> TerminalColor {
        self.cursor
    }

    /// Resolve a terminal color, honoring dynamic colors from the surface.
    pub fn resolve(&self, color: Color, dynamic: &Colors) -> TerminalColor {
        match color {
            Color::TrueColor(rgb) => rgb.into(),
            Color::Indexed(index) => self.resolve_indexed(index),
            Color::Std(standard) => dynamic[standard]
                .map(TerminalColor::from)
                .unwrap_or_else(|| self.resolve_standard(standard)),
        }
    }

    /// Replace the default terminal foreground.
    pub fn with_foreground(mut self, foreground: TerminalColor) -> Self {
        self.foreground = foreground;
        self
    }

    /// Replace the default terminal background.
    pub fn with_background(mut self, background: TerminalColor) -> Self {
        self.background = background;
        self
    }

    /// Replace the default terminal cursor color.
    pub fn with_cursor(mut self, cursor: TerminalColor) -> Self {
        self.cursor = cursor;
        self
    }

    fn resolve_indexed(&self, index: u8) -> TerminalColor {
        if index < 16 {
            return self.standard[usize::from(index)];
        }
        if index < 232 {
            let value = index - 16;
            let red = cube_channel(value / 36);
            let green = cube_channel((value / 6) % 6);
            let blue = cube_channel(value % 6);
            return TerminalColor::from_rgb(red, green, blue);
        }

        let value = 8 + (index - 232) * 10;
        TerminalColor::from_rgb(value, value, value)
    }

    fn resolve_standard(&self, color: StdColor) -> TerminalColor {
        match color {
            StdColor::Foreground | StdColor::BrightForeground => {
                self.foreground
            },
            StdColor::Background => self.background,
            StdColor::Cursor => self.cursor,
            StdColor::Black => self.standard[0],
            StdColor::Red => self.standard[1],
            StdColor::Green => self.standard[2],
            StdColor::Yellow => self.standard[3],
            StdColor::Blue => self.standard[4],
            StdColor::Magenta => self.standard[5],
            StdColor::Cyan => self.standard[6],
            StdColor::White => self.standard[7],
            StdColor::BrightBlack => self.standard[8],
            StdColor::BrightRed => self.standard[9],
            StdColor::BrightGreen => self.standard[10],
            StdColor::BrightYellow => self.standard[11],
            StdColor::BrightBlue => self.standard[12],
            StdColor::BrightMagenta => self.standard[13],
            StdColor::BrightCyan => self.standard[14],
            StdColor::BrightWhite => self.standard[15],
            StdColor::DimForeground => self.dim[8],
            StdColor::DimBlack => self.dim[0],
            StdColor::DimRed => self.dim[1],
            StdColor::DimGreen => self.dim[2],
            StdColor::DimYellow => self.dim[3],
            StdColor::DimBlue => self.dim[4],
            StdColor::DimMagenta => self.dim[5],
            StdColor::DimCyan => self.dim[6],
            StdColor::DimWhite => self.dim[7],
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            standard: [
                color(0x18, 0x18, 0x18),
                color(0xac, 0x42, 0x42),
                color(0x90, 0xa9, 0x59),
                color(0xf4, 0xbf, 0x75),
                color(0x6a, 0x9f, 0xb5),
                color(0xaa, 0x75, 0x9f),
                color(0x75, 0xb5, 0xaa),
                color(0xd8, 0xd8, 0xd8),
                color(0x6b, 0x6b, 0x6b),
                color(0xc5, 0x55, 0x55),
                color(0xaa, 0xc4, 0x74),
                color(0xfe, 0xca, 0x88),
                color(0x82, 0xb8, 0xc8),
                color(0xc2, 0x8c, 0xb8),
                color(0x93, 0xd3, 0xc3),
                color(0xf8, 0xf8, 0xf8),
            ],
            foreground: color(0xd8, 0xd8, 0xd8),
            background: color(0x18, 0x18, 0x18),
            cursor: color(0xd8, 0xd8, 0xd8),
            dim: [
                color(0x0f, 0x0f, 0x0f),
                color(0x71, 0x2b, 0x2b),
                color(0x5f, 0x6f, 0x3a),
                color(0xa1, 0x7e, 0x4d),
                color(0x45, 0x68, 0x77),
                color(0x70, 0x4d, 0x68),
                color(0x4d, 0x77, 0x70),
                color(0x8e, 0x8e, 0x8e),
                color(0x82, 0x84, 0x82),
            ],
        }
    }
}

/// Complete paint palette for the terminal grid and widget chrome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalTheme {
    palette: ColorPalette,
    selection_background: TerminalColor,
    selection_foreground: TerminalColor,
    border: TerminalColor,
    focused_border: TerminalColor,
    block_highlight: TerminalColor,
}

impl TerminalTheme {
    /// ANSI palette used for terminal cell colors.
    pub fn palette(&self) -> &ColorPalette {
        &self.palette
    }

    /// Background used for selected cells.
    pub fn selection_background(&self) -> TerminalColor {
        self.selection_background
    }

    /// Foreground used for selected cells.
    pub fn selection_foreground(&self) -> TerminalColor {
        self.selection_foreground
    }

    /// Border color for an unfocused terminal.
    pub fn border(&self) -> TerminalColor {
        self.border
    }

    /// Border color for a focused terminal.
    pub fn focused_border(&self) -> TerminalColor {
        self.focused_border
    }

    /// Block selection overlay color.
    pub fn block_highlight(&self) -> TerminalColor {
        self.block_highlight
    }

    /// Create a theme around a validated ANSI palette.
    pub fn new(palette: ColorPalette) -> Self {
        Self {
            palette,
            ..Self::default()
        }
    }

    /// Replace selection background and foreground colors.
    pub fn with_selection(
        mut self,
        background: TerminalColor,
        foreground: TerminalColor,
    ) -> Self {
        self.selection_background = background;
        self.selection_foreground = foreground;
        self
    }

    /// Replace unfocused and focused border colors.
    pub fn with_border(
        mut self,
        border: TerminalColor,
        focused_border: TerminalColor,
    ) -> Self {
        self.border = border;
        self.focused_border = focused_border;
        self
    }

    /// Replace the selected-block overlay color.
    pub fn with_block_highlight(mut self, color: TerminalColor) -> Self {
        self.block_highlight = color;
        self
    }
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self {
            palette: ColorPalette::default(),
            selection_background: color(0x36, 0x4a, 0x5c),
            selection_foreground: color(0xff, 0xff, 0xff),
            border: color(0x3a, 0x3a, 0x3a),
            focused_border: color(0x6a, 0x9f, 0xb5),
            block_highlight: color(0xff, 0xff, 0xff),
        }
    }
}

const fn color(red: u8, green: u8, blue: u8) -> TerminalColor {
    TerminalColor::from_rgb(red, green, blue)
}

fn parse_channel(value: &str, original: &str) -> Result<u8, ConfigError> {
    u8::from_str_radix(value, 16)
        .map_err(|_| ConfigError::InvalidColor(original.to_string()))
}

fn cube_channel(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
}
