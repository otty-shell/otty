# 06 — App shortcuts

Status: ready-for-agent
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
