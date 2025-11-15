//! Surface‑local color palette management.

use std::ops::{Index, IndexMut};

use crate::escape::{Rgb, StdColor};

/// Number of logical color slots tracked by the surface.
///
/// The layout is compatible with common terminal conventions:
///
/// * `0..16`  – named ANSI colors
/// * `16..232` – 6×6×6 color cube
/// * `233..256` – grayscale ramp
/// * `256` – foreground
/// * `257` – background
/// * `258` – cursor
/// * `259..267` – dim colors
/// * `267` – bright foreground
/// * `268` – dim background
pub const COUNT: usize = 269;

#[derive(Copy, Clone)]
pub struct Colors([Option<Rgb>; COUNT]);

impl Default for Colors {
    fn default() -> Self {
        Self([None; COUNT])
    }
}

impl Index<usize> for Colors {
    type Output = Option<Rgb>;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Colors {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Index<StdColor> for Colors {
    type Output = Option<Rgb>;

    #[inline]
    fn index(&self, index: StdColor) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<StdColor> for Colors {
    #[inline]
    fn index_mut(&mut self, index: StdColor) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}
