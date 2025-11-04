use std::ops::{Index, IndexMut};

use crate::cell::{Cell, CellAttributes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridRow {
    pub cells: Vec<Cell>,
}

impl GridRow {
    pub fn new(width: usize, template: &CellAttributes) -> Self {
        let cells = (0..width).map(|_| Cell::blank(template)).collect();
        Self { cells }
    }

    pub fn resize(&mut self, width: usize, template: &CellAttributes) {
        if width > self.cells.len() {
            self.cells.extend(
                (0..(width - self.cells.len())).map(|_| Cell::blank(template)),
            );
        } else {
            self.cells.truncate(width);
        }
    }

    pub fn clear(&mut self, template: &CellAttributes) {
        for cell in &mut self.cells {
            *cell = Cell::blank(template);
        }
    }
}

impl Index<usize> for GridRow {
    type Output = Cell;

    fn index(&self, index: usize) -> &Self::Output {
        &self.cells[index]
    }
}

impl IndexMut<usize> for GridRow {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cells[index]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    width: usize,
    height: usize,
    rows: Vec<GridRow>,
}

impl Grid {
    pub fn new(width: usize, height: usize, template: &CellAttributes) -> Self {
        let rows = (0..height).map(|_| GridRow::new(width, template)).collect();
        Self {
            width,
            height,
            rows,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn rows(&self) -> &[GridRow] {
        &self.rows
    }

    pub fn rows_mut(&mut self) -> &mut [GridRow] {
        &mut self.rows
    }

    pub fn row(&self, idx: usize) -> &GridRow {
        &self.rows[idx]
    }

    pub fn row_mut(&mut self, idx: usize) -> &mut GridRow {
        &mut self.rows[idx]
    }

    pub fn resize(
        &mut self,
        width: usize,
        height: usize,
        template: &CellAttributes,
    ) {
        if height > self.rows.len() {
            self.rows.extend(
                (0..(height - self.rows.len()))
                    .map(|_| GridRow::new(width, template)),
            );
        } else if height < self.rows.len() {
            self.rows.truncate(height);
        }

        for row in &mut self.rows {
            row.resize(width, template);
        }

        self.width = width;
        self.height = height;
    }

    pub fn clear(&mut self, template: &CellAttributes) {
        for row in &mut self.rows {
            row.clear(template);
        }
    }

    pub fn clear_range(
        &mut self,
        row_idx: usize,
        start_col: usize,
        end_col: usize,
        template: &CellAttributes,
    ) {
        if row_idx >= self.rows.len() || start_col >= self.width {
            return;
        }

        let end_col = end_col.min(self.width.saturating_sub(1));
        for col in start_col..=end_col {
            self.rows[row_idx].cells[col] = Cell::blank(template);
        }
    }

    pub fn insert_blank_cells(
        &mut self,
        row_idx: usize,
        col_idx: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if row_idx >= self.rows.len() || col_idx >= self.width || count == 0 {
            return;
        }

        let row = &mut self.rows[row_idx];
        let max_shift = self.width.saturating_sub(col_idx);
        let count = count.min(max_shift);
        for idx in (0..(self.width - col_idx - count)).rev() {
            let source = col_idx + idx;
            let target = source + count;
            row.cells[target] = row.cells[source].clone();
        }
        for idx in col_idx..(col_idx + count).min(self.width) {
            row.cells[idx] = Cell::blank(template);
        }
    }

    pub fn delete_cells(
        &mut self,
        row_idx: usize,
        col_idx: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if row_idx >= self.rows.len() || col_idx >= self.width || count == 0 {
            return;
        }
        let row = &mut self.rows[row_idx];
        let span = self.width.saturating_sub(col_idx);
        let count = count.min(span);

        for idx in col_idx..self.width {
            let source = idx + count;
            if source < self.width {
                row.cells[idx] = row.cells[source].clone();
            } else {
                row.cells[idx] = Cell::blank(template);
            }
        }
    }

    pub fn scroll_up(
        &mut self,
        top: usize,
        bottom: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if top >= bottom || bottom >= self.rows.len() || count == 0 {
            return;
        }

        let span = bottom.saturating_sub(top) + 1;
        let count = count.min(span);
        for _ in 0..count {
            for row in top..bottom {
                self.rows.swap(row, row + 1);
            }
            self.rows[bottom].clear(template);
        }
    }

    pub fn scroll_down(
        &mut self,
        top: usize,
        bottom: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if top >= bottom || bottom >= self.rows.len() || count == 0 {
            return;
        }

        let span = bottom.saturating_sub(top) + 1;
        let count = count.min(span);
        for _ in 0..count {
            for row in (top + 1..=bottom).rev() {
                self.rows.swap(row, row - 1);
            }
            self.rows[top].clear(template);
        }
    }
}
