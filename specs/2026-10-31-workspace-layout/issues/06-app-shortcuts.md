# 06 — App shortcuts

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

One place that knows the app shortcuts of spec Decision 3, used both while a
terminal has focus and while it does not. `crates/ui/terminal` is not edited.

- Research first (note in Comments): does `iced::keyboard::listen()` still deliver a key press that the terminal widget captured? Today the app drops `AppEvent::Keyboard` (`app/src/events/mod.rs:63`) and split pane reaches the app only through `BindingAction::Action` in `terminal_workspace/shortcuts.rs`.
- Grow `shortcuts.rs` into an app keymap: an `AppShortcut` enum (palette, left panel, files panel, focus mode, settings, new terminal, close session, split pane) and one table that gives, per platform, the key and modifiers. Both routes use it: terminal bindings (`app_bindings()`) and the keyboard subscription when no terminal has focus. Make sure one key press never fires twice.
- In this ticket wire only shortcuts whose actions already exist: Settings → open settings tab, New terminal → open terminal tab, Close session → close the active tab, Split pane as today. Panel, focus and palette shortcuts are wired by tickets 07 and 17; until then they map to nothing.
- Shortcut labels for menus and `Kbd` come from the same table (like `split_vertically_shortcut_label()` today), in Linux notation "Ctrl Shift T".

## Done when

