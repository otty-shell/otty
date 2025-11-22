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
           -> TerminalEvent::Frame(SnapshotOwned) for UI consumption
```

## Quick start

The `TerminalBuilder` presets wire up a PTY, parser, and surface for you. The
[examples/simple.rs](./examples/simple.rs) sample uses the Unix preset and drives the engine manually:

```rust
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
```

See:
- [examples/unix_shell.rs](./examples/unix_shell.rs) for a minimal ANSI renderer.

## Integrating with a UI

`otty-libterm` does not render pixels. Instead, it keeps an in-memory surface and emits owned frames whenever parsing mutates that surface.

To render:

- Drive `on_readable`, `on_writable`, and `tick` based on your event loop (mio, tokio, custom).
- Drain events from `TerminalEvents`. For `TerminalEvent::Frame { frame }`, call `frame.view()` to inspect cells, cursor, modes, and damage.
- React to other events such as `ChildExit`, `TitleChanged`, `Bell`, or cursor updates as needed.

To send input:

- Translate user input into raw bytes (encoding is the front-end's job).
- Call `queue_request(TerminalRequest::WriteBytes(bytes))` or chunk a large payload with `TerminalHandle::send_bytes_chunked`.
- For multi-step pastes or coalescing, use `TerminalHandle::batcher()` to stage bytes and flush in safe chunks.
- Use `has_pending_output()` to decide when to request writable readiness; it reflects queued write requests and partially flushed buffers.

### Input buffering and large pastes

- Large pastes should be chunked (defaults to 4 KiB in the batcher) to keep bounded channels responsive and to let `has_pending_output()` stay accurate until everything is flushed.
- The batcher helper coalesces multiple `push()` calls and sends them as a series of `WriteBytes` requests on `flush()`, preserving any unsent data if the request channel is full.
- Higher-level input encoders (keymaps, IME, bracketed paste framing) should live above `libterm`, handing only raw bytes into `WriteBytes`.

## Runtime vs manual loops

- `build()` returns `(engine, handle, events)` for manual integration with your readiness model (mio, epoll, tokio watcher, custom loop).
- `build_with_runtime()` also hands back a mio `Runtime` and `RuntimeRequestProxy` for a turnkey blocking loop. See `examples/tokio_runtime.rs` for running that runtime from Tokio.

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

## Validation

- Tests: `cargo test --workspace` covers unit + integration.
- Benches: `cargo bench -p otty-libterm --bench engine` (Criterion). Track throughput numbers locally for regressions.
