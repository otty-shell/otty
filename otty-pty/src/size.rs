#[cfg(unix)]
use nix::libc::{self, winsize};

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

#[cfg(unix)]
impl From<PtySize> for winsize {
    fn from(value: PtySize) -> winsize {
        let ws_row = value.rows as libc::c_ushort;
        let ws_col = value.cols as libc::c_ushort;

        let ws_xpixel = ws_col * value.cell_width as libc::c_ushort;
        let ws_ypixel = ws_row * value.cell_height as libc::c_ushort;

        winsize {
            ws_row,
            ws_col,
            ws_xpixel,
            ws_ypixel,
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use nix::libc::winsize;

    #[test]
    fn converts_to_winsize_without_loss() {
        let size = PtySize {
            rows: 42,
            cols: 120,
            cell_width: 8,
            cell_height: 16,
        };

        let winsize {
            ws_row,
            ws_col,
            ws_xpixel,
            ws_ypixel,
        } = winsize::from(size);

        assert_eq!(ws_row, 42);
        assert_eq!(ws_col, 120);
        assert_eq!(ws_xpixel, 960);
        assert_eq!(ws_ypixel, 672);
    }
}
