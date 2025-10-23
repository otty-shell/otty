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
    pub(crate) fn relative_luminance(self) -> f64 {
        let to_unit = |x: u8| (x as f64) / 255.0;

        let r_linearised = linearise_channel(to_unit(self.r));
        let g_linearised = linearise_channel(to_unit(self.g));
        let b_linearised = linearise_channel(to_unit(self.b));

        0.2126 * r_linearised + 0.7152 * g_linearised + 0.0722 * b_linearised
    }

    /// [W3C's contrast algorithm implementation]: https://www.w3.org/TR/WCAG20/#contrast-ratiodef
    pub(crate) fn contrast(self, other: Rgb) -> f64 {
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
fn linearise_channel(channel: f64) -> f64 {
    let channel = channel.clamp(0.0, 1.0);
    if channel <= 0.03928 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Parse colors in XParseColor format.
/// Supports `#rgb`/`#rrggbb` legacy and `rgb:r/g/b` forms used by xterm.
pub(crate) fn xparse_color(color: &[u8]) -> Option<Rgb> {
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
    let color_len = color.len() / 3;
    if color_len == 0 {
        return None;
    }

    // Normalise each component to two hex digits.
    fn parse_color(slice: &[u8]) -> Option<u8> {
        let hex = str::from_utf8(slice).ok()?;
        let value = usize::from_str_radix(hex, 16).ok()?;
        let normalized = value << 4;
        let shift = 4 * slice.len().saturating_sub(1);
        Some((normalized >> shift) as u8)
    };

    let (r_slice, rest) = color.split_at(color_len);
    let (g_slice, b_slice) = rest.split_at(color_len);

    Some(Rgb {
        r: parse_color(r_slice)?,
        g: parse_color(g_slice)?,
        b: parse_color(b_slice)?,
    })
}

/// Parse colors in `rgb:r(rrr)/g(ggg)/b(bbb)` format.
fn parse_rgb_color(input: &[u8]) -> Option<Rgb> {
    let s = std::str::from_utf8(input).ok()?;
    let colors: Vec<&str> = s.split('/').collect();

    if colors.len() != 3 {
        return None;
    }

    fn scale_hex(hex: &str) -> Option<u8> {
        if hex.is_empty() || hex.len() > 4 {
            return None;
        }

        let value = u32::from_str_radix(hex, 16).ok()?;
        let max = u32::pow(16, hex.len() as u32) - 1;
        Some((255 * value / max) as u8)
    }

    let r = scale_hex(colors[0])?;
    let g = scale_hex(colors[1])?;
    let b = scale_hex(colors[2])?;

    Some(Rgb { r, g, b })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_rgb_colors() {
        assert_eq!(
            xparse_color(b"rgb:f/e/d"),
            Some(Rgb {
                r: 0xFF,
                g: 0xEE,
                b: 0xDD
            })
        );
        assert_eq!(
            xparse_color(b"rgb:11/aa/ff"),
            Some(Rgb {
                r: 0x11,
                g: 0xAA,
                b: 0xFF
            })
        );
        assert_eq!(
            xparse_color(b"rgb:f/ed1/cb23"),
            Some(Rgb {
                r: 0xFF,
                g: 0xEC,
                b: 0xCA
            })
        );
        assert_eq!(
            xparse_color(b"rgb:ffff/0/0"),
            Some(Rgb {
                r: 0xFF,
                g: 0x0,
                b: 0x0
            })
        );
    }

    #[test]
    fn parse_valid_legacy_rgb_colors() {
        assert_eq!(
            xparse_color(b"#1af"),
            Some(Rgb {
                r: 0x10,
                g: 0xA0,
                b: 0xF0
            })
        );
        assert_eq!(
            xparse_color(b"#11aaff"),
            Some(Rgb {
                r: 0x11,
                g: 0xAA,
                b: 0xFF
            })
        );
        assert_eq!(
            xparse_color(b"#110aa0ff0"),
            Some(Rgb {
                r: 0x11,
                g: 0xAA,
                b: 0xFF
            })
        );
        assert_eq!(
            xparse_color(b"#1100aa00ff00"),
            Some(Rgb {
                r: 0x11,
                g: 0xAA,
                b: 0xFF
            })
        );
    }

    #[test]
    fn parse_invalid_rgb_colors() {
        assert_eq!(xparse_color(b"rgb:0//"), None);
        assert_eq!(xparse_color(b"rgb://///"), None);
    }

    #[test]
    fn parse_invalid_legacy_rgb_colors() {
        assert_eq!(xparse_color(b"#"), None);
        assert_eq!(xparse_color(b"#f"), None);
    }

    #[test]
    fn contrast() {
        let rgb1 = Rgb {
            r: 0xFF,
            g: 0xFF,
            b: 0xFF,
        };
        let rgb2 = Rgb {
            r: 0x00,
            g: 0x00,
            b: 0x00,
        };
        assert!((rgb1.contrast(rgb2) - 21.).abs() < f64::EPSILON);

        let rgb1 = Rgb {
            r: 0xFF,
            g: 0xFF,
            b: 0xFF,
        };
        assert!((rgb1.contrast(rgb1) - 1.).abs() < f64::EPSILON);

        let rgb1 = Rgb {
            r: 0xFF,
            g: 0x00,
            b: 0xFF,
        };
        let rgb2 = Rgb {
            r: 0x00,
            g: 0xFF,
            b: 0x00,
        };
        assert!(
            (rgb1.contrast(rgb2) - 2.285_543_608_124_253_3).abs()
                < f64::EPSILON
        );

        let rgb1 = Rgb {
            r: 0x12,
            g: 0x34,
            b: 0x56,
        };
        let rgb2 = Rgb {
            r: 0xFE,
            g: 0xDC,
            b: 0xBA,
        };
        assert!(
            (rgb1.contrast(rgb2) - 9.786_558_997_257_74).abs() < f64::EPSILON
        );
    }
}
