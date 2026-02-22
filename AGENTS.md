# AGENTS Instructions

General context lives in `README.md` at the repository root.

## Development workflow

- Crate names stay prefixed with `otty-`.
- Prefer `format!("{value}")`-style interpolation instead of passing variables as separate arguments when formatting strings.
- When touching Rust code, run `cargo fmt` followed by `cargo clippy --workspace --all-targets`. Fix warnings wherever practical.
- Run `cargo test -p <crate>` (or `cargo test --workspace`) before submitting changes that affect logic-heavy crates like `otty-escape` or `otty-pty`.
- Add concise documentation comments to new public items to communicate intent.
- Prefer borrowing over cloning; pass `&T`/`&str` where possible and keep ownership at boundaries.
- Avoid unnecessary heap allocations; use slices and references for read-only data.
- Use `Result`/`Option` for error handling; no `unwrap()` in production code (prefer `expect()` with context during initialization).
- Use explicit error types (e.g., with `thiserror`) and propagate with `?`.
- Keep APIs minimal and trait-based; use associated types for event/action contracts.
- Do not expose struct fields as `pub`; use idiomatic Rust accessors for reads (`field()` or `is_*` for booleans), and prefer domain-specific mutators for writes (use `set_*` only when a generic setter is the clearest option, or keep mutation local to the module).
- For `match` on `enum`, prefer a wildcard arm (`_ => ...`) by default for fallback logic.
- Prefer `format!("{value}")` style interpolation for strings.
- Document public items with concise doc comments and examples.
- Run `cargo fmt`, `cargo clippy --workspace --all-targets`, and relevant `cargo test` targets after changes.

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
