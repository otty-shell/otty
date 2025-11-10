use std::{collections::HashMap, sync::Arc};

use cursor_icon::CursorIcon;
use otty_escape::Rgb;

use crate::{Grid, Surface, cell::CellAttributes};

pub trait SurfaceSnapshotSource {
    fn capture_snapshot(&self) -> SurfaceSnapshot;
}

impl SurfaceSnapshotSource for Surface {
    fn capture_snapshot(&self) -> SurfaceSnapshot {
        self.snapshot()
    }
}

#[derive(Debug, Clone)]
pub struct SurfaceSnapshot {
    pub grid: Arc<Grid>,
    pub columns: usize,
    pub rows: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub display_offset: usize,
    pub cursor_icon: Option<CursorIcon>,
    pub cursor_shape: Option<otty_escape::CursorShape>,
    pub cursor_style: Option<otty_escape::CursorStyle>,
}

impl SurfaceSnapshot {
    pub fn new(
        grid: Arc<Grid>,
        columns: usize,
        rows: usize,
        cursor_row: usize,
        cursor_col: usize,
        display_offset: usize,
        cursor_icon: Option<CursorIcon>,
        cursor_shape: Option<otty_escape::CursorShape>,
        cursor_style: Option<otty_escape::CursorStyle>,
    ) -> Self {
        Self {
            grid,
            columns,
            rows,
            cursor_row,
            cursor_col,
            display_offset,
            cursor_icon,
            cursor_shape,
            cursor_style,
        }
    }
}

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
