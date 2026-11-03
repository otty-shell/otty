# 05 — Mode shortcuts and palette actions

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 04
Spec: [../spec.md](../spec.md)

## What

Shortcuts and palette items for modes (spec Decision 6). Tests first.

- Research first, note the answer in Comments: can `ShortcutKeys` and the terminal bindings match a physical key (`Digit1`…`Digit5`) instead of a character? With Shift held, Shift 1 is `!` on US layouts and other characters elsewhere. Extend the keymap so both routes match digits by physical key.
- `AppShortcut` gains: Terminal mode, Pinned mode 1–4, All modes menu. Linux: Ctrl Shift 1–5, Ctrl Shift M. macOS: Cmd 1–5, Cmd Shift M.
- Pinned mode N with fewer than N pinned does nothing.
- Palette: "Switch to Terminal mode" and "Switch to Docker mode" for every mode, pinned or not, with the shortcut label when the mode has one.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Keymap table tests still pass with the new rows: no clashes; Ctrl plus a digit or `m` without Shift is not taken on Linux.
- Shortcut handling tests for both routes (terminal focused and not); palette tests run the same intent as the switcher.
- Manual: in zsh with `cat -v`, Ctrl 1 and Ctrl M still reach the shell; Ctrl Shift 2 switches to Docker.

## Answer

Branch `worktree-agent-ae0acacf2d34ab50f`, one commit on top of `9798b42`.

### Keymap (`app/src/shortcuts.rs`)

`AppShortcut` gains six rows, appended to `ALL` (now 14):

| Shortcut | Linux | macOS | Matched by |
|---|---|---|---|
| `TerminalMode` | Ctrl Shift 1 | Cmd 1 | physical `Digit1` |
| `PinnedMode1`…`PinnedMode4` | Ctrl Shift 2–5 | Cmd 2–5 | physical `Digit2`…`Digit5` |
| `AllModesMenu` | Ctrl Shift M | Cmd Shift M | character `m` |

- `ShortcutKeys` gets `code() -> Option<Code>`: `Some` for a key matched by
  its place on the keyboard, `None` for a key matched by the character it
  types. `key()` stays the label text ("1"), so `label()` reads
  "Ctrl Shift 1" / "Cmd 1" with no change to the kit's `shortcut_text`.
  Constructors: `ShortcutKeys::new(key, modifiers)` (character) and
  `ShortcutKeys::physical(code, label, modifiers)`.
- Both routes use the same table: `terminal_bindings()` registers
  `InputKind::Physical(code)` for physical keys and `InputKind::Char` for the
  rest; `shortcut_for_key_press` compares `physical_key` for physical keys
  and the lower-cased key without modifiers for the rest (one private
  `ShortcutKeys::is_pressed_by`).
- `AppShortcut::mode_position()` (0 = Terminal, N = pinned mode N) and
  `AppShortcut::for_mode_position(position)` map shortcuts to switcher
  positions. The palette uses the second for its labels; ticket 06 can use
  it for the shortcut text of the menu rows (`pinned_rows` index).

### Handling (`app/src/events/shortcuts.rs`)

- `TerminalMode`, `PinnedMode1`…`4` → `ModesIntent::Select(mode)` where
  `mode` is `vm().pinned_rows[position]` (Terminal, then the pinned modes).
  With fewer pinned modes than N, nothing is sent. The temporary tab is not
  in `pinned_rows`, so an unpinned active mode never counts as pinned mode 1.
- `AllModesMenu` → `ModesIntent::ToggleMenu`.
- The terminal route needs nothing new: the terminal reports the action, the
  terminal workspace reducer hands every non-split shortcut to the app as
  `ShortcutPressed`, which calls the same `shortcuts::handle`.

### Terminal widget (`crates/ui/terminal`)

This crate had to change; see Comments for why. `InputKind` gains
`Physical(Code)`. `handle_keyboard_event` looks up a binding for the
physical key first; only when none matches does it go on as before (the
character binding, then the platform text). Nothing else in the crate
changes, and no default binding uses the new variant.

