# 05 Binary with the window frame

Status: needs-triage
Blocked by: 01, 02, 03

## What to do

Create the binary `otty-app` in `crates/app/otty` and the helper crate
`otty-app-slots` in `crates/app/slots`.

The binary opens a window with the frame and no features yet:

- `main.rs`, `app.rs`, `Message`, `update.rs` with the input checks;
- the overlay rules from `guards.rs` (layer order, Esc, outside click, focus
  after close; see the spec, "Overlays");
- `view.rs` with the frame and the one layer order;
- `screens/terminal.rs` and `screens/containers.rs` with empty places;
- the frame widgets `chrome`, `sidebar` and `modes` as plain modules;
- `subscription.rs` and turning iced key presses into `AppCommand`;
- `dispatch/` and `effects/` with the startup I/O (shell integration files,
  host name, saved theme).

`otty-app-slots` starts with only the `Feature` trait, `Overlay` and
`MainContent`. The frame widgets implement `Feature`, which gives the trait its
first real implementations. The place traits come with the first feature that
fills each place.

The frame widgets switch to outcomes here. Their `reduce` returns
`Vec<Outcome>`, and `dispatch/` reads them. No `Task::done` goes from one widget
to another. Rewrite their copied tests to check the outcomes first, then change
the code.

`app.rs` and `update.rs` will grow with every feature. That is expected: they
are the wiring.

## Done when

- `cargo run -p otty-app` opens a window with the header, both side panels, the
  status bar and the mode switch. Switching modes, resizing the panels and focus
  mode work as in the old app.
- No `Task::done` is sent from one widget to another.
- The old `app` is unchanged and still builds and ships.
- fmt, clippy, `cargo deny check`, the tests and the coverage comparison from
  AGENTS.md are run, and their results are reported.

## Comments
