use crate::cell::{Attrs, Cell};
use crate::color::Color;
use crate::cursor::Cursor;
use otty_vte::{Actor, CsiParam, Parser};

pub struct GridSurface {
    inner: GridSurfaceInner,
    parser: Parser,
}

impl GridSurface {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            inner: GridSurfaceInner::new(width, height),
            parser: Parser::new(),
        }
    }

    /// Current width (columns).
    #[must_use]
    pub fn width(&self) -> usize {
        self.inner.width
    }

    /// Current height (rows).
    #[must_use]
    pub fn height(&self) -> usize {
        self.inner.height
    }

    /// Get a reference to the internal cell buffer.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.inner.cells
    }

    /// Get the cursor state.
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        self.inner.cursor
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.inner.resize(width, height);
    }

    /// Feed raw bytes, parsing via VTE and mutating the grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(bytes, &mut self.inner);
    }
}

/// A resizable grid-backed surface.
struct GridSurfaceInner {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    cursor: Cursor,
    tab_width: usize,
    // New Line Mode (LNM): if true, LF behaves like CR+LF
    lnm: bool,
}

impl Actor for GridSurfaceInner {
    fn print(&mut self, c: char) {
        self.print_char(c);
    }

    fn execute(&mut self, byte: u8) {
        self.execute_c0(byte);
    }

    fn hook(
        &mut self,
        _byte: u8,
        _params: &[i64],
        _intermediates: &[u8],
        _ignored_excess_intermediates: bool,
    ) {
    }

    fn unhook(&mut self) {}

    fn put(&mut self, _byte: u8) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]]) {}

    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        _parameters_truncated: bool,
        byte: u8,
    ) {
        self.handle_csi(params, byte);
    }

    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        _intermediates: &[u8],
        _ignored_excess_intermediates: bool,
        byte: u8,
    ) {
        self.handle_esc(byte);
    }
}

impl GridSurfaceInner {
    /// Create a new grid surface with given size (columns x rows).
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        let mut s = Self {
            width: width.max(1),
            height: height.max(1),
            cells: vec![Cell::default(); width.max(1) * height.max(1)],
            cursor: Cursor::default(),
            tab_width: 8,
            lnm: true,
        };

