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

### 2026-11-10: modes as a feature crate

**The crates.** `crates/app/otty/src/modes/` moved, with its tests, to
`crates/app/features/modes/core` as `otty-app-modes`, and
`effects/modes.rs` to `crates/app/features/modes/io` as
`otty-app-modes-io` (`pinned_file.rs`), with its own `clippy.toml` (root
thresholds, no I/O bans). The core crate falls under
`crates/app/features/clippy.toml`; a planted `std::fs::read_to_string`
failed clippy with `use of a disallowed method` and was removed. `Mode`
stays in `otty-app-domain`.

**The API of `otty-app-modes`.** `Modes` with `with_pinned(&[Mode])`,
`active()`, `is_menu_open()`, `pinned()`, `switcher_modes()` (Terminal,
then the pinned modes: what the palette needs for the mode shortcut
labels, which it read from the view model before) and
`mode_switcher(theme)`; `DEFAULT_PINNED`; `Feature` with `ModesIntent`,
`ModesOutcome` and `Ctx = ()`; `Overlays`, which gives the All modes menu
as `Overlay::menu` while it is open. The view model stays private. The
crate depends on `otty-app-domain`, `otty-app-i18n`, `otty-app-slots`,
`otty-app-theme`, `otty-ui-kit`, iced and `log`; no other feature and no
io crate.

**The API of `otty-app-modes-io`.** `load_at_start()` gives the pinned
modes and the text of a read problem; `save(pinned)` gives a
`Task<ModesIntent>` that ends in `ModesIntent::PinnedSaved`. The binary
wraps it into `AppEvent::Modes`, so a failed save still becomes a
notification through `ModesOutcome::PinnedSaveFailed`.

**The binary.** `view.rs` places `app.modes.mode_switcher(theme)` above the
left panel content and draws `app.modes.overlay(theme)` in the layer of
the header menus, as before. `dispatch/modes.rs` keeps the orchestration:
on `ModeChanged` it tells the terminal and the containers, and the frame
picks the screen from `active()`. `effects/` now holds `host_name.rs`,
`locale.rs`, `shell_integration.rs` and `startup.rs`. The frame in the
binary is `chrome` and `sidebar`.
