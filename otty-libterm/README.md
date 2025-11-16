# otty-libterm

High-level terminal runtime glue for the OTTY workspace.

`otty-libterm` connects three lower-level crates:

- [`otty-pty`](../otty-pty) – spawns and manages PTY / SSH sessions.
- [`otty-escape`](../otty-escape) – parses terminal escape sequences into semantic actions.
- [`otty-surface`](../otty-surface) – maintains an in-memory terminal surface (screen model).

Together they form a reusable building block for terminal front-ends and UI toolkits.

> **Status**: Work in progress. APIs may evolve while the rest of OTTY stabilizes.

## Architecture

At a high level, data flows through `otty-libterm` like this:

```text
user input -> TerminalRequest::Write
           -> PTY Session (otty-pty)
           -> EscapeParser (otty-escape)
           -> SurfaceActor (otty-surface)
           -> TerminalEvent::SurfaceChanged / snapshots for UI
```

## Quick start

The easiest way to see `otty-libterm` in action is to look at the example in
`otty-libterm/examples/unix_shell.rs`, which wires a Unix PTY, a parser and a basic surface together.

Conceptually, the flow looks like:

```rust
use otty_libterm::{
    escape,
    pty::{self, PtySize},
    surface::{Dimensions, Surface, SurfaceConfig},
    Runtime,
    Terminal,
    TerminalClient,
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
                }
                ChildExit { status } => {
                    println!("Child process exited with: {status}");
                }
                _ => {}
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
```

This example matches `examples/simple.rs` and is intended to be copy-pasted into a standalone project (on Unix platforms).

## Integrating with a UI

`otty-libterm` does not render pixels. Instead, it gives you a snapshot of the
terminal state and events that describe what changed.

To render:

- Implement `TerminalClient` for a type that:
  - receives `TerminalEvent::SurfaceChanged { snapshot }`,
  - walks the `snapshot.surface` (from `otty-surface`) to extract cells, attributes, cursor position, etc.,
  - re-renders the view in your UI toolkit (e.g. egui, Iced, wgpu).

To send input:

- If you own the `Terminal` directly (no `Runtime`), you can call `terminal.process_request(TerminalRequest::...)` on the same thread
- If the `Terminal` is driven by `Runtime::run`, keep a `RuntimeRequestProxy` in your UI thread and:
  - translate key presses, mouse events or higher-level actions into `TerminalRequest` values,
  - send them via `request_proxy.send(TerminalRequest::...)` so the runtime loop can wake up and apply them safely.

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
  - providing a clean request / event API for front-ends,
  - driving everything from a `mio`-based runtime.

- **Is not** responsible for:
  - drawing text or glyphs,
  - window management, GPU resources, or font rendering,
  - user input handling beyond turning your input into `TerminalRequest`s.

It is intended to be embedded into different terminal front-ends (TUI, GUI, web)
and reused across them.
