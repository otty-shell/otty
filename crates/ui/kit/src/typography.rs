//! Text style tokens: font size, line height and weight of each text role.
//!
//! Interface styles (`TITLE` to `CAPTION`) use the UI font family; terminal
//! styles (`TERM`, `TERM_SMALL`, `KBD`) use the monospace family.

use iced::font::Weight;

/// Size, line height and weight of one text role, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    size: f32,
    line_height: f32,
    weight: Weight,
}

impl TextStyle {
    /// Font size in logical pixels.
    pub const fn size(&self) -> f32 {
        self.size
    }

    /// Line height in logical pixels.
    pub const fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Font weight.
    pub const fn weight(&self) -> Weight {
        self.weight
    }
}

/// `title` (20/28, 600): form and view titles. One per screen.
pub const TITLE: TextStyle = TextStyle {
    size: 20.0,
    line_height: 28.0,
    weight: Weight::Semibold,
};

/// `heading` (14/20, 600): form sections, panel titles.
pub const HEADING: TextStyle = TextStyle {
    size: 14.0,
    line_height: 20.0,
    weight: Weight::Semibold,
};

/// `body` (13/20, 400): default UI text: rows, buttons, menus.
pub const BODY: TextStyle = TextStyle {
    size: 13.0,
    line_height: 20.0,
    weight: Weight::Normal,
};

/// `body-strong` (13/20, 500): session title in pane headers.
pub const BODY_STRONG: TextStyle = TextStyle {
    size: 13.0,
    line_height: 20.0,
    weight: Weight::Medium,
};

/// `small` (12/16, 400): hints, errors, chips, block meta.
pub const SMALL: TextStyle = TextStyle {
    size: 12.0,
    line_height: 16.0,
    weight: Weight::Normal,
};

/// `caption` (11.5/16, 400): status bar, row state, counters.
pub const CAPTION: TextStyle = TextStyle {
    size: 11.5,
    line_height: 16.0,
    weight: Weight::Normal,
};

/// `term` (13/20, 400): terminal output, commands, input line.
pub const TERM: TextStyle = TextStyle {
    size: 13.0,
    line_height: 20.0,
    weight: Weight::Normal,
};

/// `term-small` (12/18, 400): paths and values inside UI: cwd chips, table
/// cells, diffs.
pub const TERM_SMALL: TextStyle = TextStyle {
    size: 12.0,
    line_height: 18.0,
    weight: Weight::Normal,
};

/// `kbd` (10.5/14, 500): shortcut hints.
pub const KBD: TextStyle = TextStyle {
    size: 10.5,
    line_height: 14.0,
    weight: Weight::Medium,
};
