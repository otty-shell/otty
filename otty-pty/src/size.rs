use nix::libc::winsize;
use portable_pty::PtySize as InnerPtySize;

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

impl From<PtySize> for InnerPtySize {
    fn from(size: PtySize) -> Self {
        InnerPtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: size.cell_width,
            pixel_height: size.cell_height,
        }
    }
}

impl Into<winsize> for PtySize {
    fn into(self) -> winsize {
        winsize {
            ws_row: self.rows,
            ws_col: self.cols,
            ws_xpixel: self.cell_width,
            ws_ypixel: self.cell_height,
        }
    }
}