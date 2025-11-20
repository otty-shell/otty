use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

use anyhow::Result;

#[cfg(unix)]
fn main() -> Result<()> {
    unix_shell::run()
}

#[cfg(not(unix))]
fn main() -> Result<()> {
    eprintln!("This example is only supported on Unix platforms.");
    Ok(())
}

#[cfg(unix)]
mod unix_shell {
    use super::*;
    use nix::fcntl::{FcntlArg, OFlag, fcntl};
    use nix::libc;
    use otty_escape::{Color as AnsiColor, StdColor};
    use otty_libterm::surface::Colors;
    use otty_libterm::{
        TerminalEngine, TerminalEvent, TerminalOptions, TerminalRequest,
        TerminalSize, escape,
        pty::{self, PtySize},
        surface::{
            Dimensions, Flags, FrameCell, FrameOwned, FrameView, Surface,
            SurfaceConfig,
        },
    };
    use std::os::fd::{AsRawFd, BorrowedFd};

    pub fn run() -> Result<()> {
        let (rows, cols) = query_winsize().unwrap_or((24, 80));

        let mut current_size = TerminalSize {
            rows,
            cols,
            cell_width: 0,
            cell_height: 0,
        };

        let pty_size = PtySize {
            rows: current_size.rows,
            cols: current_size.cols,
            cell_width: current_size.cell_width,
            cell_height: current_size.cell_height,
        };

        let session = pty::unix("/bin/sh")
            .with_arg("-i")
            .with_size(pty_size)
            .set_controling_tty_enable()
            .spawn()?;

        let surface_dimensions = TerminalDimensions {
            columns: current_size.cols as usize,
            rows: current_size.rows as usize,
        };
        let surface =
            Surface::new(SurfaceConfig::default(), &surface_dimensions);
        let parser: escape::Parser<escape::vte::Parser> = Default::default();
        let options = TerminalOptions::default();
        let mut engine =
            TerminalEngine::new(session, parser, surface, options)?;

        let mut stdin = io::stdin();
        let mut input = [0u8; 1024];
        set_nonblocking(&stdin)?;

        loop {
            match stdin.read(&mut input) {
                Ok(read) if read > 0 => {
                    engine.queue_request(TerminalRequest::WriteBytes(
                        input[..read].to_vec(),
                    ))?;
                },
                Ok(_) => {},
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {},
                Err(err) => return Err(err.into()),
            }

            engine.on_readable()?;

            if engine.has_pending_output() {
                engine.on_writable()?;
            }

            engine.tick()?;

            if let Some((rows, cols)) = query_winsize() {
                let new_size = TerminalSize {
                    rows,
                    cols,
                    cell_width: 0,
                    cell_height: 0,
                };
                if new_size.rows != current_size.rows
                    || new_size.cols != current_size.cols
                {
                    current_size = new_size;
                    engine.queue_request(TerminalRequest::Resize(new_size))?;
                }
            }

            while let Some(event) = engine.next_event() {
                match event {
                    TerminalEvent::Frame { frame } => {
                        render_frame(&frame)?;
                    },
                    TerminalEvent::ChildExit { status } => {
                        eprintln!("Child exited with {status}");
                        return Ok(());
                    },
                    TerminalEvent::TitleChanged { title } => {
                        eprintln!("Title changed: {title}");
                    },
                    TerminalEvent::ResetTitle => {
                        eprintln!("Title reset");
                    },
                    TerminalEvent::Bell => {
                        eprintln!("Bell");
                    },
                    _ => {},
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    struct TerminalDimensions {
        columns: usize,
        rows: usize,
    }

    impl Dimensions for TerminalDimensions {
        fn total_lines(&self) -> usize {
            self.rows
        }

        fn screen_lines(&self) -> usize {
            self.rows
        }

        fn columns(&self) -> usize {
            self.columns
        }
    }

    fn query_winsize() -> Option<(u16, u16)> {
        let fd = io::stdout().as_raw_fd();
        let mut ws = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let res = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
        if res == 0 && ws.ws_row > 0 && ws.ws_col > 0 {
            Some((ws.ws_row, ws.ws_col))
        } else {
            None
        }
    }

    /// Minimal ANSI renderer honoring colors and basic attributes.
    fn render_frame(frame: &FrameOwned) -> Result<()> {
        let view = frame.view();
        let cols = view.size.columns;
        let rows = view.size.screen_lines;

        let mut out = String::new();
        out.push_str("\u{1b}[?25l"); // hide cursor during redraw
        out.push_str("\u{1b}[2J\u{1b}[H"); // clear and home

        let mut last_sgr = String::new();
        let mut idx = 0usize;
        for _ in 0..rows {
            for _ in 0..cols {
                let cell = &view.cells[idx];
                idx += 1;

                let sgr = cell_sgr(cell, &view);
                if sgr != last_sgr {
                    out.push_str(&sgr);
                    last_sgr = sgr;
                }

                let mut ch = cell.cell.c;
                if cell.cell.flags.contains(Flags::HIDDEN) {
                    ch = ' ';
                }
                out.push(ch);
            }
            out.push('\n');
        }

        // Reset attributes and place cursor.
        out.push_str("\u{1b}[0m");
        let cursor = view.cursor.point;
        let cursor_row = cursor.line.0 as usize;
        let cursor_col = cursor.column.0;
        if cursor_row < rows && cursor_col < cols {
            out.push_str(&format!(
                "\u{1b}[{};{}H",
                cursor_row + 1,
                cursor_col + 1
            ));
            if matches!(view.cursor.shape, otty_escape::CursorShape::Hidden) {
                out.push_str("\u{1b}[?25l");
            } else {
                out.push_str("\u{1b}[?25h");
            }
        }

        let mut stdout = io::stdout();
        stdout.write_all(out.as_bytes())?;
        stdout.flush()?;
        Ok(())
    }

    fn cell_sgr(cell: &FrameCell, view: &FrameView<'_>) -> String {
        let mut codes: Vec<String> = Vec::new();
        codes.push("0".to_string()); // reset to base before applying attributes

        let flags = cell.cell.flags;
        if flags.contains(Flags::BOLD) || flags.contains(Flags::DIM_BOLD) {
            codes.push("1".to_string());
        }
        if flags.contains(Flags::DIM) || flags.contains(Flags::DIM_BOLD) {
            codes.push("2".to_string());
        }
        if flags.intersects(Flags::ITALIC | Flags::BOLD_ITALIC) {
            codes.push("3".to_string());
        }
        if flags.contains(Flags::STRIKEOUT) {
            codes.push("9".to_string());
        }
        if flags.intersects(Flags::UNDERLINE | Flags::DOUBLE_UNDERLINE) {
            codes.push(
                if flags.contains(Flags::DOUBLE_UNDERLINE) {
                    "21"
                } else {
                    "4"
                }
                .to_string(),
            );
        }
        if flags.intersects(
            Flags::UNDERCURL
                | Flags::DOTTED_UNDERLINE
                | Flags::DASHED_UNDERLINE,
        ) {
            // Best-effort underline for unsupported styles.
            codes.push("4".to_string());
        }
        if flags.contains(Flags::INVERSE) {
            codes.push("7".to_string());
        }
        if flags.contains(Flags::HIDDEN) {
            codes.push("8".to_string());
        }

        let mut fg = cell.cell.fg;
        let mut bg = cell.cell.bg;
        if flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        if let Some(fg_code) = color_sgr(fg, view.colors, true) {
            codes.push(fg_code);
        }
        if let Some(bg_code) = color_sgr(bg, view.colors, false) {
            codes.push(bg_code);
        }

        format!("\u{1b}[{}m", codes.join(";"))
    }

    fn color_sgr(
        color: AnsiColor,
        palette: &Colors,
        is_fg: bool,
    ) -> Option<String> {
        let prefix = if is_fg { "38" } else { "48" };
        match color {
            AnsiColor::TrueColor(rgb) => {
                Some(format!("{prefix};2;{};{};{}", rgb.r, rgb.g, rgb.b))
            },
            AnsiColor::Indexed(idx) => Some(format!("{prefix};5;{idx}")),
            AnsiColor::Std(std) => {
                if let Some(rgb) = palette[std] {
                    return Some(format!(
                        "{prefix};2;{};{};{}",
                        rgb.r, rgb.g, rgb.b
                    ));
                }

                match std {
                    StdColor::Foreground => {
                        if is_fg {
                            Some("39".to_string())
                        } else {
                            Some("49".to_string())
                        }
                    },
                    StdColor::Background => {
                        if is_fg {
                            Some("39".to_string())
                        } else {
                            Some("49".to_string())
                        }
                    },
                    StdColor::BrightForeground => {
                        if is_fg {
                            Some("97".to_string())
                        } else {
                            Some("49".to_string())
                        }
                    },
                    StdColor::DimForeground => {
                        if is_fg {
                            Some("39".to_string())
                        } else {
                            Some("49".to_string())
                        }
                    },
                    StdColor::Cursor => None,
                    _ => {
                        let (base, bright) = match std {
                            StdColor::Black => (0, false),
                            StdColor::Red => (1, false),
                            StdColor::Green => (2, false),
                            StdColor::Yellow => (3, false),
                            StdColor::Blue => (4, false),
                            StdColor::Magenta => (5, false),
                            StdColor::Cyan => (6, false),
                            StdColor::White => (7, false),
                            StdColor::BrightBlack => (0, true),
                            StdColor::BrightRed => (1, true),
                            StdColor::BrightGreen => (2, true),
                            StdColor::BrightYellow => (3, true),
                            StdColor::BrightBlue => (4, true),
                            StdColor::BrightMagenta => (5, true),
                            StdColor::BrightCyan => (6, true),
                            StdColor::BrightWhite => (7, true),
                            StdColor::DimBlack => (0, false),
                            StdColor::DimRed => (1, false),
                            StdColor::DimGreen => (2, false),
                            StdColor::DimYellow => (3, false),
                            StdColor::DimBlue => (4, false),
                            StdColor::DimMagenta => (5, false),
                            StdColor::DimCyan => (6, false),
                            StdColor::DimWhite => (7, false),
                            StdColor::DimForeground => (0, false),
                            StdColor::BrightForeground => (7, true),
                            _ => (0, false),
                        };

                        let code = if is_fg {
                            if bright { 90 + base } else { 30 + base }
                        } else if bright {
                            100 + base
                        } else {
                            40 + base
                        };
                        Some(code.to_string())
                    },
                }
            },
        }
    }

    fn set_nonblocking(stdin: &io::Stdin) -> Result<()> {
        let raw_fd = stdin.as_raw_fd();
        let fd = unsafe { BorrowedFd::borrow_raw(raw_fd) };
        let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL)?);
        let new_flags = flags | OFlag::O_NONBLOCK;
        fcntl(fd, FcntlArg::F_SETFL(new_flags))?;
        Ok(())
    }
}
