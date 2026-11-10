# 18 Modes as a feature crate

Status: needs-triage
Blocked by: 17

## Why

`modes` was kept in the binary as part of the frame, but it is a feature: it
imports nothing from the binary, draws the mode switcher and the All modes
menu from its own state, owns its own I/O (the pinned modes file) and decides
which mode is open. It gathers no buttons of other features. `chrome` and
`sidebar` stay the frame, because they compose parts of several components.

## What to do

Follow "Moving a feature" in the spec, with the layout of issue 17:

```
crates/app/features/modes/
  core/   otty-app-modes      state, reduce, the switcher and the menu
  io/     otty-app-modes-io   reading and writing the pinned modes file
```

- Move `crates/app/otty/src/modes/` into `core`. The switcher is the
  feature's own method (`mode_switcher(theme)`): it has one place, above the
  left panel content, like the containers engine tree. The All modes menu
  goes through `Overlays` as a menu.
- Move `crates/app/otty/src/effects/modes.rs` into `io`.
- The binary keeps the orchestration: it places the switcher above the left
  panel content, and on `ModeChanged` it tells the terminal and containers
  and picks the screen.
- The `Mode` type stays in `otty-app-domain`.

After this the frame in the binary is `chrome` and `sidebar` only, and the
binary's own I/O is the host name.

## Done when

- `otty-app-modes` depends on no other feature and does no I/O; the pinned
  modes file is read and written by `otty-app-modes-io`.
- Switching modes, pinning and the All modes menu work as before.
- fmt, clippy, `cargo deny check` and the tests pass, and their results are
  reported.

## Comments
