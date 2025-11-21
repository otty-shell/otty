# otty-libterm API redesign (clean slate)

No backward-compat constraints; optimize for embedders building terminal emulators of any complexity.

## Design goals
- Ergonomic: renderers receive owned, `'static`, shareable frames; inputs are explicit bytes encoded by the front-end.
- Modular: keep trait boundaries for session/escape/surface; allow swapping drivers (mio/tokio/manual).
- Render-friendly: expose damage/dirty regions and compact data for partial repaints.
- Simple wiring: builders/presets hide boilerplate for common PTY/SSH/Surface setups.

## Proposed public surface (vNext)
- Core types:
  - `TerminalEngine<S, P, Surf>`: owns session, parser, surface; pure core with no runtime baked in.
  - `TerminalHandle`: cloneable sender for requests; callable from UI threads/tasks.
  - `TerminalEvents`: receiver (sync + async variants) yielding `'static` events.
  - `TerminalEvent`: `Frame(SnapshotOwned)`, `ChildExit`, `TitleChanged/Reset`, `Bell`, `Cursor{Shape,Style,Icon}`, `Hyperlink`, `ModeChanged`, `Metrics` (optional stats).
  - `TerminalRequest`: `Resize(TerminalSize)`, `ScrollDisplay`, `Selection(Start/Update)`, `Shutdown`, `WriteBytes(Bytes)` (front-end stays responsible for encoding user input to bytes).
- Frame/surface:
  - `SnapshotOwned` = `Arc<FrameData>` with `SnapshotView<'_>` accessor; includes `size`, `cursor`, `mode`, `display_offset`, `damage: DamageSet`, and an iterator over visible cells (or spans/runs).
  - `DamageSet`: list of dirty regions (line ranges + columns) plus flags for “full clear”/“title change”.
  - `SurfaceModel` trait (replaces `SurfaceActor + SurfaceSnapshotSource`) with owned snapshot export; default impl is `Surface`.
- Session/driver:
  - `Session` trait (read/write/resize/close + readiness tokens) stays; built-ins: `UnixSession`, `SshSession`.
  - `Driver` trait: `on_readable()`, `on_writable()`, `tick(now)`, `queue(request)`; returns `Option<Deadline>` for timers. Enables mio, tokio, or manual loops.
- Configuration:
  - `TerminalConfig`: size, buffers, scrollback, sync-mode limits, feature flags (damage on/off, span encoding vs. per-cell).
  - Builders/presets: `TerminalBuilder::unix(cmd, args)`, `::ssh(target)`, `::with_surface(surface)`, `::with_parser(parser)`, `::with_driver(DriverKind)`, producing `(engine, events, handle)`.

## Usage sketches

### Sync (custom poller)
```rust
// Configure engine: use default Unix PTY, with explicit size.
let (mut engine, mut events, handle) = TerminalBuilder::unix("/bin/sh")
    .with_size((80, 24))
    .build()?;

// Integrate with your poller: invoke engine when PTY is readable/writable.
poller.on_readable(|_| engine.on_readable()?);
poller.on_writable(|_| engine.on_writable()?);

// Consume terminal events and render.
while let Some(ev) = events.recv() {
    match ev {
        TerminalEvent::Frame(frame) => renderer.draw(frame.view()), // draw owned frame
        TerminalEvent::ChildExit { status } => break,               // exit when child ends
        _ => {}
    }
}

// Send raw bytes (encode input yourself).
handle.send_request(TerminalRequest::WriteBytes(b"ls\n".to_vec()))?;
```

### Tokio
```rust
// Engine + async event receiver + handle.
let (mut engine, mut events, handle) = TerminalBuilder::unix("/bin/sh").build()?;

// Push frames to UI task.
tokio::spawn(async move {
    while let Some(ev) = events.recv().await {
        if let TerminalEvent::Frame(frame) = ev {
            ui_tx.send(frame).await.unwrap();
        }
    }
});

// Drive readable/writable/tick in the async loop.
loop {
    tokio::select! {
        _ = pty_read_ready() => engine.on_readable()?,
        _ = pty_write_ready(), if engine.has_pending_output() => engine.on_writable()?,
        _ = tick() => engine.tick()?,
    }
}
```

### Damage-aware rendering
```rust
if let TerminalEvent::Frame(frame) = ev {
    for region in frame.damage().iter() {
        renderer.update(frame.view(), region);
    }
}
```