- Table tests: every shortcut has a Linux and a macOS binding; no Linux binding is Ctrl + a letter, `\` or `-` without Shift; no two shortcuts share a binding; labels match bindings.
- Manual: in zsh, `cat -v` still prints `^B`, `^K`, `^W`, `^\` (spec AC4).

## Answer

### Where the keymap lives

`app/src/shortcuts.rs` (moved up from `widgets/terminal_workspace/shortcuts.rs`,
because the subscription route, menus and later the palette use it too, not
only the terminal workspace).

- `AppShortcut`: Command palette, Left panel, Files panel, Focus mode,
  Settings, New terminal, Close session, Split pane. It is also the action type
  the terminal widgets carry (`Terminal<AppShortcut>`); it replaces
  `TerminalWorkspaceAction`.
- `AppShortcut::keys(platform)`: the one table (spec Decision 3), a single
  `match` on `(shortcut, Platform::Linux | Platform::MacOs)` that gives a
  `ShortcutKeys { key, modifiers }`. Linux uses explicit `CTRL | SHIFT`, macOS
  `LOGO` (not `Modifiers::COMMAND`, which changes with the build target), so
  both rows are tested on any machine. Windows uses the Linux row.
- `AppShortcut::label()` / `label_on(platform)`: menu and `Kbd` text from the
  same table, through the kit's `shortcut_text` ("Ctrl Shift T"; Cmd is
  written "Super", as before). The pane menu now uses
  `AppShortcut::SplitPane.label()`; `split_vertically_shortcut_label()` is
  gone.
- `terminal_bindings()`: one `BindingAction::Action(shortcut)` binding per
  table row, registered in every terminal (`new_terminal` in
  `terminal_workspace/state.rs`). They replace the terminal's own binding for
  the same keys; on Linux that is only the Ctrl Shift duplicates of
  Ctrl B/K/E/F/T/W, so Ctrl B, Ctrl K, Ctrl W, Ctrl \ still reach the shell.
- `shortcut_for_key_press(event, platform)`: the subscription-side match. It
  matches like the terminal does: `key` (winit's key without modifiers),
  lower-cased, and exactly the table's modifiers. Repeats, releases and
  modifier changes give `None`.

### How routing works

Both routes end in `events::shortcuts::handle(app, shortcut)`
(`app/src/events/shortcuts.rs`):

1. **A terminal has focus.** The terminal matches its binding and publishes
   `BindingActionDispatched { action }`. The terminal workspace reducer runs
   Split pane itself (it needs the reporting pane; behavior unchanged) and
   turns every other shortcut into the new effect
   `TerminalWorkspaceEffect::ShortcutPressed { shortcut }`; the router
   (`events/terminal_workspace.rs`) passes it to `shortcuts::handle`.
2. **No terminal has focus.** `AppEvent::Keyboard` (from
   `iced::keyboard::listen()`) now goes to `shortcuts::handle_key_event`, which
   looks the key up with `shortcut_for_key_press(event, Platform::current())`.

`handle` runs: Settings → `TabsIntent::OpenSettingsTab`; New terminal →
`TabsIntent::OpenTerminalTab` with the shell name (same as the sidebar);
Close session → `TabsIntent::CloseTab` for the active tab (nothing when there
is none). Command palette, Left panel, Files panel, Focus mode → nothing yet
(one `match` arm with a comment pointing to tickets 07 and 17). Split pane →
nothing on this route: with no terminal focused there is no pane to split,
which is today's behavior.

### Research: does `keyboard::listen()` see keys the terminal captured?

No. In iced 0.14 `iced_futures::keyboard::listen()` keeps only
`subscription::Event::Interaction { status: Status::Ignored, .. }`, and
`iced_winit` broadcasts each window event with the status the whole widget
tree returned for it (one status per event; any `shell.capture_event()` makes
it `Captured`).

The terminal widget (`crates/ui/terminal/src/input.rs`,
`handle_keyboard_event`) only looks at keys while `is_focused`. For a key with
an `Action` binding it publishes the action and returns `Captured`, so
`listen()` never gets it. Keys it writes to the shell are `Captured` too. Keys
it neither writes nor binds return `Ignored`.

Why a press never fires twice:

- Terminal focused, first press: the terminal reports it and captures it; the
  subscription does not see it. One run.
- Terminal focused, auto-repeat: the terminal drops repeated `Action`
  bindings (`BindingAction::Action(_) if !is_repeat`) and returns `Ignored`, so
  the repeat reaches `listen()`; `shortcut_for_key_press` drops every repeat.
  No run.
- No terminal focused: every terminal returns `Ignored`; only the subscription
  runs it. One run.

Also checked: a focused iced `text_input` (Settings form, launch editor) does
not capture Ctrl Shift + letter (the text is a control character, which it
skips), so these shortcuts work while typing in a field too.

### Tests added (29 new, 3 updated)

| Where | Tests | What they pin |
|---|---|---|
| `app/src/shortcuts.rs` | 17 | every shortcut has a key and a modifier on Linux and macOS; no Linux shortcut is Ctrl + letter, `\` or `-` without Shift; no two shortcuts share keys on one platform; Linux and macOS labels equal the spec table; labels are built from the bindings; `label()` and `terminal_bindings()` use the current platform; terminal bindings register every row with its keys and `Action(shortcut)`; every row is found from its own key press on both platforms; an upper-case key matches; repeat, release, modifier change, extra modifier, named key, the other platform's keys and Ctrl B/K/T/W/\/- match nothing |
| `app/src/events/shortcuts.rs` | 8 | Settings, New terminal (shell title), Close session (active tab; nothing without tabs) emit the existing tab intents; palette, panels, focus mode and split pane do nothing; a subscription key press runs its shortcut, a repeat or another key does nothing |
| `app/src/events/mod.rs` | 1 | `AppEvent::Keyboard` reaches the shortcut router |
| `app/src/events/terminal_workspace.rs` | 2 | the `ShortcutPressed` effect is run by the app; a non-split binding action does not sync the explorer |
| `app/src/widgets/terminal_workspace/reducer.rs` | 1 (+3 updated) | every non-split shortcut from a terminal becomes `ShortcutPressed` and leaves the panes alone; the split tests now use `AppShortcut::SplitPane` |

The 3 old tests of `terminal_workspace/shortcuts.rs` are replaced by the
table tests above. `crates/ui/terminal` is not changed. No new dependencies.

## Comments

- **Manual check left for the owner (spec AC4):** in zsh run `cat -v` in an
  OTTY terminal and press Ctrl B, Ctrl K, Ctrl W, Ctrl \; it should print
  `^B`, `^K`, `^W`, `^\`. Also try Ctrl Shift T, Ctrl Shift W and Ctrl , with
  the terminal focused and with the Settings tab active. The agent cannot run
  the GUI.
- Inside a terminal, Ctrl Shift K/B/E/F no longer send `^K`/`^B`/`^E`/`^F`
  (the terminal used to send the same byte as without Shift); until tickets 07
  and 17 those keys do nothing. Ctrl , no longer types into the terminal.
- Matching uses the key without modifiers, as the terminal already does for
  split pane. With a non-Latin layout (for example Russian) the letter
  shortcuts do not match on either route; that was already true for split
  pane.
- On GNOME, IBus may take Ctrl Shift E (emoji) before the app sees it. Worth a
  look during the manual check.
- Coverage (`cargo llvm-cov --workspace --all-features`, lines): merge base
  `695358c` 71.82% → this ticket 72.15%.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok (the
  three old `yanked` warnings remain).
