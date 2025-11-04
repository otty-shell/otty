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
    use std::io::{self, Read, Write};
    use std::mem::MaybeUninit;
    use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use anyhow::{Context, Result};
    use mio::unix::SourceFd;
    use mio::{Events, Interest, Poll, Token};
    use nix::fcntl::{FcntlArg, OFlag, fcntl};
    use nix::libc;
    use nix::sys::termios::{self, SetArg};
    use otty_escape::Parser;
    use otty_pty::{Pollable, PtySize, Session, SessionError, unix};
    use otty_surface::{Surface, SurfaceConfig};
    use signal_hook::consts::signal::SIGWINCH;

    const PTY_IO: Token = Token(0);
    const PTY_CHILD: Token = Token(1);
    const STDIN_TOKEN: Token = Token(2);

    pub fn run() -> Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let stdin_fd = stdin.as_raw_fd();
        let stdout_fd = stdout.as_raw_fd();

        let _raw_guard = RawModeGuard::enable(stdin_fd)?;
        let _stdin_nonblocking = NonBlockingGuard::set(stdin_fd)?;

        let (rows, cols) = query_winsize(stdout_fd)
            .context("failed to query terminal size")?;
        let mut surface = Surface::new(SurfaceConfig {
            columns: cols as usize,
            rows: rows as usize,
        });
        let mut parser = Parser::new();

        let mut session = unix("/bin/sh")
            .with_arg("-i")
            .with_size(PtySize {
                rows,
                cols,
                cell_width: 0,
                cell_height: 0,
            })
            .spawn()
            .context("failed to spawn shell session")?;

        let mut poll = Poll::new().context("failed to create poll instance")?;
        session
            .register(poll.registry(), Interest::READABLE, PTY_IO, PTY_CHILD)
            .context("failed to register PTY with poll")?;

        let mut stdin_source = SourceFd(&stdin_fd);
        poll.registry()
            .register(&mut stdin_source, STDIN_TOKEN, Interest::READABLE)
            .context("failed to register stdin with poll")?;

        let mut screen =
            Screen::new().context("failed to initialize screen")?;
        let mut needs_redraw = true;

        let (resize_tx, resize_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut signals =
                signal_hook::iterator::Signals::new([SIGWINCH]).unwrap();
            for _ in &mut signals {
                if resize_tx.send(()).is_err() {
                    break;
                }
            }
        });

        let mut events = Events::with_capacity(128);
        let mut last_render = Instant::now() - Duration::from_millis(16);
        let mut running = true;

        while running {
            let timeout = Some(Duration::from_millis(16));
            poll.poll(&mut events, timeout)
                .context("event loop poll failed")?;

            for event in events.iter() {
                match event.token() {
                    PTY_IO if event.is_readable() => {
                        if handle_pty_read(
                            &mut session,
                            &mut parser,
                            &mut surface,
                        )? {
                            needs_redraw = true;
                        }
                    },
                    PTY_CHILD if event.is_readable() => {
                        if let Some(status) =
                            session.try_get_child_exit_status()?
                        {
                            eprintln!(
                                "\r\nShell exited with status {:?}",
                                status.code()
                            );
                            running = false;
                        }
                    },
                    STDIN_TOKEN if event.is_readable() => {
                        handle_stdin(&mut session)?;
                    },
                    _ => {},
                }
            }

            while resize_rx.try_recv().is_ok() {
                let (rows, cols) = query_winsize(stdout_fd)?;
                session.resize(PtySize {
                    rows,
                    cols,
                    cell_width: 0,
                    cell_height: 0,
                })?;
                surface.resize(cols as usize, rows as usize);
                needs_redraw = true;
            }

            if needs_redraw && last_render.elapsed() > Duration::from_millis(16)
            {
                render_surface(&surface, screen.writer())?;
                last_render = Instant::now();
                needs_redraw = false;
            }
        }

        // Drain remaining output to ensure we show the final prompt/output.
        if handle_pty_read(&mut session, &mut parser, &mut surface)? {
            render_surface(&surface, screen.writer())?;
        }
        let _ = session.close();

        Ok(())
    }

    fn handle_pty_read(
        session: &mut impl Session,
        parser: &mut Parser,
        surface: &mut Surface,
    ) -> Result<bool> {
        let mut buffer = [0u8; 8192];
        let mut updated = false;
        loop {
            match session.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    parser.advance(&buffer[..read], surface);
                    updated = true;
                },
                Err(SessionError::IO(err))
                    if err.kind() == io::ErrorKind::WouldBlock =>
                {
                    break;
                },
                Err(SessionError::IO(err))
                    if err.kind() == io::ErrorKind::Interrupted =>
                {
                    continue;
                },
                Err(err) => return Err(err.into()),
            }
        }

        Ok(updated)
    }

    fn handle_stdin(session: &mut impl Session) -> Result<()> {
        let mut buffer = [0u8; 1024];
        loop {
            match io::stdin().read(&mut buffer) {
                Ok(0) => {
                    session.write(&[4])?; // Send EOF (Ctrl+D)
                    break;
                },
                Ok(read) => {
                    let mut written = 0;
                    while written < read {
                        match session.write(&buffer[written..read]) {
                            Ok(bytes) => written += bytes,
                            Err(SessionError::IO(err))
                                if err.kind() == io::ErrorKind::WouldBlock =>
                            {
                                break;
                            },
                            Err(SessionError::IO(err))
                                if err.kind() == io::ErrorKind::Interrupted =>
                            {
                                continue;
                            },
                            Err(err) => return Err(err.into()),
                        }
                    }
                },
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                    continue;
                },
                Err(err) => return Err(err.into()),
            }
        }

        Ok(())
    }

    fn render_surface(
        surface: &Surface,
        out: &mut impl Write,
    ) -> io::Result<()> {
        write!(out, "\x1b[H")?;
        let mut buf = [0u8; 4];

        for (row_idx, row) in surface.grid().rows().iter().enumerate() {
            for cell in &row.cells {
                let ch = if cell.attributes.hidden { ' ' } else { cell.ch };
                let encoded = ch.encode_utf8(&mut buf);
                out.write_all(encoded.as_bytes())?;
            }

            if row_idx + 1 != surface.grid().height() {
                out.write_all(b"\x1b[K\r\n")?;
            } else {
                out.write_all(b"\x1b[K")?;
            }
        }

        out.flush()
    }

    fn query_winsize(fd: RawFd) -> Result<(u16, u16)> {
        let mut winsize = MaybeUninit::<libc::winsize>::zeroed();
        let res =
            unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, winsize.as_mut_ptr()) };

        if res == -1 {
            return Err(io::Error::last_os_error()).context("ioctl TIOCGWINSZ");
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
        fn enable(fd: RawFd) -> Result<Self> {
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
        fn set(fd: RawFd) -> Result<Self> {
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
    }

    impl Drop for Screen {
        fn drop(&mut self) {
            let _ = write!(self.stdout, "\x1b[?25h\x1b[0m\x1b[2J\x1b[H");
            let _ = self.stdout.flush();
        }
    }
}
