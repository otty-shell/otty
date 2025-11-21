use std::io::{self, Read, Write};
use std::time::Duration;

use anyhow::{Result, anyhow};

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
        TerminalBuilder, TerminalEvent, TerminalRequest, TerminalSize, pty,
        surface::{Flags, FrameCell, FrameOwned},
    };
    use std::os::fd::{AsRawFd, BorrowedFd};
    use std::thread;

    pub fn run() -> Result<()> {
        let (rows, cols) = query_winsize().unwrap_or((24, 80));
        let mut current_size = TerminalSize {
            rows,
            cols,
            cell_width: 0,
            cell_height: 0,
        };

        let unix_builder = pty::unix("/bin/sh")
            .with_arg("-i")
            .set_controling_tty_enable()
            .with_size(current_size.into());

        let (mut engine, handle, events) =
            TerminalBuilder::from_unix_builder(unix_builder)
                .with_size(current_size)
                .build()?;

        let mut stdin = io::stdin();
        let mut input = [0u8; 1024];
        set_nonblocking(&stdin)?;

        loop {
            match stdin.read(&mut input) {
                Ok(read) if read > 0 => {
                    handle
                        .send(TerminalRequest::WriteBytes(
                            input[..read].to_vec(),
                        ))
                        .map_err(|err| anyhow!("send input: {err:?}"))?;
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
                    handle
                        .send(TerminalRequest::Resize(new_size))
                        .map_err(|err| anyhow!("send resize: {err:?}"))?;
                }
            }

            while let Ok(event) = events.try_recv() {
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

        let mut out = io::stdout();
        write!(out, "\x1b[?25l\x1b[2J\x1b[H")?;

        let mut buf = [0u8; 4];

        for row in 0..rows {
            let mut prev_attrs: Option<RenderAttributes> = None;
            write!(out, "\x1b[{};1H\x1b[2K", row + 1)?;

            for col in 0..cols {
                let idx = row * cols + col;
                let Some(cell) = view.cells.get(idx) else {
                    break;
                };
                let attrs = RenderAttributes::from_cell(cell);
                if prev_attrs.as_ref() != Some(&attrs) {
                    write_sgr_for_attrs(&mut out, &attrs, view.colors)?;
                    prev_attrs = Some(attrs);
                }
                write_cell(&mut out, cell, &mut buf)?;
            }
        }

        write!(out, "\x1b[0m")?;

        if view.cursor.shape != otty_escape::CursorShape::Hidden {
            if let Some(cursor) = otty_libterm::surface::point_to_viewport(
                view.display_offset,
                view.cursor.point,
            ) {
                let row = cursor.line;
                let col = cursor.column.0;
                if row < rows && col < cols {
                    write!(out, "\x1b[{};{}H\x1b[?25h", row + 1, col + 1)?;
                }
            }
        }

        out.flush()?;
        Ok(())
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RenderUnderline {
        None,
        Single,
        Double,
        Curl,
        Dotted,
        Dashed,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct RenderAttributes {
        bold: bool,
        dim: bool,
        italic: bool,
        underline: RenderUnderline,
        reverse: bool,
        strike: bool,
        foreground: AnsiColor,
        background: AnsiColor,
    }

    impl RenderAttributes {
        fn from_cell(cell: &FrameCell) -> Self {
            let flags = cell.cell.flags;
            let underline = if flags.contains(Flags::DOUBLE_UNDERLINE) {
                RenderUnderline::Double
            } else if flags.contains(Flags::UNDERCURL) {
                RenderUnderline::Curl
            } else if flags.contains(Flags::DOTTED_UNDERLINE) {
                RenderUnderline::Dotted
            } else if flags.contains(Flags::DASHED_UNDERLINE) {
                RenderUnderline::Dashed
            } else if flags.contains(Flags::UNDERLINE) {
                RenderUnderline::Single
            } else {
                RenderUnderline::None
            };

            Self {
                bold: flags.intersects(
                    Flags::BOLD | Flags::BOLD_ITALIC | Flags::DIM_BOLD,
                ),
                dim: flags.intersects(Flags::DIM | Flags::DIM_BOLD),
                italic: flags.intersects(Flags::ITALIC | Flags::BOLD_ITALIC),
                underline,
                reverse: flags.contains(Flags::INVERSE),
                strike: flags.contains(Flags::STRIKEOUT),
                foreground: cell.cell.fg,
                background: cell.cell.bg,
            }
        }
    }

    fn write_sgr_for_attrs(
        out: &mut impl Write,
        attrs: &RenderAttributes,
        palette: &Colors,
    ) -> io::Result<()> {
        write!(out, "\x1b[0")?;

        if attrs.bold {
            write!(out, ";1")?;
        }
        if attrs.dim {
            write!(out, ";2")?;
        }
        if attrs.italic {
            write!(out, ";3")?;
        }
        match attrs.underline {
            RenderUnderline::Single => write!(out, ";4")?,
            RenderUnderline::Double => write!(out, ";21")?,
            RenderUnderline::Curl => write!(out, ";4:3")?,
            RenderUnderline::Dotted => write!(out, ";4:4")?,
            RenderUnderline::Dashed => write!(out, ";4:5")?,
            RenderUnderline::None => {},
        }
        if attrs.reverse {
            write!(out, ";7")?;
        }
        if attrs.strike {
            write!(out, ";9")?;
        }

        write_color(out, attrs.foreground, palette, true)?;
        write_color(out, attrs.background, palette, false)?;

        write!(out, "m")?;
        Ok(())
    }

    fn write_color(
        out: &mut impl Write,
        color: AnsiColor,
        palette: &Colors,
        is_foreground: bool,
    ) -> io::Result<()> {
        let base = if is_foreground { 30 } else { 40 };
        let bright_base = if is_foreground { 90 } else { 100 };

        match color {
            AnsiColor::Std(std_color) => {
                if let Some(rgb) = palette[std_color] {
                    write!(
                        out,
                        ";{};2;{};{};{}",
                        base + 8,
                        rgb.r,
                        rgb.g,
                        rgb.b
                    )?;
                    return Ok(());
                }

                match std_color {
                    StdColor::Black => write!(out, ";{}", base)?,
                    StdColor::Red => write!(out, ";{}", base + 1)?,
                    StdColor::Green => write!(out, ";{}", base + 2)?,
                    StdColor::Yellow => write!(out, ";{}", base + 3)?,
                    StdColor::Blue => write!(out, ";{}", base + 4)?,
                    StdColor::Magenta => write!(out, ";{}", base + 5)?,
                    StdColor::Cyan => write!(out, ";{}", base + 6)?,
                    StdColor::White => write!(out, ";{}", base + 7)?,
                    StdColor::BrightBlack => write!(out, ";{}", bright_base)?,
                    StdColor::BrightRed => write!(out, ";{}", bright_base + 1)?,
                    StdColor::BrightGreen => {
                        write!(out, ";{}", bright_base + 2)?
                    },
                    StdColor::BrightYellow => {
                        write!(out, ";{}", bright_base + 3)?
                    },
                    StdColor::BrightBlue => {
                        write!(out, ";{}", bright_base + 4)?
                    },
                    StdColor::BrightMagenta => {
                        write!(out, ";{}", bright_base + 5)?
                    },
                    StdColor::BrightCyan => {
                        write!(out, ";{}", bright_base + 6)?
                    },
                    StdColor::BrightWhite => {
                        write!(out, ";{}", bright_base + 7)?
                    },
                    StdColor::Foreground
                    | StdColor::Background
                    | StdColor::BrightForeground
                    | StdColor::DimForeground => {
                        write!(out, ";{}", if is_foreground { 39 } else { 49 })?
                    },
                    StdColor::Cursor
                    | StdColor::DimBlack
                    | StdColor::DimRed
                    | StdColor::DimGreen
                    | StdColor::DimYellow
                    | StdColor::DimBlue
                    | StdColor::DimMagenta
                    | StdColor::DimCyan
                    | StdColor::DimWhite => {
                        let base_idx = match std_color {
                            StdColor::DimBlack => 0,
                            StdColor::DimRed => 1,
                            StdColor::DimGreen => 2,
                            StdColor::DimYellow => 3,
                            StdColor::DimBlue => 4,
                            StdColor::DimMagenta => 5,
                            StdColor::DimCyan => 6,
                            StdColor::DimWhite => 7,
                            _ => 0,
                        };
                        write!(out, ";{}", base + base_idx)?;
                    },
                }
            },
            AnsiColor::Indexed(idx) => {
                write!(out, ";{};5;{}", base + 8, idx)?;
            },
            AnsiColor::TrueColor(rgb) => {
                write!(out, ";{};2;{};{};{}", base + 8, rgb.r, rgb.g, rgb.b)?;
            },
        }

        Ok(())
    }

    fn write_cell(
        out: &mut impl Write,
        cell: &FrameCell,
        buf: &mut [u8; 4],
    ) -> io::Result<()> {
        let mut ch = cell.cell.c;
        let flags = cell.cell.flags;
        if flags.contains(Flags::HIDDEN)
            || flags.contains(Flags::WIDE_CHAR_SPACER)
            || flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
        {
            ch = ' ';
        }

        let encoded = ch.encode_utf8(buf);
        out.write_all(encoded.as_bytes())?;

        if let Some(extra) = cell.cell.zerowidth() {
            for zw in extra {
                let encoded = zw.encode_utf8(buf);
                out.write_all(encoded.as_bytes())?;
            }
        }

        Ok(())
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
