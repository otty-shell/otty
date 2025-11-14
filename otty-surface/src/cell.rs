use std::sync::Arc;

use otty_escape::{Color, Hyperlink, Rgb, StdColor};

pub type HyperlinkRef = Arc<Hyperlink>;

/// Visual effects for blinking text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellBlink {
    /// No blinking.
    #[default]
    None,
    /// Slow blink.
    Slow,
    /// Fast blink.
    Fast,
}

/// Supported underline variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellUnderline {
    /// No underline.
    #[default]
    None,
    /// Single underline.
    Single,
    /// Double underline.
    Double,
    /// Curly underline.
    Curl,
    /// Dotted underline.
    Dotted,
    /// Dashed underline.
    Dashed,
}

/// Per-cell visual attributes used when rendering a terminal grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellAttributes {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: CellUnderline,
    pub blink: CellBlink,
    pub reverse: bool,
    pub hidden: bool,
    pub strike: bool,
    pub foreground: Color,
    pub background: Color,
    pub underline_color: Option<Color>,
    pub hyperlink: Option<HyperlinkRef>,
}

impl Default for CellAttributes {
    fn default() -> Self {
        Self {
            bold: false,
            dim: false,
            italic: false,
            underline: CellUnderline::None,
            blink: CellBlink::None,
            reverse: false,
            hidden: false,
            strike: false,
            foreground: Color::Std(StdColor::Foreground),
            background: Color::Std(StdColor::Background),
            underline_color: None,
            hyperlink: None,
        }
    }
}

impl CellAttributes {
    /// Returns attributes with explicit foreground/background colors.
    pub fn with_colors(foreground: Rgb, background: Rgb) -> Self {
        Self {
            foreground: Color::TrueColor(foreground),
            background: Color::TrueColor(background),
            ..Self::default()
        }
    }

    pub fn set_hyperlink(&mut self, hyperlink: Option<Hyperlink>) {
        self.hyperlink = hyperlink.map(Arc::new);
    }
}

/// Represents a single cell in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub attributes: CellAttributes,
    pub zero_width: Vec<char>,
    pub wide_leading: bool,
    pub wide_trailing: bool,
    pub touched: bool,
}

impl Cell {
    pub fn blank(attributes: &CellAttributes) -> Self {
        Self {
            ch: ' ',
            attributes: attributes.clone(),
            zero_width: Vec::new(),
            wide_leading: false,
            wide_trailing: false,
            touched: false,
        }
    }

    pub fn with_char(ch: char, attributes: &CellAttributes) -> Self {
        Self {
            ch,
            attributes: attributes.clone(),
            zero_width: Vec::new(),
            wide_leading: false,
            wide_trailing: false,
            touched: true,
        }
    }

    pub fn is_blank(&self) -> bool {
        self.ch == ' '
    }

    pub fn clear_zero_width(&mut self) {
        self.zero_width.clear();
    }

    pub fn push_zero_width(&mut self, ch: char) {
        self.zero_width.push(ch);
    }

    pub fn is_wide_leading(&self) -> bool {
        self.wide_leading
    }

    pub fn is_wide_trailing(&self) -> bool {
        self.wide_trailing
    }
}
