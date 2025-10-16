use crate::color::Color;

/// Text attributes for a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Attrs {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub fg: Color,
    pub bg: Color,
}

/// A single grid cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    /// 0-based column index within the grid.
    pub column: usize,
    /// 0-based line (row) index within the grid.
    pub line: usize,
    /// A single character stored in this cell.
    pub ch: char,
    /// Rendering attributes associated with the cell.
    pub attr: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            column: 0,
            line: 0,
            ch: '\0',
            attr: Attrs::default(),
        }
    }
}
