use std::{
    cmp::{max, min},
    ops::{Index, IndexMut},
};

use log::debug;

use crate::cell::{Cell, CellAttributes};

/// A single row in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridRow {
    pub cells: Vec<Cell>,
    overflow: Vec<Cell>,
    soft_wrap: bool,
}

impl GridRow {
    pub fn new(width: usize, template: &CellAttributes) -> Self {
        let cells = (0..width).map(|_| Cell::blank(template)).collect();
        Self {
            cells,
            overflow: Vec::new(),
            soft_wrap: false,
        }
    }

    pub fn resize(&mut self, width: usize, template: &CellAttributes) {
        if width > self.cells.len() {
            let deficit = width - self.cells.len();
            let restored = self
                .overflow
                .split_off(self.overflow.len().saturating_sub(deficit));
            for cell in restored.into_iter().rev() {
                self.cells.push(cell);
            }
            if self.cells.len() < width {
                self.cells.extend(
                    (0..(width - self.cells.len()))
                        .map(|_| Cell::blank(template)),
                );
            }
        } else {
            while self.cells.len() > width {
                if let Some(cell) = self.cells.pop() {
                    self.overflow.push(cell);
                }
            }
        }
    }

    pub fn clear(&mut self, template: &CellAttributes) {
        for cell in &mut self.cells {
            *cell = Cell::blank(template);
        }
        self.overflow.clear();
        self.soft_wrap = false;
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn set_soft_wrap(&mut self, value: bool) {
        self.soft_wrap = value;
    }

    pub fn is_soft_wrap(&self) -> bool {
        self.soft_wrap
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
            self.reflow_columns(columns, visible_lines, template);
            return;
        }

        self.resize_visible_lines(visible_lines, columns, template);
    }

    fn resize_visible_lines(
        &mut self,
        visible_lines: usize,
        columns: usize,
        template: &CellAttributes,
    ) {
        let previous_offset = self.display_offset;
        let target_visible = visible_lines.max(1);
        let current_total = self.storage.len;

        if target_visible > current_total {
            let growth = target_visible - current_total;
            self.storage.grow_lines(growth, columns, template);
        }

        self.storage.visible_lines = target_visible;
        self.visible_lines = target_visible;
        self.display_offset = min(previous_offset, self.history_size());

        let max_total =
            self.visible_lines.saturating_add(self.max_scroll_limit);
        if self.storage.len > max_total {
            let excess = self.storage.len - max_total;
            if self.storage.len > 0 {
                self.storage.rotate(excess % self.storage.len);
            }
            self.storage.shrink_lines(excess);
        }
    }

