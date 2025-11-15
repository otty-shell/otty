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
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io::{self, Read, Write};
    use std::mem::MaybeUninit;
    use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::thread;

    use anyhow::{Context, Result};
    use nix::fcntl::{FcntlArg, OFlag, fcntl};
    use nix::libc;
    use nix::sys::termios::{self, SetArg};
    use otty_libterm::TerminalSnapshot;
    use otty_libterm::{
        LibTermError, Runtime, RuntimeHooks, RuntimeRequestProxy, Terminal,
        TerminalClient, TerminalEvent, TerminalOptions, TerminalRequest,
        TerminalSize,
        escape::{self, Color, StdColor},
        pty::{self, PtySize},
        surface::{
            Cell, Dimensions, Flags, Surface, SurfaceConfig, point_to_viewport,
        },
    };
    use signal_hook::consts::signal::SIGWINCH;

    pub fn run() -> Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let stdin_fd = stdin.as_raw_fd();
        let stdout_fd = stdout.as_raw_fd();

        let raw_guard = RawModeGuard::enable(stdin_fd)
            .context("failed to enable raw mode")?;
        let nonblocking_guard = NonBlockingGuard::set(stdin_fd)
            .context("failed to toggle non-blocking mode")?;

        let (rows, cols) = query_winsize(stdout_fd)
            .context("failed to query terminal size")?;

        let pty_size = PtySize {
            rows,
            cols,
            cell_width: 0,
            cell_height: 0,
        };

        let session = pty::unix("/bin/sh")
            .with_arg("-i")
            .with_size(pty_size)
            .set_controling_tty_enable()
            .spawn()
            .context("failed to spawn shell session")?;

        let surface_dimensions = TerminalDimensions {
            columns: cols as usize,
            rows: rows as usize,
        };
        let surface =
            Surface::new(SurfaceConfig::default(), &surface_dimensions);
        let parser: escape::Parser<escape::vte::Parser> = Default::default();
        let options = TerminalOptions::default();

        let mut terminal = Terminal::new(session, surface, parser, options)
            .context("failed to construct terminal runtime")?;

        let mut runtime =
            Runtime::new().context("failed to create terminal runtime")?;
        let runtime_handle = runtime.proxy();

        let (resize_tx, resize_rx) = mpsc::channel();
        thread::spawn(move || {
            if let Ok(mut signals) =
                signal_hook::iterator::Signals::new([SIGWINCH])
            {
                for _ in &mut signals {
                    if resize_tx.send(()).is_err() {
                        break;
                    }
                }
            }
        });

        let shared_state = Rc::new(RefCell::new(
            ShellState::new((rows, cols)).context("failed to configure tty")?,
        ));

        let poll_hooks = ShellPollHooks::new(
            runtime_handle,
            resize_rx,
            raw_guard,
            nonblocking_guard,
            shared_state.clone(),
        );

        let event_handler = ShellEventHandler::new(shared_state);
        terminal.set_event_client(event_handler);

        runtime.run(terminal, poll_hooks)?;
        Ok(())
    }

    type ShellTerminal = Terminal<
        pty::UnixSession,
        escape::Parser<escape::vte::Parser>,
        Surface,
    >;

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

    struct ShellState {
        pending_input: VecDeque<u8>,
        screen: Screen,
        size: (u16, u16),
        stdin_closed: bool,
    }

    impl ShellState {
        fn new(size: (u16, u16)) -> io::Result<Self> {
            Ok(Self {
                pending_input: VecDeque::new(),
                screen: Screen::new()?,
                size,
                stdin_closed: false,
            })
        }
    }

    struct ShellPollHooks {
        runtime_proxy: RuntimeRequestProxy,
        resize_rx: mpsc::Receiver<()>,
        state: Rc<RefCell<ShellState>>,
        _raw_guard: RawModeGuard,
        _nonblocking_guard: NonBlockingGuard,
    }

    impl ShellPollHooks {
        fn new(
            runtime_proxy: RuntimeRequestProxy,
            resize_rx: mpsc::Receiver<()>,
            raw_guard: RawModeGuard,
            nonblocking_guard: NonBlockingGuard,
            state: Rc<RefCell<ShellState>>,
        ) -> Self {
            Self {
                runtime_proxy,
                resize_rx,
                state,
                _raw_guard: raw_guard,
                _nonblocking_guard: nonblocking_guard,
            }
        }

        fn handle_resize(&mut self) -> Result<(), LibTermError> {
            let mut resized = false;
            while self.resize_rx.try_recv().is_ok() {
                resized = true;
            }

            if !resized {
                return Ok(());
            }

            let fd = { self.state.borrow().screen.fd() };
            let (rows, cols) = query_winsize(fd).map_err(LibTermError::from)?;

            let mut state = self.state.borrow_mut();
            if (rows, cols) != state.size {
                self.runtime_proxy.send(TerminalRequest::Resize(
                    TerminalSize {
                        rows,
                        cols,
                        cell_width: 0,
                        cell_height: 0,
                    },
                ))?;
                state.size = (rows, cols);
                state.screen.clear().map_err(LibTermError::from)?;
            }

            Ok(())
        }

        fn flush_pending_input(&mut self) -> Result<(), LibTermError> {
            let mut state = self.state.borrow_mut();
            if state.pending_input.is_empty() {
                return Ok(());
            }

            let chunk: Vec<u8> = state.pending_input.drain(..).collect();
            drop(state);

            if !chunk.is_empty() {
                self.runtime_proxy.send(TerminalRequest::Write(chunk))?;
            }

            Ok(())
        }

        fn read_stdin(&mut self) -> Result<(), LibTermError> {
            if self.state.borrow().stdin_closed {
                return Ok(());
            }

            let mut buffer = [0u8; 1024];
            let mut stdin = io::stdin();

            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) => {
                        {
                            let mut state = self.state.borrow_mut();
                            if state.stdin_closed {
                                break;
                            }
                            state.stdin_closed = true;
                            state.pending_input.push_back(4);
                        }
                        break;
                    },
                    Ok(read) => {
                        self.state
                            .borrow_mut()
                            .pending_input
                            .extend(&buffer[..read]);
                    },
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    },
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                        continue;
                    },
                    Err(err) => return Err(LibTermError::Io(err)),
                }
            }

            Ok(())
        }
    }

    impl RuntimeHooks<ShellTerminal> for ShellPollHooks {
        fn before_poll(
            &mut self,
            _terminal: &mut ShellTerminal,
        ) -> Result<(), LibTermError> {
            self.handle_resize()?;
            self.flush_pending_input()?;
            self.read_stdin()?;
            self.flush_pending_input()?;
            Ok(())
        }
    }

    struct ShellEventHandler {
        state: Rc<RefCell<ShellState>>,
    }

    impl ShellEventHandler {
        fn new(state: Rc<RefCell<ShellState>>) -> Self {
            Self { state }
        }

        fn render(
            &self,
            snapshot: TerminalSnapshot,
        ) -> Result<(), LibTermError> {
            let mut state = self.state.borrow_mut();
            let size = state.size;
            render_surface(snapshot, size, state.screen.writer())
                .map_err(LibTermError::from)
        }

        fn handle_exit(
            &self,
            status: &std::process::ExitStatus,
        ) -> Result<(), LibTermError> {
            let mut state = self.state.borrow_mut();
            let out = state.screen.writer();
            let exit_repr = status
                .code()
                .map(|code| format!("{code}"))
                .unwrap_or_else(|| "terminated by signal".to_string());
            writeln!(out, "\r\nShell exited with {exit_repr}")
                .map_err(LibTermError::from)?;
            out.flush().map_err(LibTermError::from)
        }
    }

    impl TerminalClient for ShellEventHandler {
        fn handle_event(
            &mut self,
            event: TerminalEvent,
        ) -> Result<(), LibTermError> {
            match event {
                TerminalEvent::SurfaceChanged { snapshot } => {
                    self.render(snapshot)
                },
                TerminalEvent::ChildExit { status } => {
                    self.handle_exit(&status)
                },
                TerminalEvent::TitleChanged { .. }
                | TerminalEvent::ResetTitle
                | TerminalEvent::Bell
                | TerminalEvent::CursorShapeChanged { .. }
                | TerminalEvent::CursorStyleChanged { .. }
                | TerminalEvent::CursorIconChanged { .. }
                | TerminalEvent::Hyperlink { .. } => Ok(()),
            }
        }
    }

    fn render_surface(
        snapshot: TerminalSnapshot,
        viewport_size: (u16, u16),
        out: &mut impl Write,
    ) -> io::Result<()> {
        write!(out, "\x1b[?25l")?;
        let mut buf = [0u8; 4];
        let mut iter = snapshot.surface.display_iter;
        let mut prev_attrs: Option<RenderAttributes> = None;
        let mut prev_line: Option<i32> = None;
        let mut row_idx: usize = 0;
        let mut rendered_any = false;

        while let Some(indexed) = iter.next() {
            rendered_any = true;
            let line = indexed.point.line.0;

            if prev_line.map_or(true, |prev| prev != line) {
                if prev_line.is_some() {
                    row_idx += 1;
                }
                write!(out, "\x1b[{};1H\x1b[2K", row_idx + 1)?;
                if prev_attrs.is_some() {
                    write!(out, "\x1b[0m")?;
                    prev_attrs = None;
                }
                prev_line = Some(line);
            }

            let attrs = RenderAttributes::from_cell(indexed.cell);
            if prev_attrs.as_ref() != Some(&attrs) {
                write_sgr_for_attrs(out, &attrs)?;
                prev_attrs = Some(attrs.clone());
            }

            write_cell(out, indexed.cell, &mut buf)?;
        }

        if prev_attrs.is_some() {
            write!(out, "\x1b[0m")?;
        }

        let total_rows = viewport_size.0 as usize;
        let rendered_rows = if rendered_any { row_idx + 1 } else { 0 };
        if rendered_rows < total_rows {
            for extra in rendered_rows..total_rows {
                write!(out, "\x1b[{};1H\x1b[2K", extra + 1)?;
            }
        }

        if snapshot.surface.cursor.shape != escape::CursorShape::Hidden {
            if let Some(cursor) = point_to_viewport(
                snapshot.surface.display_offset,
                snapshot.surface.cursor.point,
            ) {
                write!(
                    out,
                    "\x1b[{};{}H\x1b[?25h",
                    cursor.line + 1,
                    cursor.column.0 + 1
                )?;
            }
        }

        out.flush()
    }

    fn write_cell(
        out: &mut impl Write,
        cell: &Cell,
        buf: &mut [u8; 4],
    ) -> io::Result<()> {
        let mut ch = cell.c;
        let flags = cell.flags;
        if flags.contains(Flags::HIDDEN)
            || flags.contains(Flags::WIDE_CHAR_SPACER)
            || flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
        {
            ch = ' ';
        }

        let encoded = ch.encode_utf8(buf);
        out.write_all(encoded.as_bytes())?;

        if let Some(extra) = cell.zerowidth() {
            for zw in extra {
                let encoded = zw.encode_utf8(buf);
                out.write_all(encoded.as_bytes())?;
            }
        }

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

    #[derive(Clone, Debug, PartialEq)]
    struct RenderAttributes {
        bold: bool,
        dim: bool,
        italic: bool,
        underline: RenderUnderline,
        reverse: bool,
        strike: bool,
        foreground: Color,
        background: Color,
    }

    impl Default for RenderAttributes {
        fn default() -> Self {
            Self {
                bold: false,
                dim: false,
                italic: false,
                underline: RenderUnderline::None,
                reverse: false,
                strike: false,
                foreground: Color::Std(StdColor::Foreground),
                background: Color::Std(StdColor::Background),
            }
        }
    }

    impl RenderAttributes {
        fn from_cell(cell: &Cell) -> Self {
            let flags = cell.flags;
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
                foreground: cell.fg,
                background: cell.bg,
            }
        }
    }

    fn write_sgr_for_attrs(
        out: &mut impl Write,
        attrs: &RenderAttributes,
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

        write_color(out, &attrs.foreground, true)?;
        write_color(out, &attrs.background, false)?;

        write!(out, "m")?;
        Ok(())
    }

    fn write_color(
        out: &mut impl Write,
        color: &Color,
        is_foreground: bool,
    ) -> io::Result<()> {
        let base = if is_foreground { 30 } else { 40 };
        let bright_base = if is_foreground { 90 } else { 100 };

        match color {
            Color::Std(std_color) => match std_color {
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
                StdColor::BrightGreen => write!(out, ";{}", bright_base + 2)?,
                StdColor::BrightYellow => write!(out, ";{}", bright_base + 3)?,
                StdColor::BrightBlue => write!(out, ";{}", bright_base + 4)?,
                StdColor::BrightMagenta => write!(out, ";{}", bright_base + 5)?,
                StdColor::BrightCyan => write!(out, ";{}", bright_base + 6)?,
                StdColor::BrightWhite => write!(out, ";{}", bright_base + 7)?,
                StdColor::Foreground
                | StdColor::Background
                | StdColor::BrightForeground
                | StdColor::DimForeground => {
                    write!(out, ";{}", if is_foreground { 39 } else { 49 })?
                },
                _ => {},
            },
            Color::Indexed(idx) => {
                write!(out, ";{};5;{}", base + 8, idx)?;
            },
            Color::TrueColor(rgb) => {
                write!(out, ";{};2;{};{};{}", base + 8, rgb.r, rgb.g, rgb.b)?;
            },
        }

        Ok(())
    }

    fn query_winsize(fd: RawFd) -> io::Result<(u16, u16)> {
        let mut winsize = MaybeUninit::<libc::winsize>::zeroed();
        let res =
            unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, winsize.as_mut_ptr()) };

        if res == -1 {
            return Err(io::Error::last_os_error());
        }

        let winsize = unsafe { winsize.assume_init() };
        let rows = if winsize.ws_row == 0 {
            24
        } else {
            winsize.ws_row
        };
        let cols = if winsize.ws_col == 0 {
            80
        } else {
            winsize.ws_col
        };
        Ok((rows, cols))
    }

    struct RawModeGuard {
        fd: RawFd,
        original: termios::Termios,
    }

    impl RawModeGuard {
        fn enable(fd: RawFd) -> io::Result<Self> {
            let original =
                termios::tcgetattr(unsafe { BorrowedFd::borrow_raw(fd) })?;
            let mut raw = original.clone();
            termios::cfmakeraw(&mut raw);
            termios::tcsetattr(
                unsafe { BorrowedFd::borrow_raw(fd) },
                SetArg::TCSANOW,
                &raw,
            )?;
            Ok(Self { fd, original })
        }
    }

    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            let _ = termios::tcsetattr(
                unsafe { BorrowedFd::borrow_raw(self.fd) },
                SetArg::TCSANOW,
                &self.original,
            );
        }
    }

    struct NonBlockingGuard {
        fd: RawFd,
        original: OFlag,
    }

    impl NonBlockingGuard {
        fn set(fd: RawFd) -> io::Result<Self> {
            let flags = OFlag::from_bits_truncate(fcntl(
                unsafe { BorrowedFd::borrow_raw(fd) },
                FcntlArg::F_GETFL,
            )?);
            let new_flags = flags | OFlag::O_NONBLOCK;
            fcntl(
                unsafe { BorrowedFd::borrow_raw(fd) },
                FcntlArg::F_SETFL(new_flags),
            )?;
            Ok(Self {
                fd,
                original: flags,
            })
        }
    }

    impl Drop for NonBlockingGuard {
        fn drop(&mut self) {
            let _ = fcntl(
                unsafe { BorrowedFd::borrow_raw(self.fd) },
                FcntlArg::F_SETFL(self.original),
            );
        }
    }

    struct Screen {
        stdout: io::Stdout,
    }

    impl Screen {
        fn new() -> io::Result<Self> {
            let mut stdout = io::stdout();
            write!(stdout, "\x1b[2J\x1b[H\x1b[?25l")?;
            stdout.flush()?;
            Ok(Self { stdout })
        }

        fn writer(&mut self) -> &mut io::Stdout {
            &mut self.stdout
        }

        fn clear(&mut self) -> io::Result<()> {
            write!(self.stdout, "\x1b[2J\x1b[H")?;
            self.stdout.flush()
        }

        fn fd(&self) -> RawFd {
            self.stdout.as_raw_fd()
        }
    }

    impl Drop for Screen {
        fn drop(&mut self) {
            let _ = write!(self.stdout, "\x1b[?25h\x1b[0m\x1b[2J\x1b[H");
            let _ = self.stdout.flush();
        }
    }
}
