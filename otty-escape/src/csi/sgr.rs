use super::color::ColorSpec;

/// The `Intensity` of a cell describes its boldness.  Most terminals
/// implement `Intensity::Bold` by either using a bold font or by simply
/// using an alternative color. Some terminals implement `Intensity::Half`
/// as a dimmer color variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intensity {
    Normal,
    Bold,
    Half,
}

/// Specify just how underlined you want your `Cell` to be
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Underline {
    /// The cell is not underlined
    None,
    /// The cell is underlined with a single line
    Single,
    /// The cell is underlined with two lines
    Double,
    /// Curly underline
    Curly,
    /// Dotted underline
    Dotted,
    /// Dashed underline
    Dashed,
}

impl Default for Underline {
    fn default() -> Self {
        Self::None
    }
}

/// Specify whether you want to slowly or rapidly annoy your users
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blink {
    None,
    Slow,
    Rapid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    BaseLine,
    SuperScript,
    SubScript,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Font {
    Default,
    Alternate(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sgr {
    /// Resets rendition to defaults.  Typically switches off
    /// all other Sgr options, but may have greater or lesser impact.
    Reset,
    /// Set the intensity/bold level
    Intensity(Intensity),
    /// Underline character
    Underline(Underline),
    UnderlineColor(ColorSpec),
    Blink(Blink),
    Italic(bool),
    Inverse(bool),
    Invisible(bool),
    StrikeThrough(bool),
    Font(Font),
    Foreground(ColorSpec),
    Background(ColorSpec),
    Overline(bool),
    VerticalAlign(VerticalAlign),
}
