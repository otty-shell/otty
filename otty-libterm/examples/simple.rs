use otty_libterm::{
    Runtime, Terminal, TerminalClient, TerminalEvent, TerminalOptions,
    TerminalRequest, escape,
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
    let mut terminal = Terminal::new(session, surface, parser, options)?;

    // 4. Attach a minimal event client: log when the child exits.
    struct SimpleClient;

    impl TerminalClient for SimpleClient {
        fn handle_event(
            &mut self,
            event: TerminalEvent,
        ) -> otty_libterm::Result<()> {
            use TerminalEvent::*;

            match event {
                SurfaceChanged { .. } => {
                    println!("surface was changed need render!")
                },
                ChildExit { status } => {
                    println!("Child process exited with: {status}");
                },
                _ => {},
            }

            Ok(())
        }
    }

    terminal.set_event_client(SimpleClient);

    // 5. Create a runtime and a proxy for sending requests.
    let mut runtime = Runtime::new()?;
    let proxy = runtime.proxy();

    // 6. Send a couple of commands to the shell, then run the runtime loop.
    proxy.send(TerminalRequest::Write(
        b"echo 'hello from otty-libterm'\n".to_vec(),
    ))?;
    proxy.send(TerminalRequest::Write(b"exit\n".to_vec()))?;

    // 7. Drive the runtime until the child process exits.
    runtime.run(terminal, ())?;

    Ok(())
}
