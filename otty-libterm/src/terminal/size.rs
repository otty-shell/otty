use otty_pty::PtySize;

use crate::surface::{Column, Dimensions, Line};

#[derive(Clone, Copy, Debug)]
pub struct TerminalSize {
    pub cell_width: u16,
    pub cell_height: u16,
    pub cols: u16,
    pub rows: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            cell_width: 1,
            cell_height: 1,
            cols: 80,
            rows: 50,
        }
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.rows as usize
    }

    fn columns(&self) -> usize {
        self.cols as usize
    }

    fn last_column(&self) -> Column {
        Column(self.cols as usize - 1)
    }

    fn bottommost_line(&self) -> Line {
        Line(self.rows as i32 - 1)
    }
}

impl From<TerminalSize> for PtySize {
    fn from(val: TerminalSize) -> Self {
        PtySize {
            rows: val.rows,
            cols: val.cols,
            cell_width: val.cell_width,
            cell_height: val.cell_height,
        }
    }
}
