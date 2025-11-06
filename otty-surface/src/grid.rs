use std::ops::{Index, IndexMut};

use crate::cell::{Cell, CellAttributes};

/// A single row in the terminal grid.
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

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
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

/// Direction for scrolling through history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Delta(i32),
    PageUp,
    PageDown,
    Top,
    Bottom,
}

/// Ring buffer storage for grid rows with scrollback history.
///
/// Uses a ring buffer to efficiently rotate rows without allocations.
/// The `zero` field tracks the starting position in the ring, allowing
/// O(1) rotation operations.
#[derive(Debug, Clone)]
struct Storage {
    /// The actual row buffer (may contain more rows than currently active).
    inner: Vec<GridRow>,
    /// Starting index for the ring buffer (the bottommost visible line).
    zero: usize,
    /// Number of visible lines in the terminal.
    visible_lines: usize,
    /// Total number of active lines (visible + history).
    len: usize,
}

impl Storage {
    fn new(
        visible_lines: usize,
        columns: usize,
        template: &CellAttributes,
    ) -> Self {
        let mut inner = Vec::with_capacity(visible_lines);
        for _ in 0..visible_lines {
            inner.push(GridRow::new(columns, template));
        }
        Self {
            inner,
            zero: 0,
            visible_lines,
            len: visible_lines,
        }
    }

    /// Get the actual index in the ring buffer.
    #[inline]
    fn compute_index(&self, index: usize) -> usize {
        (self.zero + index) % self.inner.len()
    }

    /// Rotate the buffer up by `n` positions (moves bottom to top).
    #[allow(dead_code)]
    fn rotate(&mut self, n: usize) {
        self.zero = (self.zero + n) % self.inner.len();
    }

    /// Rotate the buffer down by `n` positions (moves top to bottom).
    #[allow(dead_code)]
    fn rotate_down(&mut self, n: usize) {
        self.zero = (self.zero + self.inner.len() - n % self.inner.len())
            % self.inner.len();
    }

    /// Increase the number of active lines (grow history).
    fn grow_lines(
        &mut self,
        additional: usize,
        columns: usize,
        template: &CellAttributes,
    ) {
        let new_len = self.len + additional;

        // Allocate more rows if needed.
        while self.inner.len() < new_len {
            self.inner.push(GridRow::new(columns, template));
        }

        self.len = new_len;
    }

    /// Decrease the number of active lines (shrink history).
    fn shrink_lines(&mut self, count: usize) {
        self.len = self.len.saturating_sub(count);
        if self.len < self.visible_lines {
            self.len = self.visible_lines;
        }
    }

    /// Get a reference to a row by logical index.
    fn get(&self, index: usize) -> Option<&GridRow> {
        if index < self.len {
            Some(&self.inner[self.compute_index(index)])
        } else {
            None
        }
    }

    /// Get a mutable reference to a row by logical index.
    fn get_mut(&mut self, index: usize) -> Option<&mut GridRow> {
        if index < self.len {
            let idx = self.compute_index(index);
            Some(&mut self.inner[idx])
        } else {
            None
        }
    }

    /// Resize all rows to a new width.
    fn resize_columns(&mut self, columns: usize, template: &CellAttributes) {
        for row in &mut self.inner {
            row.resize(columns, template);
        }
    }
}

/// Terminal grid with scrollback history support.
///
/// Maintains a visible area and scrollback buffer using a ring buffer
/// for efficient scrolling operations.
#[derive(Debug, Clone)]
pub struct Grid {
    storage: Storage,
    columns: usize,
    visible_lines: usize,
    max_scroll_limit: usize,
    display_offset: usize,
}

impl Grid {
    /// Create a new grid with the specified dimensions and scrollback limit.
    pub fn new(
        visible_lines: usize,
        columns: usize,
        max_scroll_limit: usize,
        template: &CellAttributes,
    ) -> Self {
        Self {
            storage: Storage::new(visible_lines, columns, template),
            columns,
            visible_lines,
            max_scroll_limit,
            display_offset: 0,
        }
    }

    /// Get the width (number of columns) of the grid.
    pub fn width(&self) -> usize {
        self.columns
    }

    /// Get the height (number of visible lines) of the grid.
    pub fn height(&self) -> usize {
        self.visible_lines
    }

    /// Get the number of lines currently in scrollback history.
    pub fn history_size(&self) -> usize {
        self.storage.len.saturating_sub(self.visible_lines)
    }

    /// Get the total number of lines (visible + history).
    pub fn total_lines(&self) -> usize {
        self.storage.len
    }

    /// Get the current display offset (how far scrolled back into history).
    pub fn display_offset(&self) -> usize {
        self.display_offset
    }