        s.clamp_cursor();
        s.update_all_coords();
        s
    }

    /// Resize the surface, preserving as much content as possible.
    fn resize(&mut self, width: usize, height: usize) {
        let width = width.max(1);
        let height = height.max(1);
        if width == self.width && height == self.height {
            return;
        }

        let mut new_cells = vec![Cell::default(); width * height];
        let min_w = self.width.min(width);
        let min_h = self.height.min(height);
        for row in 0..min_h {
            let src = &self.cells[row * self.width..row * self.width + min_w];
            let dst = &mut new_cells[row * width..row * width + min_w];
            dst.copy_from_slice(src);
        }

        self.width = width;
        self.height = height;
        self.cells = new_cells;
        self.clamp_cursor();
        self.update_all_coords();
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn clamp_cursor(&mut self) {
        self.cursor.x = self.cursor.x.min(self.width - 1);
        self.cursor.y = self.cursor.y.min(self.height - 1);
    }

    fn clear_cell(&mut self, x: usize, y: usize) {
        let i = self.idx(x, y);
        let mut cell = Cell::default();
        cell.column = x;
        cell.line = y;
        self.cells[i] = cell;
    }

    fn clear_line_range(&mut self, y: usize, start: usize, end: usize) {
        let start = start.min(self.width);
        let end = end.min(self.width);
        for x in start..end {
            self.clear_cell(x, y);
        }
    }

    fn clear_display_from(&mut self, x: usize, y: usize) {
        // Clear from (x,y) to end of display
        self.clear_line_range(y, x, self.width);
        for row in (y + 1)..self.height {
            self.clear_line_range(row, 0, self.width);
        }
    }

    fn clear_display_to(&mut self, x: usize, y: usize) {
        // Clear from start to (x,y)
        for row in 0..y {
            self.clear_line_range(row, 0, self.width);
        }
        self.clear_line_range(y, 0, x + 1);
    }

    fn clear_display_all(&mut self) {
        for y in 0..self.height {
            self.clear_line_range(y, 0, self.width);
        }
    }

    fn scroll_up(&mut self, lines: usize) {
        let n = lines.min(self.height);
        if n == 0 {
            return;
        }
        // Move rows up by n; drop top n, add n blank rows at bottom
        for row in 0..(self.height - n) {
            let dst_start = row * self.width;
            let src_start = (row + n) * self.width;
            let (left, right) = self.cells.split_at_mut(src_start);
            let dst_slice = &mut left[dst_start..dst_start + self.width];
            let src_slice = &right[..self.width];
            dst_slice.copy_from_slice(src_slice);
            // Fix coordinates for moved row
            self.update_coords_row(row);
        }
        for row in (self.height - n)..self.height {
            self.clear_line_range(row, 0, self.width);
        }
    }

    fn scroll_down(&mut self, lines: usize) {
        let n = lines.min(self.height);
        if n == 0 {
            return;
        }
        // Move rows down by n; drop bottom n, add n blank rows at top
        for row in (n..self.height).rev() {
            let dst_start = row * self.width;
            let src_start = (row - n) * self.width;
            let (left, right) = self.cells.split_at_mut(dst_start);
            let dst_slice = &mut right[..self.width];
            let src_slice = &left[src_start..src_start + self.width];
            dst_slice.copy_from_slice(src_slice);
            // Fix coordinates for moved row
            self.update_coords_row(row);
        }
        for row in 0..n {
            self.clear_line_range(row, 0, self.width);
        }
    }

    fn carriage_return(&mut self) {
        self.cursor.x = 0;
    }

    fn linefeed(&mut self) {
        if self.cursor.y + 1 >= self.height {
            self.scroll_up(1);
        } else {
            self.cursor.y += 1;
        }
    }

    fn reverse_index(&mut self) {
        if self.cursor.y == 0 {
            self.scroll_down(1);
        } else {
            self.cursor.y -= 1;
        }
    }

    fn horizontal_tab(&mut self) {
        let next = ((self.cursor.x / self.tab_width) + 1) * self.tab_width;
        self.cursor.x = next.min(self.width - 1);
    }

    fn backspace(&mut self) {
        if self.cursor.x > 0 {
            self.cursor.x -= 1;
        }
    }

    fn print_char(&mut self, ch: char) {
        match ch {
            '\n' => {
                if self.lnm {
                    self.carriage_return();
                }
                self.linefeed();
            },
            '\r' => self.carriage_return(),
            '\t' => self.horizontal_tab(),
            _ => {
                let x = self.cursor.x;
                let y = self.cursor.y;
                let i = self.idx(x, y);
                self.cells[i] = Cell {
                    column: x,
                    line: y,
                    ch,
                    attr: self.cursor.attr,
                };
                if x + 1 >= self.width {
                    self.carriage_return();
                    self.linefeed();
                } else {
                    self.cursor.x += 1;
                }
            },
        }
    }

    fn execute_c0(&mut self, byte: u8) {
        match byte {
            0x08 => self.backspace(),      // BS
            0x09 => self.horizontal_tab(), // HT
            0x0A | 0x0B | 0x0C => {
                if self.lnm {
                    self.carriage_return();
                }
                self.linefeed();
            }, // LF/VT/FF
            0x0D => self.carriage_return(), // CR
            _ => {},
        }
    }

    fn handle_esc(&mut self, byte: u8) {
        match byte as char {
            'D' => self.linefeed(), // IND
            'E' => {
                self.carriage_return();
                self.linefeed();
            }, // NEL
            'M' => self.reverse_index(), // RI
            'c' => self.reset(),    // RIS (hard reset)
            _ => {},
        }
    }

    fn handle_csi(&mut self, params: &[CsiParam], byte: u8) {
        match byte as char {
            'A' => {
                // CUU
                let n = csi_first_or_default(params, 1) as usize;
                self.cursor.y =
                    self.cursor.y.saturating_sub(n).min(self.height - 1);
            },
            'B' => {
                // CUD
                let n = csi_first_or_default(params, 1) as usize;
                self.cursor.y = (self.cursor.y + n).min(self.height - 1);
            },
            'C' => {
                // CUF
                let n = csi_first_or_default(params, 1) as usize;
                self.cursor.x = (self.cursor.x + n).min(self.width - 1);
            },
            'D' => {
                // CUB
                let n = csi_first_or_default(params, 1) as usize;
                self.cursor.x = self.cursor.x.saturating_sub(n);
            },
            'H' | 'f' => {
                // CUP / HVP
                let (row, col) = csi_two_or_default(params, 1, 1);
                // 1-based to 0-based
                let y = row.saturating_sub(1) as usize;
                let x = col.saturating_sub(1) as usize;
                self.cursor.y = y.min(self.height - 1);
                self.cursor.x = x.min(self.width - 1);
            },
            'J' => {
                // ED
                match csi_first_or_default(params, 0) {
                    0 => self.clear_display_from(self.cursor.x, self.cursor.y),
                    1 => self.clear_display_to(self.cursor.x, self.cursor.y),
                    2 => self.clear_display_all(),
                    _ => {},
                }
            },
            'K' => {
                // EL
                match csi_first_or_default(params, 0) {
                    0 => self.clear_line_range(
                        self.cursor.y,
                        self.cursor.x,
                        self.width,
                    ),
                    1 => self.clear_line_range(
                        self.cursor.y,
                        0,
                        self.cursor.x + 1,
                    ),
                    2 => self.clear_line_range(self.cursor.y, 0, self.width),
                    _ => {},
                }
            },
            'S' => {
                // SU
                let n = csi_first_or_default(params, 1) as usize;
                self.scroll_up(n);
            },
            'T' => {
                // SD
                let n = csi_first_or_default(params, 1) as usize;
                self.scroll_down(n);
            },
            'X' => {
                // ECH
                let n = csi_first_or_default(params, 1) as usize;
                let end = (self.cursor.x + n).min(self.width);
                self.clear_line_range(self.cursor.y, self.cursor.x, end);
            },
            'm' => self.handle_sgr(params),
            _ => {},
        }
    }

    fn handle_sgr(&mut self, params: &[CsiParam]) {
        let mut ints = params_integers(params);
        if ints.is_empty() {
            ints.push(0);
        }

        let mut i = 0;
        while i < ints.len() {
            let p = ints[i];
            i += 1;
            match p {
                0 => self.cursor.attr = Attrs::default(),
                1 => self.cursor.attr.bold = true,
                3 => self.cursor.attr.italic = true,
                4 => self.cursor.attr.underline = true,
                7 => self.cursor.attr.inverse = true,
                22 => self.cursor.attr.bold = false,
                23 => self.cursor.attr.italic = false,
                24 => self.cursor.attr.underline = false,
                27 => self.cursor.attr.inverse = false,
                30..=37 => {
                    self.cursor.attr.fg = Color::Indexed((p - 30) as u8);
                },
                90..=97 => {
                    self.cursor.attr.fg = Color::Indexed((p - 90 + 8) as u8);
                },
                40..=47 => {
                    self.cursor.attr.bg = Color::Indexed((p - 40) as u8);
                },
                100..=107 => {
                    self.cursor.attr.bg = Color::Indexed((p - 100 + 8) as u8);
                },
                39 => self.cursor.attr.fg = Color::Default,
                49 => self.cursor.attr.bg = Color::Default,
                38 | 48 => {
                    // Extended color: 38/48 ; 5 ; idx  or  38/48 ; 2 ; r ; g ; b
                    let is_fg = p == 38;
                    if i + 0 >= ints.len() {
                        break;
                    }
                    let mode = ints[i];
                    i += 1;
                    match mode {
                        5 => {
                            if i >= ints.len() {
                                break;
                            }
                            let idx = ints[i].clamp(0, 255) as u8;
                            i += 1;
                            if is_fg {
                                self.cursor.attr.fg = Color::Indexed(idx);
                            } else {
                                self.cursor.attr.bg = Color::Indexed(idx);
                            }
                        },
                        2 => {
                            if i + 2 >= ints.len() {
                                break;
                            }
                            let r = ints[i].clamp(0, 255) as u8;
                            i += 1;
                            let g = ints[i].clamp(0, 255) as u8;
                            i += 1;
                            let b = ints[i].clamp(0, 255) as u8;
                            i += 1;
                            let c = Color::Rgb(r, g, b);
                            if is_fg {
                                self.cursor.attr.fg = c;
                            } else {
                                self.cursor.attr.bg = c;
                            }
                        },
                        _ => {},
                    }
                },
                _ => {},
            }
        }
    }

    fn reset(&mut self) {
        self.cursor = Cursor::default();
        self.clear_display_all();
    }

    fn update_all_coords(&mut self) {
        for y in 0..self.height {
            self.update_coords_row(y);
        }
    }

    fn update_coords_row(&mut self, y: usize) {
        let start = y * self.width;
        for x in 0..self.width {
            let idx = start + x;
            self.cells[idx].line = y;
            self.cells[idx].column = x;
        }
    }
}