### Swap in your own modules (pty / surface / parser)
```rust
// Custom PTY session implementing the Session trait.
struct MySession { /* ... fd, ssh client, etc. ... */ }
impl Session for MySession { /* read/write/resize/close */ }
impl Pollable for MySession { /* tokens/registration */ }

// Custom surface implementing the SurfaceModel trait.
struct MySurface { /* gpu-backed grid */ }
impl SurfaceModel for MySurface {
    fn apply(&mut self, action: Action) { /* mutate GPU buffers */ }
    fn snapshot_owned(&self) -> SnapshotOwned { /* produce owned frame */ }
}

// Custom escape parser implementing EscapeParser.
struct MyParser { /* wraps otty-vte or alternate */ }
impl EscapeParser for MyParser {
    fn advance<A: EscapeActor>(&mut self, bytes: &[u8], actor: &mut A) {
        /* parse bytes and emit actions */
    }
}

// Inject your implementations via the builder.
let my_session = MySession::new(/* ... */); // your PTY backend
let my_surface = MySurface::new(/* ... */); // your renderer-backed surface
let my_parser = MyParser::default();        // your escape parser

let (mut engine, mut events, handle) = TerminalBuilder::new()
    .with_session(my_session)   // swap session
    .with_surface(my_surface)   // swap surface model
    .with_parser(my_parser)     // swap parser
    .build()?;                  // engine + events + handle
```

### Using the runtime (keep easy “just run” mode)
```rust
// Construct engine with defaults and hand it to the provided runtime.
let (engine, events, handle) = TerminalBuilder::unix("/bin/sh")
    .with_size((100, 30))
    .build()?;

// Runtime drives I/O; you only handle events and push writes.
let mut runtime = Runtime::new()?; // mio-based default runtime
std::thread::spawn({
    let mut events = events;
    move || {
        while let Some(ev) = events.recv() {
            if let TerminalEvent::Frame(frame) = ev {
                renderer.draw(frame.view()); // render frame in UI thread
            }
        }
    }
});

// Post some bytes to the child process.
handle.send_request(TerminalRequest::WriteBytes(b"echo hi\n".to_vec()))?;

// Run blocking loop until shutdown/child exit.
runtime.run(engine)?;
```

### Standalone without runtime
```rust
// Build engine and drive it manually (e.g., inside your own loop).
let (mut engine, mut events, handle) = TerminalBuilder::unix("/bin/sh").build()?;

loop {
    // Read incoming PTY data.
    engine.on_readable()?;

    // Flush pending writes if any.
    if engine.has_pending_output() {
        engine.on_writable()?;
    }

    // Periodic maintenance (sync timeout, etc.).
    engine.tick()?;

    // Drain events and render.
    while let Some(ev) = events.try_recv() {
        if let TerminalEvent::Frame(frame) = ev {
            renderer.draw(frame.view());
        }
    }

    // Exit condition (your choice), or break when child exits.
}
```

## Implementation roadmap (breaking-friendly)

1) Core refactor:
   - Rename `Terminal` → `TerminalEngine`; strip runtime glue out of the core.
   - Remove `TerminalClient` callback storage; core emits events only via channels.
   - Replace `TerminalSnapshot` with owned `SnapshotOwned`/`SnapshotView`.

2) Surface + damage:
   - Extend `otty-surface` to track dirty regions and emit owned snapshots/spans.
   - Replace `SurfaceActor + SurfaceSnapshotSource` with `SurfaceModel` supporting `snapshot_owned()` and damage export.

3) Channel plumbing:
   - Introduce `TerminalEvents` (sync + async) and `TerminalHandle`; wire engine to push events into channels.
   - Ensure `SnapshotOwned` is `'static` and cheaply cloneable (`Arc`).

4) Drivers/runtimes:
   - Define a `Driver` interface on top of `TerminalEngine` (`on_readable/on_writable/tick/queue`).
   - Port the existing mio `Runtime` to the driver; add a tokio driver example and a manual-loop helper.
   - Document: “use Runtime if you don’t want to manage readiness; use driver hooks for custom loops.”

5) Requests/input handling:
   - Keep `TerminalRequest::WriteBytes` as the single input path; document encoding responsibilities for front-ends.
   - Add a light helper (optional) to batch writes and flush pending output safely.

6) Builders/presets + docs:
   - Ship `TerminalBuilder` with unix/ssh presets, surface/parser defaults, config knobs (size, buffers, damage options).
   - Provide guided examples for: runtime-driven, manual loop, tokio, custom modules, damage-aware rendering.
   - Add a “when to choose runtime vs manual driver” note.

7) Validation + quality:
   - Add tests for owned frame export + damage correctness + channel delivery order.
   - Add benchmarks/fuzzing for surface snapshot/damage overhead and parser-to-surface pipe.
   - Optional metrics/tracing hooks for throughput/latency; configurable sync-mode limits.
