use std::{collections::HashMap, sync::Arc};

use cursor_icon::CursorIcon;
use otty_escape::{Charset, CharsetIndex, Rgb};

use crate::{Grid, Surface, cell::CellAttributes};

pub trait SurfaceSnapshotSource {
    fn capture_snapshot(&mut self) -> SurfaceSnapshot;
}

impl SurfaceSnapshotSource for Surface {
    fn capture_snapshot(&mut self) -> SurfaceSnapshot {
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
    pub damage: SurfaceDamage,
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
        damage: SurfaceDamage,
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
            damage,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub attributes: CellAttributes,
    pub charsets: [Charset; 4],
    pub active_charset: CharsetIndex,
}

impl CursorSnapshot {
    pub fn new(
        row: usize,
        col: usize,
        attributes: CellAttributes,
        charsets: [Charset; 4],
        active_charset: CharsetIndex,
    ) -> Self {
        Self {
            row,
            col,
            attributes,
            charsets,
            active_charset,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineDamage {
    pub row: usize,
    pub left: usize,
    pub right: usize,
}

impl LineDamage {
    pub fn undamaged(row: usize, width: usize) -> Self {
        Self {
            row,
            left: width,
            right: 0,
        }
    }

    pub fn reset(&mut self, width: usize) {
        self.left = width;
        self.right = 0;
    }

    pub fn include(&mut self, left: usize, right: usize) {
        self.left = self.left.min(left);
        self.right = self.right.max(right);
    }

    pub fn is_damaged(&self) -> bool {
        self.left <= self.right
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceDamage {
    None,
    Full,
    Partial(Vec<LineDamage>),
}

impl SurfaceDamage {
    pub fn is_none(&self) -> bool {
        matches!(self, SurfaceDamage::None)
    }
}
