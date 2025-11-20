use std::thread;
use std::time::Duration;

use otty_libterm::{
    TerminalEngine, TerminalEvent, TerminalOptions, TerminalRequest, escape,
    pty::{self, PtySize},
    surface::{Dimensions, Surface, SurfaceConfig},
};

#[cfg(not(unix))]
fn main() -> otty_libterm::Result<()> {
    eprintln!("This example is only supported on Unix platforms.");
    Ok(())
}

#[cfg(unix)]
fn main() -> otty_libterm::Result<()> {
    // 1. Spawn an interactive /bin/sh attached to a PTY.
    let pty_size = PtySize {
        rows: 24,
        cols: 80,
        cell_width: 0,
        cell_height: 0,
    };

    let session = pty::unix("/bin/sh")
        .with_arg("-i")
        .with_size(pty_size)
        .set_controling_tty_enable()
        .spawn()?;

    // 2. Create a surface for our terminal grid.
    struct SimpleDimensions {
        columns: usize,
        rows: usize,
    }

    impl Dimensions for SimpleDimensions {
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

    let surface_dimensions = SimpleDimensions {
        columns: pty_size.cols as usize,
        rows: pty_size.rows as usize,
    };

    let surface = Surface::new(SurfaceConfig::default(), &surface_dimensions);

    // 3. Create an escape parser and terminal runtime.
    let parser: escape::Parser<escape::vte::Parser> = Default::default();
    let options = TerminalOptions::default();
    let mut terminal = TerminalEngine::new(session, parser, surface, options)?;

    // 4. Enqueue a couple of commands to the shell.
    terminal.queue_request(TerminalRequest::WriteBytes(
        b"echo 'hello from otty-libterm'\n".to_vec(),
    ))?;
    terminal.queue_request(TerminalRequest::WriteBytes(b"exit\n".to_vec()))?;

    // 5. Drive the engine manually until the child process exits.
    loop {
        terminal.on_readable()?;

        if terminal.has_pending_output() {
            terminal.on_writable()?;
        }

        terminal.tick()?;

        while let Some(event) = terminal.next_event() {
            match event {
                TerminalEvent::Frame { frame } => {
                    let view = frame.view();
                    println!(
                        "frame updated: {}x{} ({} cells)",
                        view.size.columns,
                        view.size.screen_lines,
                        view.visible_cell_count
                    );
                },
                TerminalEvent::ChildExit { status } => {
                    println!("Child process exited with: {status}");
                    return Ok(());
                },
                _ => {},
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}
