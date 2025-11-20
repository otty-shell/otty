# otty-libterm

High-level terminal core for the OTTY workspace.

`otty-libterm` wires three lower-level crates together:

- [`otty-pty`](../otty-pty) – spawns and manages PTY / SSH sessions.
- [`otty-escape`](../otty-escape) – parses terminal escape sequences into semantic actions.
- [`otty-surface`](../otty-surface) – maintains an in-memory terminal surface (screen model).

The [`TerminalEngine`] owns a PTY session, escape parser and surface. It exposes a small API around input requests, readiness hooks (`on_readable` / `on_writable` / `tick`) and emits owned frames through [`TerminalEvent`]s.

> **Status**: Work in progress. APIs may evolve while the rest of OTTY stabilizes.

## Architecture

At a high level, data flows through `otty-libterm` like this:

```text
user input -> TerminalRequest::WriteBytes
           -> PTY Session (otty-pty)
           -> EscapeParser (otty-escape)
           -> SurfaceActor (otty-surface)
           -> TerminalEvent::Frame(FrameOwned) for UI consumption
```

## Quick start

The easiest way to see `otty-libterm` in action is to look at the example in
`otty-libterm/examples/simple.rs`, which wires a Unix PTY, a parser and a basic surface together.

Conceptually, the flow looks like:

```rust
use std::thread;
use std::time::Duration;

use otty_libterm::{
    escape,
    pty::{self, PtySize},
    surface::{Dimensions, Surface, SurfaceConfig},
    TerminalEngine,
    TerminalEvent,
    TerminalOptions,
    TerminalRequest,
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

    // 3. Create an escape parser and terminal engine.
    let parser: escape::Parser<escape::vte::Parser> = Default::default();
    let options = TerminalOptions::default();
    let mut terminal =
        TerminalEngine::new(session, parser, surface, options)?;

    // 4. Enqueue a couple of commands to the shell.
    terminal.queue_request(TerminalRequest::WriteBytes(
        b"echo 'hello from otty-libterm'\n".to_vec(),
    ))?;
    terminal.queue_request(TerminalRequest::WriteBytes(
        b"exit\n".to_vec(),
    ))?;

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
                    println!(
                        "frame ready with {} cells",
                        frame.view().visible_cell_count
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
```

## Integrating with a UI

`otty-libterm` does not render pixels. Instead, it keeps an in-memory surface and emits owned frames whenever parsing mutates that surface.

To render:

- Drive `on_readable`, `on_writable`, and `tick` based on your event loop (mio, tokio, custom).
- Drain events from `next_event()`. For `TerminalEvent::Frame { frame }`, call `frame.view()` to inspect cells, cursor, modes, and damage.
- React to other events such as `ChildExit`, `TitleChanged`, `Bell`, or cursor updates as needed.

To send input:

- Translate user input into raw bytes.
- Call `queue_request(TerminalRequest::WriteBytes(bytes))`.
- Use `has_pending_output()` to decide when to request writable readiness.

## Configuration

- `TerminalOptions`
  - Currently exposes `read_buffer_capacity`, a hint for how much PTY output to read per syscall.
  - Can be tuned for workloads with very high throughput or constrained memory.

- `TerminalSize`
  - Describes the grid geometry (rows / columns) and cell size in pixels.
  - Implements `otty_surface::Dimensions` to stay consistent with surface APIs.
  - Converts directly into `otty_pty::PtySize` for PTY resize requests.

## What otty-libterm is and is not

`otty-libterm`:

- **Is** responsible for:
  - wiring PTY I/O into the escape parser and surface model,
  - providing a clean request / event API for front-ends.

- **Is not** responsible for:
  - drawing text or glyphs,
  - window management, GPU resources, or font rendering,
  - user input handling beyond turning your input into `TerminalRequest`s.

A minimal `mio` runtime driver is still present as a stub for future integration tasks, but the core engine no longer depends on it.
