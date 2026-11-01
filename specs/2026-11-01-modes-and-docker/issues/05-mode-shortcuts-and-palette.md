# 05 — Mode shortcuts and palette actions

Status: ready-for-agent
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
