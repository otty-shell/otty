//! Tracking of damaged lines and cells within the surface.
//!
//! The [`SurfaceDamageState`] is updated by the [`crate::surface::Surface`]
//! whenever content changes. Callers can then query a snapshot of the damage
//! using [`crate::surface::Surface::damage`] and only repaint affected
//! regions.

use std::{cmp, slice};

use crate::index::Point;

/// Damage bounds for a single line in the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineDamageBounds {
    /// Damaged line number.
    pub line: usize,

    /// Leftmost damaged column.
    pub left: usize,

    /// Rightmost damaged column.
    pub right: usize,
}

impl LineDamageBounds {
    /// Create a new damaged span on a line.
    #[inline]
    pub fn new(line: usize, left: usize, right: usize) -> Self {
        Self { line, left, right }
    }

    /// Create an "undamaged" line covering no cells.
    #[inline]
    pub fn undamaged(line: usize, num_cols: usize) -> Self {
        Self {
            line,
            left: num_cols,
            right: 0,
        }
    }

    /// Reset this line to an undamaged state.
    #[inline]
    pub fn reset(&mut self, num_cols: usize) {
        *self = Self::undamaged(self.line, num_cols);
    }

    /// Expand damage bounds to include the given span.
    #[inline]
    pub fn expand(&mut self, left: usize, right: usize) {
        self.left = cmp::min(self.left, left);
        self.right = cmp::max(self.right, right);
    }

    /// Check whether this line has any damage.
    #[inline]
    pub fn is_damaged(&self) -> bool {
        self.left <= self.right
    }
}

/// Terminal damage information collected since the last
/// [`crate::surface::Surface::reset_damage`] call.
#[derive(Debug)]
pub enum SurfaceDamage<'a> {
    /// The entire terminal is damaged.
    Full,

    /// Iterator over damaged lines in the terminal.
    Partial(SurfaceDamageIterator<'a>),
}

/// Iterator over the terminal's viewport damaged lines.
#[derive(Clone, Debug)]
pub struct SurfaceDamageIterator<'a> {
    line_damage: slice::Iter<'a, LineDamageBounds>,
    display_offset: usize,
}

impl<'a> SurfaceDamageIterator<'a> {
    /// Create a new iterator over damaged lines.
    ///
    /// The `display_offset` is taken into account so that damage outside the
    /// visible viewport is skipped.
    pub fn new(
        line_damage: &'a [LineDamageBounds],
        display_offset: usize,
    ) -> Self {
        let num_lines = line_damage.len();
        // Filter out invisible damage.
        let line_damage =
            &line_damage[..num_lines.saturating_sub(display_offset)];
        Self {
            display_offset,
            line_damage: line_damage.iter(),
        }
    }
}

impl Iterator for SurfaceDamageIterator<'_> {
    type Item = LineDamageBounds;

    fn next(&mut self) -> Option<Self::Item> {
        self.line_damage.find_map(|line| {
            line.is_damaged().then_some(LineDamageBounds::new(
                line.line + self.display_offset,
                line.left,
                line.right,
            ))
        })
    }
}

/// State of the terminal damage.
pub(crate) struct SurfaceDamageState {
    /// Hint whether terminal should be damaged entirely regardless of the actual damage changes.
    pub full: bool,

    /// Information about damage on terminal lines.
    pub lines: Vec<LineDamageBounds>,

    /// Old terminal cursor point.
    pub last_cursor: Point,
}

impl SurfaceDamageState {
    /// Create a new damage state covering the given number of visible lines
    /// and columns.
    pub(crate) fn new(num_cols: usize, num_lines: usize) -> Self {
        let lines = (0..num_lines)
            .map(|line| LineDamageBounds::undamaged(line, num_cols))
            .collect();

        Self {
            full: true,
            lines,
            last_cursor: Default::default(),
        }
    }

    #[inline]
    pub(crate) fn resize(&mut self, num_cols: usize, num_lines: usize) {
        // Reset point, so old cursor won't end up outside of the viewport.
        self.last_cursor = Default::default();
        self.full = true;

        self.lines.clear();
        self.lines.reserve(num_lines);
        for line in 0..num_lines {
            self.lines.push(LineDamageBounds::undamaged(line, num_cols));
        }
    }

    /// Damage point inside of the viewport.
    #[inline]
    pub(crate) fn damage_point(&mut self, point: Point<usize>) {
        self.damage_line(point.line, point.column.0, point.column.0);
    }

    /// Expand `line`'s damage to span at least `left` to `right` column.
    #[inline]
    pub(crate) fn damage_line(
        &mut self,
        line: usize,
        left: usize,
        right: usize,
    ) {
        self.lines[line].expand(left, right);
    }

    /// Reset information about terminal damage.
    pub(crate) fn reset(&mut self, num_cols: usize) {
        self.full = false;
        self.lines.iter_mut().for_each(|line| line.reset(num_cols));
    }
}
