use nix::libc::winsize;

/// The size of the visible display area in the pty
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    /// The number of lines of text
    pub rows: u16,
    /// The number of columns of text
    pub cols: u16,
    /// The width of a cell in pixels.
    pub cell_width: u16,
    /// The height of a cell in pixels.
    pub cell_height: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        PtySize {
            rows: 30,
            cols: 80,
            cell_width: 0,
            cell_height: 0,
        }
    }
}

impl From<PtySize> for winsize {
    fn from(value: PtySize) -> winsize {
        winsize {
            ws_row: value.rows,
            ws_col: value.cols,
            ws_xpixel: value.cell_width,
            ws_ypixel: value.cell_height,
        }
    }
}