### Palette

- `PaletteTarget::SwitchMode(Mode)`; `PaletteCtx` gains `pinned_modes`
  (Terminal and the pinned modes in switcher order, from
  `app.widgets.modes.vm().pinned_rows`); `events::palette::palette_ctx`
  takes it as a fourth argument.
- `palette_items` adds one Actions entry per mode in `Mode::ALL` order,
  pinned or not: "Switch to Terminal mode", "Switch to Docker mode". The
  shortcut label is `for_mode_position` of the mode's position in
  `pinned_modes`, so an unpinned mode shows no shortcut.
- Running it sends `ModesIntent::Select(mode)`, the intent a switcher tab
  sends. For Terminal mode it also sends `FocusActive`, like the panel
  toggles, so the terminal gets back the keyboard focus the palette input
  took. Other modes have no terminal on screen, so they do not.

### i18n

`palette_switch_to_terminal_mode` and `palette_switch_to_docker_mode` in all
ten catalogs.

### Tests (written first, seen failing, then implemented)

| Where | New or changed | What they pin |
|---|---|---|
| `crates/ui/terminal/src/input.rs` | 4 new | a physical binding matches "1", "!" and "&" on the `1` key and reaches no pty; it needs its exact modifiers (Ctrl 1 still writes "1"); the keypad 1 does not match it; Ctrl M still sends CR next to it |
| `app/src/shortcuts.rs` | 6 new, 5 changed | labels for both platforms (spec table); digits are physical on both platforms; Shift/AZERTY characters on the `1` key still run Terminal mode; keypad 2 and an unidentified key reporting "2" do not; Ctrl 1–5 and Ctrl M run nothing on Linux; mode positions in order and back; the shell-key rule now also covers digits; the clash test compares label text plus modifiers; terminal bindings use `InputKind::Physical` for physical keys |
| `app/src/events/shortcuts.rs` | 7 new | Terminal mode; pinned mode 1 → Docker; pinned modes 2–4 with one pinned and pinned mode 1 with none do nothing; the temporary tab is skipped; menu toggle; a subscription key press reporting "@" on the `2` key switches to Docker |
| `app/src/events/terminal_workspace.rs` | 1 new | terminal route: `ShortcutPressed { PinnedMode1 }` → `Select(Docker)` |
| `app/src/widgets/terminal_workspace/reducer.rs` | 1 changed | the six mode shortcuts reported by a terminal go to the app |
| `app/src/widgets/palette/items.rs` | 1 new, 3 changed | mode entries with their labels; an unpinned mode has no shortcut |
| `app/src/events/palette.rs` | 3 new, 1 changed | the ctx carries the pinned modes; the open palette lists an unpinned Docker without a shortcut; running a mode entry sends `Select` (plus `FocusActive` for Terminal) |

### Not done here

- The `expect(dead_code)` on `pub(crate) mod modes;` stays: the view still
  does not use the menu rows, pin buttons and other parts, so the
  expectation is still met. Ticket 06 removes it.
- `ModeChanged` still does nothing (ticket 06). Ctrl Shift M toggles the
  menu state, but the menu is not drawn until ticket 06.

## Comments

### Research: matching digits by physical key on both routes

- **What iced reports.** iced 0.14 (`iced_winit` `conversion.rs`) builds each
  `KeyPressed` from winit with `key` = `key_without_modifiers()`,
  `modified_key` = the logical key, `physical_key` = the scancode as a
  `Code` (`Digit1`…`Digit5` for the digit row, `Numpad1`… for the keypad),
  and `text` = `text_with_all_modifiers()`. So `physical_key` is the only
  field that is the same for Ctrl Shift 1 on every layout. `key` is "1" on
  a US layout but "&" on a French AZERTY layout (its unshifted digit row
  types symbols), and `text` changes with Shift ("!", "@"). iced's own
  `Key::to_latin` does not help: it returns any single Latin character
  as is, so "&" stays "&".