    fn reflow_columns(
        &mut self,
        columns: usize,
        visible_lines: usize,
        template: &CellAttributes,
    ) {
        let target_visible = visible_lines.max(1);
        let max_total = target_visible.saturating_add(self.max_scroll_limit);

        let mut logical_lines: Vec<Vec<Cell>> = Vec::new();
        let mut current_line: Vec<Cell> = Vec::new();

        for idx in 0..self.storage.len {
            if let Some(row) = self.storage.get(idx) {
                let trimmed = trim_trailing_blanks(&row.cells);
                if trimmed.is_empty() && !row.is_soft_wrap() {
                    if !current_line.is_empty() {
                        logical_lines.push(std::mem::take(&mut current_line));
                    }
                    logical_lines.push(Vec::new());
                    continue;
                }

                current_line.extend(trimmed.into_iter());
                if !row.is_soft_wrap() {
                    logical_lines.push(std::mem::take(&mut current_line));
                }
            }
        }

        if !current_line.is_empty() {
            logical_lines.push(current_line);
        }

        if logical_lines.is_empty() {
            logical_lines.push(Vec::new());
        }

        let mut new_rows: Vec<GridRow> = Vec::new();
        for line in logical_lines.into_iter() {
            if line.is_empty() {
                let mut row = GridRow::new(columns, template);
                row.set_soft_wrap(false);
                new_rows.push(row);
                continue;
            }

            let mut cursor = 0;
            while cursor < line.len() {
                let mut chunk_end = (cursor + columns).min(line.len());
                if chunk_end < line.len() {
                    while chunk_end > cursor
                        && line[chunk_end - 1].is_wide_leading()
                    {
                        chunk_end -= 1;
                    }
                    if chunk_end == cursor {
                        chunk_end = (cursor + 1).min(line.len());
                    }
                }

                let mut row = GridRow::new(columns, template);
                for (dst, cell) in
                    row.cells.iter_mut().zip(line[cursor..chunk_end].iter())
                {
                    *dst = cell.clone();
                }
                row.set_soft_wrap(chunk_end < line.len());
                new_rows.push(row);
                cursor = chunk_end;
            }
        }

        if new_rows.len() > max_total {
            let excess = new_rows.len() - max_total;
            new_rows.drain(0..excess);
        }

        while new_rows.len() < target_visible {
            new_rows.push(GridRow::new(columns, template));
        }

        self.storage.inner = new_rows;
        self.storage.zero = 0;
        self.storage.len = self.storage.inner.len();
        self.storage.visible_lines = target_visible.min(self.storage.len);
        self.columns = columns;
        self.visible_lines = target_visible.min(self.storage.len);
        self.display_offset = self.display_offset.min(self.history_size());
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
        debug!("[grid] clear_history: resetting display_offset to 0");
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
            let overflow = count.saturating_sub(grow_by);

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

            if overflow > 0 && self.storage.len > 0 {
                // No room left to grow history: rotate the ring buffer so the oldest
                // lines fall off the front, then reuse those rows for the new blanks.
                self.storage.rotate(overflow);
                let new_len = self.storage.len;
                for i in 0..overflow {
                    let idx = new_len.saturating_sub(overflow) + i;
                    if let Some(row) = self.storage.get_mut(idx) {
                        row.clear(template);
                    }
                }
            }

            // Reset display offset when new content arrives.
            if self.display_offset > 0 {
                debug!(
                    "[grid] scroll_up: resetting display_offset {} -> 0 (new content)",
                    self.display_offset
                );
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
        let old_offset = self.display_offset;
        self.display_offset = match direction {
            ScrollDirection::Delta(count) => min(
                max((self.display_offset as i32) + count, 0) as usize,
                self.history_size(),
            ),
            ScrollDirection::PageUp => min(
                self.display_offset + self.visible_lines,
                self.history_size(),
            ),
            ScrollDirection::PageDown => {
                self.display_offset.saturating_sub(self.visible_lines)
            },
            ScrollDirection::Top => self.history_size(),
            ScrollDirection::Bottom => 0,
        };
        debug!(
            "[grid] scroll_display: {:?}, offset {} -> {}, history={}",
            direction,
            old_offset,
            self.display_offset,
            self.history_size()
        );
    }

    /// Iterate over the currently displayed rows (accounting for display_offset).
    pub fn display_iter(&self) -> impl Iterator<Item = &GridRow> {
        let history = self.history_size();
        let start = history.saturating_sub(self.display_offset);
        (start..start + self.visible_lines)
            .filter_map(move |i| self.storage.get(i))
    }
}

fn trim_trailing_blanks(cells: &[Cell]) -> Vec<Cell> {
    if cells.is_empty() {
        return Vec::new();
    }

    let mut end = cells.len();
    while end > 0 {
        let cell = &cells[end - 1];
        if !cell.is_blank()
            || cell.touched
            || cell.is_wide_leading()
            || cell.is_wide_trailing()
        {
            break;
        }

        end -= 1;
    }

    cells[..end].to_vec()
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

    #[test]
    fn history_overflow_drops_oldest_lines() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(2, 1, 3, &attrs);

        grid.row_mut(0).cells[0] = Cell::with_char('0', &attrs);
        grid.row_mut(1).cells[0] = Cell::with_char('1', &attrs);
        let bottom = grid.height().saturating_sub(1);

        for ch in ['2', '3', '4', '5'] {
            grid.scroll_up(0, bottom, 1, &attrs);
            grid.row_mut(bottom).cells[0] = Cell::with_char(ch, &attrs);
        }

        assert_eq!(grid.history_size(), 3);
        let rows: Vec<char> =
            grid.rows().iter().map(|row| row.cells[0].ch).collect();
        assert_eq!(rows, vec!['1', '2', '3', '4', '5']);
    }