fn params_integers(params: &[CsiParam]) -> Vec<i64> {
    let mut out = Vec::with_capacity(params.len());
    for p in params {
        if let CsiParam::Integer(v) = p {
            out.push(*v);
        }
    }
    out
}

fn csi_first_or_default(params: &[CsiParam], default: i64) -> i64 {
    params_integers(params).first().copied().unwrap_or(default)
}

fn csi_two_or_default(params: &[CsiParam], a: i64, b: i64) -> (i64, i64) {
    let ints = params_integers(params);
    let first = *ints.get(0).unwrap_or(&a);
    let second = *ints.get(1).unwrap_or(&b);
    (first, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_print_and_wrap() {
        let mut s = GridSurface::new(3, 2);
        s.feed(b"abcd");
        // Expect: row0: abc, row1: d..
        assert_eq!(s.cells()[0].ch, 'a');
        assert_eq!(s.cells()[1].ch, 'b');
        assert_eq!(s.cells()[2].ch, 'c');
        assert_eq!(s.cells()[3].ch, 'd');
    }

    #[test]
    fn csi_move_and_sgr() {
        let mut s = GridSurface::new(5, 2);
        s.feed(b"\x1b[2;3H\x1b[31mX"); // Move to (2,3) 1-based => y=1,x=2; set red fg; print X
        let idx = 1 * s.width() + 2;
        assert_eq!(s.cells()[idx].ch, 'X');
        assert!(matches!(s.cells()[idx].attr.fg, Color::Indexed(1)));
    }
}