- **App subscription route.** `shortcut_for_key_press` already gets the
  whole `keyboard::Event`, so it could compare `physical_key` with no other
  change.
- **Terminal route.** It could not. The terminal widget matched bindings
  only by `InputKind::Char(key)` or `InputKind::KeyCode(Named)`, and when no
  binding matched it wrote `text` to the shell and captured the key. So
  Ctrl Shift 1 on AZERTY ("&") wrote to the shell, and the app never saw it:
  `iced::keyboard::listen` only gets keys no widget captured, and
  `iced::event::listen_with` sees them only after the terminal has already
  written to the shell. Registering every character a digit key can type on
  every layout is not possible. Wrapping the terminal widget in an app
  widget that grabs keys first would be a new layer only to work around a
  missing match kind. So `crates/ui/terminal` gets one new binding kind,
  `InputKind::Physical(Code)`, looked up before the character bindings. It
  is a small, general binding feature, not app logic: the crate still knows
  nothing about modes.
- **Answer:** yes, both routes now match `Digit1`…`Digit5` by physical key,
  with exact modifiers; the keypad digits do not count. Ctrl Shift M stays
  a character match like the other letter shortcuts, so like them it does
  not match with a non-Latin layout (noted in workspace-layout ticket 06).

### Behaviour changes inside a focused terminal

- Ctrl Shift 2 used to send NUL (`^@`) through the terminal's own vt100
  binding for "2" with Ctrl Shift, and Ctrl Shift M sent CR. Both now run
  app shortcuts (pinned mode 1, All modes menu), as Decision 6 asks. Ctrl M
  (CR) is unchanged; Ctrl 1–5 without Shift have no binding and still write
  what the platform reports.

### Checks

- `cargo +nightly fmt`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace --all-features`
  (1282 passed, 0 failed) and `cargo deny check` (advisories, bans,
  licenses, sources ok; the three old `yanked` warnings remain) pass. No new
  dependencies.
- Coverage (`cargo llvm-cov --workspace --all-features`, lines): base
  `9798b42` 80.70% → this ticket 80.92%.
- Manual check not run: the agent cannot open the GUI and press keys. To
  check: run `cat -v` in zsh inside OTTY and press Ctrl 1 (prints `1` or
  nothing, as the platform reports it) and Ctrl M (ends the line, CR);
  neither may switch modes. Ctrl Shift 2 should switch to Docker (the
  active mode is not drawn until ticket 06, so for now only a log or a
  debugger shows it); also try it on an AZERTY layout, and Cmd 2 on macOS.
  On GNOME, check that IBus does not take Ctrl Shift with a digit.

### Palette commands of a hidden tab (2026-11-03)

Spec Decision 5 now says what the palette does with a hidden tab.

- `PaletteCtx` gains `active_mode`. Outside Terminal mode `palette_items`
  leaves out Split pane, Close session and New folder, so the list has no entry
  that does nothing.
- `PaletteTarget::SplitPane` and `PaletteTarget::NewFolder` do nothing outside
  Terminal mode, like `close_active_tab`. All three guards stay: a mode shortcut
  can switch the mode while the palette is open, and the open list keeps its
  entries until the next keystroke rebuilds it.
- Clicks and Enter go by the place among the matches of the list on screen, so
  hiding entries moves nothing out of step. Tests in `palette/reducer.rs` run
  clicks, arrows and filtering in Docker mode.

"New folder" in Docker mode used to show nothing: it set up the name field in
the Launches panel, which is not on screen, and left the Launches section
unfolded, and a closed left panel or focus mode opened or left. The owner
approved hiding it, so it is now hidden and guarded as above.

Read but left as they are:

- The panel toggles, Launch and dismiss send `FocusActive` in Docker mode too.
  The hidden terminal is not on screen, so this only clears keyboard focus,
  which the palette input already held. No visible effect of its own. Docker
  mode does not get back the field that had focus before the palette opened.