    /// Get a reference to all rows (for iteration, debugging).
    pub fn rows(&self) -> Vec<&GridRow> {
        (0..self.storage.len)
            .filter_map(|i| self.storage.get(i))
            .collect()
    }

    /// Get a reference to a visible row by index (0 = top visible row).
    pub fn row(&self, idx: usize) -> &GridRow {
        let history = self.history_size();
        let logical_idx = history.saturating_sub(self.display_offset) + idx;
        self.storage
            .get(logical_idx)
            .expect("row index out of bounds")
    }

    /// Get a mutable reference to a visible row by index.
    pub fn row_mut(&mut self, idx: usize) -> &mut GridRow {
        let history = self.history_size();
        let logical_idx = history.saturating_sub(self.display_offset) + idx;
        self.storage
            .get_mut(logical_idx)
            .expect("row index out of bounds")
    }

    /// Resize the grid to new dimensions.
    pub fn resize(
        &mut self,
        columns: usize,
        visible_lines: usize,
        template: &CellAttributes,
    ) {
        if columns != self.columns {
            self.storage.resize_columns(columns, template);
            self.columns = columns;
        }

        if visible_lines > self.visible_lines {
            let additional = visible_lines - self.visible_lines;
            self.storage.grow_lines(additional, columns, template);
        } else if visible_lines < self.visible_lines {
            let to_remove = self.visible_lines - visible_lines;
            self.storage.shrink_lines(to_remove);
        }

        self.storage.visible_lines = visible_lines;
        self.visible_lines = visible_lines;
        self.display_offset = 0;
    }

    /// Clear the entire grid with a template cell.
    pub fn clear(&mut self, template: &CellAttributes) {
        for i in 0..self.storage.len {
            if let Some(row) = self.storage.get_mut(i) {
                row.clear(template);
            }
        }
    }

    /// Clear all scrollback history.
    pub fn clear_history(&mut self) {
        self.storage.len = self.visible_lines;
        self.display_offset = 0;
    }

    /// Clear a range of cells in a row.
    pub fn clear_range(
        &mut self,
        row_idx: usize,
        start_col: usize,
        end_col: usize,
        template: &CellAttributes,
    ) {
        if row_idx >= self.visible_lines || start_col >= self.columns {
            return;
        }

        let end_col = end_col.min(self.columns.saturating_sub(1));
        let row = self.row_mut(row_idx);
        for col in start_col..=end_col {
            if col < row.cells.len() {
                row.cells[col] = Cell::blank(template);
            }
        }
    }

    /// Insert blank cells at a position, shifting content right.
    pub fn insert_blank_cells(
        &mut self,
        row_idx: usize,
        col_idx: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if row_idx >= self.visible_lines
            || col_idx >= self.columns
            || count == 0
        {
            return;
        }

        let columns = self.columns;
        let row = self.row_mut(row_idx);
        let max_shift = columns.saturating_sub(col_idx);
        let count = count.min(max_shift);

        for idx in (0..(columns - col_idx - count)).rev() {
            let source = col_idx + idx;
            let target = source + count;
            if target < row.cells.len() && source < row.cells.len() {
                row.cells[target] = row.cells[source].clone();
            }
        }

        for idx in col_idx..(col_idx + count).min(columns) {
            if idx < row.cells.len() {
                row.cells[idx] = Cell::blank(template);
            }
        }
    }

    /// Delete cells at a position, shifting content left.
    pub fn delete_cells(
        &mut self,
        row_idx: usize,
        col_idx: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if row_idx >= self.visible_lines
            || col_idx >= self.columns
            || count == 0
        {
            return;
        }

        let columns = self.columns;
        let row = self.row_mut(row_idx);
        let span = columns.saturating_sub(col_idx);
        let count = count.min(span);

        for idx in col_idx..columns {
            let source = idx + count;
            if source < row.cells.len() && idx < row.cells.len() {
                row.cells[idx] = row.cells[source].clone();
            } else if idx < row.cells.len() {
                row.cells[idx] = Cell::blank(template);
            }
        }
    }

