use std::thread;
use std::time::Duration;

use otty_libterm::{
    TerminalBuilder, TerminalEvent, TerminalRequest, TerminalSize, pty,
};

#[cfg(not(unix))]
fn main() -> otty_libterm::Result<()> {
    eprintln!("This example is only supported on Unix platforms.");
    Ok(())
}

#[cfg(unix)]
fn main() -> otty_libterm::Result<()> {
    // 1. Spawn an interactive /bin/sh attached to a PTY.
    let size = TerminalSize {
        rows: 24,
        cols: 80,
        cell_width: 0,
        cell_height: 0,
    };

    let unix_builder = pty::unix("/bin/sh")
        .with_arg("-i")
        .set_controling_tty_enable();

    let (mut terminal, handle, events) =
        TerminalBuilder::from_unix_builder(unix_builder)
            .with_size(size)
            .build()?;

    // 4. Send an echo first so we can render a frame before exiting.
    handle
        .send(TerminalRequest::WriteBytes(
            b"echo 'hello from otty-libterm'\n".to_vec(),
        ))
        .expect("request channel open");

    // 5. Drive the engine manually until the child process exits.
    loop {
        terminal.on_readable()?;

        if terminal.has_pending_output() {
            terminal.on_writable()?;
        }

        terminal.tick()?;

        while let Ok(event) = events.try_recv() {
            match event {
                TerminalEvent::Frame { frame } => {
                    let view = frame.view();
                    println!(
                        "frame updated: {}x{} ({} cells)",
                        view.size.columns,
                        view.size.screen_lines,
                        view.visible_cell_count
                    );

                    handle
                        .send(TerminalRequest::WriteBytes(b"exit\n".to_vec()))
                        .expect("request channel open");
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
