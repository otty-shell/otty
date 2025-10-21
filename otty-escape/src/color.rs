use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Std(StdColor),
    TrueColor(Rgb),
    Indexed(u8),
}

/// Standard colors.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum StdColor {
    Black = 0,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Foreground = 256,
    Background,
    Cursor,
    DimBlack,
    DimRed,
    DimGreen,
    DimYellow,
    DimBlue,
    DimMagenta,
    DimCyan,
    DimWhite,
    BrightForeground,
    DimForeground,
}

impl StdColor {
    pub fn to_bright(self) -> Self {
        match self {
            Self::Foreground => Self::BrightForeground,
            Self::Black => Self::BrightBlack,
            Self::Red => Self::BrightRed,
            Self::Green => Self::BrightGreen,
            Self::Yellow => Self::BrightYellow,
            Self::Blue => Self::BrightBlue,
            Self::Magenta => Self::BrightMagenta,
            Self::Cyan => Self::BrightCyan,
            Self::White => Self::BrightWhite,
            Self::DimForeground => Self::Foreground,
            Self::DimBlack => Self::Black,
            Self::DimRed => Self::Red,
            Self::DimGreen => Self::Green,
            Self::DimYellow => Self::Yellow,
            Self::DimBlue => Self::Blue,
            Self::DimMagenta => Self::Magenta,
            Self::DimCyan => Self::Cyan,
            Self::DimWhite => Self::White,
            val => val,
        }
    }

    pub fn to_dim(self) -> Self {
        match self {
            Self::Black => Self::DimBlack,
            Self::Red => Self::DimRed,
            Self::Green => Self::DimGreen,
            Self::Yellow => Self::DimYellow,
            Self::Blue => Self::DimBlue,
            Self::Magenta => Self::DimMagenta,
            Self::Cyan => Self::DimCyan,
            Self::White => Self::DimWhite,
            Self::Foreground => Self::DimForeground,
            Self::BrightBlack => Self::Black,
            Self::BrightRed => Self::Red,
            Self::BrightGreen => Self::Green,
            Self::BrightYellow => Self::Yellow,
            Self::BrightBlue => Self::Blue,
            Self::BrightMagenta => Self::Magenta,
            Self::BrightCyan => Self::Cyan,
            Self::BrightWhite => Self::White,
            Self::BrightForeground => Self::Foreground,
            val => val,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Display for Rgb {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl FromStr for Rgb {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let chars = if s.starts_with("0x") && s.len() == 8 {
            &s[2..]
        } else if s.starts_with('#') && s.len() == 7 {
            &s[1..]
        } else {
            return Err(());
        };

        let r = u8::from_str_radix(&chars[0..=1], 16).map_err(|_| ())?;
        let g = u8::from_str_radix(&chars[2..=3], 16).map_err(|_| ())?;
        let b = u8::from_str_radix(&chars[4..=5], 16).map_err(|_| ())?;

        Ok(Self { r, g, b })
    }
}

impl Rgb {
    /// [W3C's luminance algorithm implementation]: https://www.w3.org/TR/WCAG20/#relativeluminancedef
    pub(crate) fn relative_luminance(self) -> f32 {
        let to_unit = |x: u8| (x as f32) / 255.0;

        let r_linearised = linearise_channel(to_unit(self.r));
        let g_linearised = linearise_channel(to_unit(self.g));
        let b_linearised = linearise_channel(to_unit(self.b));

        0.2126 * r_linearised + 0.7152 * g_linearised + 0.0722 * b_linearised
    }

    /// [W3C's contrast algorithm implementation]: https://www.w3.org/TR/WCAG20/#contrast-ratiodef
    pub(crate) fn contrast(self, other: Rgb) -> f32 {
        let self_luminance = self.relative_luminance();
        let other_luminance = other.relative_luminance();

        let (darker, lighter) = if self_luminance > other_luminance {
            (other_luminance, self_luminance)
        } else {
            (self_luminance, other_luminance)
        };

        (lighter + 0.05) / (darker + 0.05)
    }
}

/// Convert the r/g/b channel to linear form
#[inline]
fn linearise_channel(channel: f32) -> f32 {
    let channel = channel.clamp(0.0, 1.0);
    if channel <= 0.03928 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Parse colors in XParseColor format.
fn xparse_color(color: &[u8]) -> Option<Rgb> {
    if !color.is_empty() && color[0] == b'#' {
        parse_legacy_color(&color[1..])
    } else if color.len() >= 4 && &color[..4] == b"rgb:" {
        parse_rgb_color(&color[4..])
    } else {
        None
    }
}

/// Parse colors in `#r(rrr)g(ggg)b(bbb)` format.
fn parse_legacy_color(color: &[u8]) -> Option<Rgb> {
    let item_len = color.len() / 3;

    // Truncate/Fill to two byte precision.
    let color_from_slice = |slice: &[u8]| {
        let col =
            usize::from_str_radix(str::from_utf8(slice).ok()?, 16).ok()? << 4;
        Some((col >> (4 * slice.len().saturating_sub(1))) as u8)
    };

    Some(Rgb {
        r: color_from_slice(&color[0..item_len])?,
        g: color_from_slice(&color[item_len..item_len * 2])?,
        b: color_from_slice(&color[item_len * 2..])?,
    })
}

/// Parse colors in `rgb:r(rrr)/g(ggg)/b(bbb)` format.
fn parse_rgb_color(color: &[u8]) -> Option<Rgb> {
    let colors = str::from_utf8(color).ok()?.split('/').collect::<Vec<_>>();

    if colors.len() != 3 {
        return None;
    }

    // Scale values instead of filling with `0`s.
    let scale = |input: &str| {
        if input.len() > 4 {
            None
        } else {
            let max = u32::pow(16, input.len() as u32) - 1;
            let value = u32::from_str_radix(input, 16).ok()?;
            Some((255 * value / max) as u8)
        }
    };

    Some(Rgb {
        r: scale(colors[0])?,
        g: scale(colors[1])?,
        b: scale(colors[2])?,
    })
}
