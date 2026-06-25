# AGENTS Instructions

General context lives in [README.md](./README.md) at the repository root.

## Development workflow

- You MUST write the tests before writting the implementation.
- You MUST use `mockall` for mocks in RUST code
- You MUST run all linters, checks and tests before finishing your work.
- You MUST ask me before installing the new dependencies with dependency description and reason.
- You MUST prefer simple, direct, readable code with explicit business logic. Avoid clever generics, macros, type gymnastics, and dense control flow when straightforward code is easier to understand.
- You MUST keep each source file focused on one cohesive responsibility. When one file combines multiple independent responsibilities or becomes difficult to navigate, split it into clearly named modules and files, with each file owning one responsibility. Do not split tightly coupled logic solely because of line count, and do not introduce empty, pass-through, or speculative modules.
- You MUST separate distinct logical phases inside Rust functions with a single blank line, including input preparation, validation or branching, external or repository I/O, state changes, and result construction. Keep statements that form one tightly coupled operation together; do not add a blank line after every statement mechanically.
- You MUST NOT create abstractions by default. Every new trait, interface, layer, factory, manager, service, or extension point MUST solve a current problem and be briefly justified. "Maybe useful later" is not a valid justification; use a concrete implementation or private function instead.
- You MAY introduce an abstraction only when it has multiple real implementations, crosses an actual infrastructure boundary, protects domain or usecase code from infrastructure, removes duplication with the same business meaning and reason to change, or makes testing significantly simpler without hiding logic.
- You MUST apply DRY only when duplicated logic has the same business meaning and changes for the same reason. Duplication is acceptable when extraction would create a vague or harder-to-read abstraction, and speculative traits, configuration, factories, placeholder layers, and unused extension points are forbidden by YAGNI.
- Crate names stay prefixed with `otty-`.
- Prefer `format!("{value}")`-style interpolation instead of passing variables as separate arguments when formatting strings.
- Add concise documentation comments to new public items to communicate intent.
- Prefer borrowing over cloning; pass `&T`/`&str` where possible and keep ownership at boundaries.
- Avoid unnecessary heap allocations; use slices and references for read-only data.
- Use `Result`/`Option` for error handling; no `unwrap()` in production code (prefer `expect()` with context during initialization).
- Use explicit error types (e.g., with `thiserror`) and propagate with `?`.
- Keep APIs minimal and trait-based; use associated types for event/action contracts.
- Do not expose struct fields as `pub`; use idiomatic Rust accessors for reads (`field()` or `is_*` for booleans), and prefer domain-specific mutators for writes (use `set_*` only when a generic setter is the clearest option, or keep mutation local to the module). Exception: plain input/context structs with no invariants to protect (e.g. feature `Ctx` types passed into `reduce`) MAY use `pub(crate)` fields directly — accessors would be unnecessary boilerplate for parameter bags.
- For `match` on `enum`, prefer a wildcard arm (`_ => ...`) by default for fallback logic.
- Document public items with concise doc comments and examples.
- Run `cargo +nightly fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and fix all errors and warnings.
- Run `cargo deny check` and fix all output errors.
- Run `cargo test --workspace --all-features` all tests MUST be passed
- Run `cargo llvm-cov --workspace --all-features --fail-under-lines 80` for checking the test coverage level and ensure that it's not decreased for changed code (baseline >= 80%)

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