    /// Scroll a region up by `count` lines.
    ///
    /// If `top == 0`, scrolled lines are moved into history.
    pub fn scroll_up(
        &mut self,
        top: usize,
        bottom: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if top > bottom || bottom >= self.visible_lines || count == 0 {
            return;
        }

        // If scrolling from the top, move lines into history.
        if top == 0 {
            // Add scrolled lines to history by growing the buffer.
            // Growing adds new rows at the end, which become the new bottom visible rows.
            // The old top rows are now in history.
            let history = self.history_size();
            let can_grow = self.max_scroll_limit.saturating_sub(history);
            let grow_by = count.min(can_grow);

            if grow_by > 0 {
                self.storage.grow_lines(grow_by, self.columns, template);

                // The newly added rows at the end need to be cleared (they're blank by default).
                // These represent the new bottom visible rows.
                let new_len = self.storage.len;
                for i in 0..grow_by {
                    let idx = new_len - grow_by + i;
                    if let Some(row) = self.storage.get_mut(idx) {
                        row.clear(template);
                    }
                }
            }

            // Reset display offset when new content arrives.
            if self.display_offset > 0 {
                self.display_offset = 0;
            }
        } else {
            // Scroll within a region (no history).
            let history = self.history_size();
            for _ in 0..count {
                for row in top..bottom {
                    let idx1 = history + row;
                    let idx2 = history + row + 1;
                    let physical1 = self.storage.compute_index(idx1);
                    let physical2 = self.storage.compute_index(idx2);
                    self.storage.inner.swap(physical1, physical2);
                }
                let clear_idx = history + bottom;
                if let Some(row) = self.storage.get_mut(clear_idx) {
                    row.clear(template);
                }
            }
        }
    }

    /// Scroll a region down by `count` lines.
    pub fn scroll_down(
        &mut self,
        top: usize,
        bottom: usize,
        count: usize,
        template: &CellAttributes,
    ) {
        if top > bottom || bottom >= self.visible_lines || count == 0 {
            return;
        }

        let history = self.history_size();

        for _ in 0..count {
            for row in (top + 1..=bottom).rev() {
                let idx1 = history + row;
                let idx2 = history + row - 1;
                let physical1 = self.storage.compute_index(idx1);
                let physical2 = self.storage.compute_index(idx2);
                self.storage.inner.swap(physical1, physical2);
            }
            let clear_idx = history + top;
            if let Some(row) = self.storage.get_mut(clear_idx) {
                row.clear(template);
            }
        }
    }

    /// Scroll the display viewport (user scrolling through history).
    pub fn scroll_display(&mut self, direction: ScrollDirection) {
        let history = self.history_size();

        match direction {
            ScrollDirection::Delta(delta) => {
                if delta > 0 {
                    // Scroll up into history.
                    self.display_offset =
                        (self.display_offset + delta as usize).min(history);
                } else {
                    // Scroll down toward bottom.
                    self.display_offset =
                        self.display_offset.saturating_sub((-delta) as usize);
                }
            },
            ScrollDirection::PageUp => {
                let page = self.visible_lines.saturating_sub(1).max(1);
                self.display_offset = (self.display_offset + page).min(history);
            },
            ScrollDirection::PageDown => {
                let page = self.visible_lines.saturating_sub(1).max(1);
                self.display_offset = self.display_offset.saturating_sub(page);
            },
            ScrollDirection::Top => {
                self.display_offset = history;
            },
            ScrollDirection::Bottom => {
                self.display_offset = 0;
            },
        }
    }

    /// Iterate over the currently displayed rows (accounting for display_offset).
    pub fn display_iter(&self) -> impl Iterator<Item = &GridRow> {
        let history = self.history_size();
        let start = history.saturating_sub(self.display_offset);
        (start..start + self.visible_lines)
            .filter_map(move |i| self.storage.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_rotation() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(3, 5, 100, &attrs);

        // Write distinct chars to each row.
        for i in 0..3 {
            grid.row_mut(i).cells[0] = Cell::with_char(
                char::from_digit(i as u32, 10).unwrap(),
                &attrs,
            );
        }

        // Scroll up (should rotate buffer).
        grid.scroll_up(0, 2, 1, &attrs);

        // Row 0 should now show what was row 1.
        assert_eq!(grid.row(0).cells[0].ch, '1');
        assert_eq!(grid.row(1).cells[0].ch, '2');
        // Row 2 should be cleared.
        assert_eq!(grid.row(2).cells[0].ch, ' ');
    }

    #[test]
    fn history_accumulation() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(3, 5, 100, &attrs);

        // Initially no history.
        assert_eq!(grid.history_size(), 0);

        // Scroll up from top should add to history.
        grid.scroll_up(0, 2, 1, &attrs);
        assert_eq!(grid.history_size(), 1);

        grid.scroll_up(0, 2, 2, &attrs);
        assert_eq!(grid.history_size(), 3);
    }

    #[test]
    fn display_offset_scrolling() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(3, 5, 100, &attrs);

        // Add some history.
        grid.scroll_up(0, 2, 5, &attrs);
        assert_eq!(grid.history_size(), 5);

        // Scroll display up.
        grid.scroll_display(ScrollDirection::Delta(2));
        assert_eq!(grid.display_offset(), 2);

        // Scroll to top.
        grid.scroll_display(ScrollDirection::Top);
        assert_eq!(grid.display_offset(), 5);

        // Scroll to bottom.
        grid.scroll_display(ScrollDirection::Bottom);
        assert_eq!(grid.display_offset(), 0);
    }
}