    #[test]
    fn row_resize_preserves_truncated_cells() {
        let attrs = CellAttributes::default();
        let mut row = GridRow::new(4, &attrs);
        for (idx, ch) in ['A', 'B', 'C', 'D'].into_iter().enumerate() {
            row.cells[idx] = Cell::with_char(ch, &attrs);
        }

        row.resize(2, &attrs);
        assert_eq!(row.cells.len(), 2);
        assert_eq!(row.cells[0].ch, 'A');
        assert_eq!(row.cells[1].ch, 'B');

        row.resize(4, &attrs);
        let chars: Vec<char> = row.cells.iter().map(|cell| cell.ch).collect();
        assert_eq!(chars, vec!['A', 'B', 'C', 'D']);
    }

    #[test]
    fn resize_reflows_when_columns_shrink() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(3, 10, 100, &attrs);
        let text: Vec<char> = "HELLOWORLD".chars().collect();
        for (idx, ch) in text.iter().enumerate() {
            grid.row_mut(0).cells[idx] = Cell::with_char(*ch, &attrs);
        }

        grid.resize(4, 3, &attrs);

        let rows: Vec<String> = grid
            .rows()
            .iter()
            .map(|row| row.cells.iter().map(|c| c.ch).collect::<String>())
            .collect();
        assert!(rows.len() >= 3);
        assert_eq!(&rows[0][..4], "HELL");
        assert_eq!(&rows[1][..4], "OWOR");
        assert_eq!(&rows[2][..2], "LD");
    }

    #[test]
    fn resize_reflows_when_columns_expand() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(3, 4, 100, &attrs);
        let segments = ["HELL", "OWOR", "LD  "];
        for (idx, segment) in segments.iter().enumerate() {
            for (col, ch) in segment.chars().enumerate() {
                grid.row_mut(idx).cells[col] = Cell::with_char(ch, &attrs);
            }
            if idx < segments.len() - 1 {
                grid.row_mut(idx).set_soft_wrap(true);
            }
        }

        grid.resize(10, 3, &attrs);

        let rows: Vec<String> = grid
            .rows()
            .iter()
            .map(|row| row.cells.iter().map(|c| c.ch).collect::<String>())
            .collect();
        assert!(!rows.is_empty());
        assert_eq!(&rows[0][..10], "HELLOWORLD");
    }

    #[test]
    fn resize_preserves_explicit_trailing_spaces() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(2, 6, 100, &attrs);

        grid.row_mut(0).cells[0] = Cell::with_char('A', &attrs);
        grid.row_mut(0).cells[1] = Cell::with_char(' ', &attrs);
        grid.row_mut(0).cells[2] = Cell::with_char(' ', &attrs);

        grid.resize(4, 2, &attrs);
        grid.resize(6, 2, &attrs);

        let row = grid.row(0);
        let snapshot: Vec<char> =
            row.cells.iter().take(3).map(|cell| cell.ch).collect();
        assert_eq!(snapshot, vec!['A', ' ', ' ']);
    }

    #[test]
    fn shrinking_visible_lines_moves_rows_into_history() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(4, 2, 10, &attrs);

        for row in 0..4 {
            let ch = char::from(b'0' + row as u8);
            grid.row_mut(row).cells[0] = Cell::with_char(ch, &attrs);
        }

        grid.resize(2, 2, &attrs);

        assert_eq!(grid.height(), 2);
        assert_eq!(grid.history_size(), 2);
        assert_eq!(grid.row(0).cells[0].ch, '2');
        assert_eq!(grid.row(1).cells[0].ch, '3');

        let rows: Vec<char> =
            grid.rows().iter().map(|row| row.cells[0].ch).collect();
        assert_eq!(rows, vec!['0', '1', '2', '3']);
    }

    #[test]
    fn resize_expands_view_using_history_before_adding_blanks() {
        let attrs = CellAttributes::default();
        let mut grid = Grid::new(2, 1, 10, &attrs);
        let bottom = grid.height().saturating_sub(1);

        for ch in ['0', '1', '2', '3', '4'] {
            grid.scroll_up(0, bottom, 1, &attrs);
            grid.row_mut(bottom).cells[0] = Cell::with_char(ch, &attrs);
        }

        grid.resize(1, 4, &attrs);

        let visible: Vec<char> = (0..grid.height())
            .map(|row| grid.row(row).cells[0].ch)
            .collect();
        assert_eq!(visible, vec!['1', '2', '3', '4']);
    }
}
