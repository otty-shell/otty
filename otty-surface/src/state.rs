use std::collections::HashMap;

use otty_escape::Rgb;

use crate::cell::CellAttributes;

#[derive(Debug, Clone)]
pub struct CursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub attributes: CellAttributes,
}

impl CursorSnapshot {
    pub fn new(row: usize, col: usize, attributes: CellAttributes) -> Self {
        Self {
            row,
            col,
            attributes,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SurfacePalette {
    overrides: HashMap<usize, Rgb>,
}

impl SurfacePalette {
    pub fn set(&mut self, index: usize, color: Rgb) {
        self.overrides.insert(index, color);
    }

    pub fn reset(&mut self, index: usize) {
        self.overrides.remove(&index);
    }

    pub fn get(&self, index: usize) -> Option<Rgb> {
        self.overrides.get(&index).copied()
    }
}
