# 08 Palette

Status: needs-triage
Blocked by: 07

## What to do

Move `app/src/widgets/palette` into `otty-app-palette`, following "Moving a
feature" in the spec.

- Picking an entry returns `CommandPicked(AppCommand)`. The binary runs it
  through `dispatch/command.rs`, the same path as a shortcut.
- Whether a command can run is checked by the palette's `reduce` when an entry
  is picked, from data in its `Ctx`, as in the old app.
- The palette's own shortcut keeps its query; the palette opens above a
  Containers dialog and under the SSH secret prompt.

## Done when

- Opening, searching and running commands from the palette works as in the old
  app.
- `otty-app-palette` depends only on helper crates and `otty-ui-*`.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: the palette feature crate

**`otty-app-palette`** (`crates/app/features/palette`) is a copy of
`app/src/widgets/palette`; the old widget is unchanged
(`git diff --stat 4c81023 -- app` is empty). It depends on `iced` (with
`svg`), `otty-app-domain`, `otty-app-i18n`, `otty-app-slots`,
`otty-app-theme` and `otty-ui-kit`; for tests on `iced_test` and
`otty-app-testing`. Its public API:

- `Palette` with `new`, `is_open` and `query() -> Option<&str>` (what is
  typed while it is open; the binary's input checks are tested with it);
- `impl Feature` with `Ctx<'a> = PaletteCtx<'a>` (in `ctx.rs`, with
  `PaletteSession` and `PaletteLaunch`): `sessions`, `launches`,
  `pinned_modes` and `commands: CommandCtx`, the old `command_ctx`. The
  palette lists from it when it opens and on every query change, and checks
  a picked command against it at the moment it is picked
  (`AppCommand::is_available`), as in the old app;
- `impl Overlays`: the open palette as `Overlay::menu`, with its own
  transparent dismiss layer over the main area.

```rust
pub enum PaletteIntent { Open, Dismiss, FocusInput, Action(PaletteAction) }

pub enum PaletteOutcome {
    /// Give the palette input the keyboard focus (it opened, or asks again).
    FocusInput,
    /// An open tab was picked: switch to it.
    SessionPicked { tab_id: u64 },
    /// A launch was picked: run it, as the New session menu does.
    LaunchPicked { path: Vec<String> },
    /// An app command was picked and can run now: run it, as its shortcut would.
    CommandPicked(AppCommand),
    /// The palette gave up the keyboard and nothing it ran takes it: in
    /// Terminal mode the active pane gets it back.
    KeyboardReleased,
}
```

`PaletteEffect`, the `PaletteEvent` wrapper, the `Task::done` of the reducer
and its `operation::focus` are gone. The decision the old
`effects/palette.rs` made (a launch, the panel toggles and Terminal mode
give the active terminal its focus back; other commands do not) is now in
`reduce`: such a pick returns `[LaunchPicked | CommandPicked, KeyboardReleased]`,
Esc and an outside click return `[KeyboardReleased]`, and a command that can
no longer run only closes the palette and returns nothing, as before. The
copied reducer and view tests were rewritten to check the returned outcomes
and run first against the copied `Task` reducer (10 build errors: no `==`,
no `is_empty` and no iterator on a `Task`), then `reduce` changed to return
`Vec<PaletteOutcome>`. New in the feature: which picked commands release the
keyboard, the menu overlay, the query getter.

**How the binary wires it:**

- `AppEvent::Palette(PaletteIntent)` and `App::palette`;
- `dispatch/palette.rs`: builds `PaletteCtx` (Terminal and the pinned modes
  from the modes widget, `command_ctx(app)`; no sessions and no launches
  yet), calls `reduce` and reads each outcome with one call:
  `FocusInput` → `operation::focus(input_id())`, `CommandPicked(command)` →
  `dispatch::command::deliver`, the same path as a shortcut, in the same
  update; `SessionPicked`, `LaunchPicked` and `KeyboardReleased` do nothing
  until their features move in (notes on the arms);
- `dispatch/command.rs`: `AppCommand::OpenPalette` → `PaletteIntent::Open`,
  so the shortcut and the title bar search open it;
- `guards.rs`: `Overlay::Palette`, a menu, Esc → `Dismiss`; in `top_overlay`
  it lies under the notifications list and over the All modes menu and the
  header menus; under a menu its own input (`AppEvent::Palette(_)`) passes;
  `command_ctx` is documented as what the palette reads too;
- `update.rs`: the palette shortcut under the open palette goes straight to
  `dispatch` (it keeps the query and focuses the input again, owner Q1 of
  the old app); `close_all_menus` dismisses the palette;
- `view.rs`: the palette layer between the modes menu and the notifications
  list, in the main area. A Containers dialog lies in the mode overlays
  (layer 2), so the palette is drawn over it; the SSH secret prompt
  (issue 12) goes over it. The resize grips stay on while the palette is
  open, as in the old app;
- `testing/window.rs` got `open_palette`, `testing/task_outputs.rs` got
  `focuses_widget(task, id)` (`focuses_nothing` uses it).

**Gaps of issue 05 resolved:** the palette layer, `AppCommand::OpenPalette`,
the palette shortcut under the open palette in `filter_input`, closing it in
`close_all_menus`, `top_overlay`. From issue 06: the palette entry that
opens the notifications list now works (`CommandPicked(OpenNotifications)`).
None of them had a `dead_code` mark, so no mark was removed. No "issue 08"
note is left in `crates/app/`; the domain doc of `AppCommand::is_available`
now names `otty-app-palette`.

**Left for later issues:**

- 09 settings: the dialog check; the old test
  `a_dialog_ignores_shortcuts_app_buttons_and_keys` lists the palette
  shortcut, the title bar search and `PaletteIntent::Open` among what a
  dialog ignores; Settings opens from the palette entry through
  `dispatch/command.rs`.
- 10 tabs: `PaletteCtx::sessions` from the tabs (the old `palette_ctx` and
  its test `the_palette_lists_sessions_by_machine_and_launches_in_tree_order`),
  `SessionPicked` → `TabsIntent::ActivateTab`, `has_active_tab` in
  `command_ctx`. The ctx then borrows from the tabs, so `palette_ctx` is
  built from the fields of `App` next to `app.palette` (note on it).
- 11 terminal: `KeyboardReleased` → focus the active pane;
  `has_focused_pane`; keys under the open palette kept out of the terminal;
  the old tests that the palette offers both splits with a terminal tab and
  hides the commands of the hidden tab outside Terminal mode.
- 12 quick launch: `PaletteCtx::launches`, `LaunchPicked` →
  `QuickLaunchIntent::LaunchNode`, the SSH prompt layer over the palette
  (old `the_secret_prompt_lies_over_an_open_menu` lists the palette).

**Tests.** Copied and ported: the reducer, items and view tests of the
widget; from the old binary the palette in the overlay rules, Esc, the
palette's own input under a menu, the header buttons, window buttons and
shortcuts under the open palette, the palette shortcut keeping the query,
the pinned modes in the ctx, the palette over a Containers dialog (now: over
the mode overlays in either mode) and the list over the palette. New: a
picked command, the notifications entry and a mode entry run in the same
update, a command that can not run is not listed, opening focuses the
input. `otty-app-palette` has 37 tests and 2 doc tests, `otty-app` 376 tests
(353 before). The whole workspace runs 4404 tests, all passing.

**Checks.** `cargo tree -p otty-app-palette -e normal | grep notify` prints
nothing. `tokio` comes only through `otty-ui-term` → `otty-app-theme`
(`cargo tree -i tokio`), as for the other features; no direct `tokio`, and
the only app crates in its tree are `otty-app-domain`, `-i18n`, `-slots` and
`-theme`. A planted `std::fs::read` in the reducer failed clippy with `use of
a disallowed method std::fs::read` and the note `a feature touches no files;
the binary does it in effects/`; it was removed. `cargo +nightly fmt --all --
--check`, `cargo clippy --workspace --all-targets --all-features -- -D
warnings`, `cargo deny check` (only the 3 known yanked-crate warnings),
`cargo test --workspace --all-features` and `cargo build -p otty-app` pass.
`cargo llvm-cov` was not run, as asked. The window was not opened (no
display for the agent).

**For the owner to decide.**

- Which picks give the active terminal its focus back is now a rule in the
  palette's `reduce` (`KeyboardReleased`), copied from the old
  `effects/palette.rs`. Terminal mode will also focus the pane on
  `ModeChanged(Terminal)` (issue 11), so the palette's release for
  `SwitchMode(Terminal)` may then be dropped.
- A command picked but no longer runnable closes the palette and releases
  nothing, as in the old app, so no widget has the keyboard after it.
- The resize grips stay usable while the palette is open (old app); every
  other menu turns them off.
- `PaletteCtx`, `PaletteSession` and `PaletteLaunch` have `pub` fields: they
  are plain input bags that cross the crate boundary (AGENTS.md exception).
