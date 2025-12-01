# AGENTS Instructions

General context lives in `README.md` at the repository root.

## Architecture

- OTTY is evolving toward a terminal-centric workspace. The current codebase focuses on the terminal stack: pseudo-terminal access, virtual terminal parsing, and higher-level escape handling.
- Runtime flow: `otty-pty` produces raw PTY bytes → `otty-vte` runs the finite-state VT parser → `otty-escape` lifts sequences into semantic `Action`s that a renderer/UI can consume.
- `otty-pty` already exposes interchangeable Unix and SSH backends. Both implement the shared `Session` and `Pollable` traits so higher layers can multiplex I/O with `mio`.
- `otty-vte` is a table-driven ECMA-48/xterm parser with UTF-8 support. It mirrors WezTerm/Alacritty style state machines and forwards events through the lightweight `Actor` trait.
- `otty-escape` wraps `otty-vte`, mapping control/CSI/OSC sequences into strongly typed Rust enums (cursor modes, colors, hyperlinks, keyboard reports, etc.).
- `otty-surface` currently contains placeholder code; it will host UI-surface abstractions once the rendering layer lands.
- Vendored playgrounds: `alacritty_terminal` and `iced_term` are checked in for experimentation with Iced-based frontends, but they are not part of the workspace build graph yet.
- Original high-level goals mention `otty-agent` and `otty-client`; those components are not implemented in this repository at the moment.

## Workspace crates

- `otty-vte` (`otty-vte/src/*`): VT parser core. Depends only on `utf8parse`. Key modules: `parser.rs` (state machine driver), `transitions.rs` (generated table), `csi.rs` (parameter decoding), `utf8.rs` (UTF-8 decoder).
- `otty-escape` (`otty-escape/src/*`): Provides the `Parser` facade and the `Action` enum consumed by terminal surfaces. Relies on `log` for tracing, `bitflags` for attribute masks, `cursor-icon` for cursor shapes, and the local `otty-vte`.
- `otty-pty` (`otty-pty/src/*`): Abstracts PTY sessions. Uses `mio` for event loop integration, `nix` + `signal-hook` for Unix PTYs, and `ssh2` for remote sessions. Exposes builders (`unix()`, `ssh()`) plus shared `Session`/`Pollable` traits, `PtySize`, and `SSHAuth`.
- `otty-surface` (`otty-surface/src/lib.rs`): Stub crate created by `cargo new`; expect it to evolve into the UI rendering layer.

## Development workflow

- Crate names stay prefixed with `otty-`.
- Prefer `format!("{value}")`-style interpolation instead of passing variables as separate arguments when formatting strings.
- When touching Rust code, run `cargo fmt` followed by `cargo clippy --workspace --all-targets`. Fix warnings wherever practical.
- Run `cargo test -p <crate>` (or `cargo test --workspace`) before submitting changes that affect logic-heavy crates like `otty-escape` or `otty-pty`.
- Add concise documentation comments to new public items to communicate intent.

## Terminal emulation

The VTE parser must cover the full xterm/ECMA-48 sequence set.

Primary references:

- https://vt100.net/emu/dec_ansi_parser
- https://wezfurlong.org/ecma48
- https://ecma-international.org/publications-and-standards/standards/ecma-48/
- https://mitchellh.com/
- https://gitlab.gnome.org/GNOME/vte
- https://github.com/alacritty/vte
- https://github.com/wezterm/wezterm/tree/main/vtparse
- https://github.com/TragicWarrior/libvterm
- https://docs.google.com/spreadsheets/d/19W-lXWS9jYwqCK-LwgYo31GucPPxYVld_hVEcfpNpXg/edit?gid=1724051764#gid=1724051764
- https://wezterm.org/escape-sequences.html#operating-system-command-sequences
