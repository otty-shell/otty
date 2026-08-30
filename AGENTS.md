# AGENTS Instructions

General context lives in [README.md](./README.md) at the repository root.

## Agent skills

### Issue tracker

Issues are tracked as local Markdown under `specs/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage uses the default canonical label vocabulary. See `docs/agents/triage-labels.md`.

### Domain docs

Domain documentation uses the single-context layout. See `docs/agents/domain.md`.

## Rules

- Use English for all repository content, including comments, documentation, commit messages, and test data. Non-English text is allowed only when it is necessary for testing.
- Use plain, direct language in chats and repository content, including specifications. Prefer everyday explanations to technical jargon. Use technical terms only when they are necessary or widely understood, and explain less common terms in plain language when first used.
- You MUST write the tests before writting the implementation.
- You MUST write tests only for business-significant packages such as usecases, repositories, helpers, and domain logic. Do not add tests for infrastructure/bootstrap packages such as lifecycle, config, metrics, logging, or server wiring unless they contain business-significant behavior.
- You MUST use `mockall` for mocks in RUST code
- You MUST ask me before installing the new dependencies with dependency description and reason.
- You MUST prefer simple, direct, readable code with explicit business logic. Avoid clever generics, macros, type gymnastics, and dense control flow when straightforward code is easier to understand.
- You MUST keep each source file focused on one cohesive responsibility. When one file combines multiple independent responsibilities or becomes difficult to navigate, split it into clearly named modules and files, with each file owning one responsibility. Do not split tightly coupled logic solely because of line count, and do not introduce empty, pass-through, or speculative modules.
- Rust modules MUST be organized in this order: imports; structs with their implementations; public functions; private functions; tests. Each struct declaration MUST be followed immediately by its related implementations before declaring the next struct. Within a struct's implementations, use this order: getters; constructors and other logic methods; `#[cfg(test)]` implementations; trait implementations. Use `otty-ui/terminal/src/render_runs.rs` and `otty-ui/terminal/src/shaped_text.rs` as good examples of this layout.
- You MUST separate distinct logical phases inside Rust functions with a single blank line, including input preparation, validation or branching, external or repository I/O, state changes, and result construction. Keep statements that form one tightly coupled operation together; do not add a blank line after every statement mechanically.
- You MUST NOT create abstractions by default. Every new trait, interface, layer, factory, manager, service, or extension point MUST solve a current problem and be briefly justified. "Maybe useful later" is not a valid justification; use a concrete implementation or private function instead.
- You MAY introduce an abstraction only when it has multiple real implementations, crosses an actual infrastructure boundary, protects domain or usecase code from infrastructure, removes duplication with the same business meaning and reason to change, or makes testing significantly simpler without hiding logic.
- You MUST prefer meaningful domain names over generic names such as `Manager`, `Processor`, `Helper`, `Service`, or `Util`. Split functions, files, and layers only when doing so improves the current design and readability.
- You MUST apply DRY only when duplicated logic has the same business meaning and changes for the same reason. Duplication is acceptable when extraction would create a vague or harder-to-read abstraction, and speculative traits, configuration, factories, placeholder layers, and unused extension points are forbidden by YAGNI.
- crate names MUST stay prefixed with `otty-`.
- Prefer `format!("{value}")`-style interpolation instead of passing variables as separate arguments when formatting strings.
- You MUST add concise documentation comments to new public items to communicate intent.
- Prefer borrowing over cloning; pass `&T`/`&str` where possible and keep ownership at boundaries.
- Avoid unnecessary heap allocations; use slices and references for read-only data.
- Use `Result`/`Option` for error handling; no `unwrap()` in production code (prefer `expect()` with context during initialization).
- Use explicit error types (e.g., with `thiserror`) and propagate with `?`.
- Keep APIs minimal and trait-based; use associated types for event/action contracts.
- Do not expose struct fields as `pub`; use idiomatic Rust accessors for reads (`field()` or `is_*` for booleans), and prefer domain-specific mutators for writes (use `set_*` only when a generic setter is the clearest option, or keep mutation local to the module). Exception: plain input/context structs with no invariants to protect (e.g. feature `Ctx` types passed into `reduce`) MAY use `pub(crate)` fields directly — accessors would be unnecessary boilerplate for parameter bags.
- For `match` on `enum`, prefer a wildcard arm (`_ => ...`) by default for fallback logic.
- Document public items with concise doc comments and examples.
- You MUST run all linters, checks and tests before finishing your work.
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

## Review-derived PR gates

Every future PR MUST read `docs/review-lessons.md` and complete these checks
before requesting review:

- Write the issue contract as observable behavior, including supported
  platforms, default values, user operation order, and a reproducible case.
- Inspect the current `main` base, rebase before review when the branch is
  stale, and classify every changed path as issue work, a regression fix, or
  unrelated work. Remove unrelated lint, docs, test-data, policy, and cleanup
  changes from the PR.
- Read every GraphQL review thread, including resolved and outdated threads,
  and classify each as `ACCEPT`, `REJECT`, or `CLARIFY`. Do not guess when a
  comment changes scope or leaves the required behavior unspecified.
- Trace the complete behavior path, including focus or ownership, event
  routing, reporting identifiers, side effects, shutdown paths, and the
  equivalent user operation. Test both the normal and alternate paths.
- Treat platform conditionals as separate behavior. Add target-specific CI or
  a portable selector test, and list real-device or real-OS behavior as
  unverified when it was not executed.
- For asynchronous state, test stale completion, newer edits, reset or reload,
  failure, and retry behavior. Do not rely on a green happy-path test.
- Write a failing behavioral test before each behavior change. Avoid wall-clock
  timing assumptions, ambient host state such as `$SHELL`, synthetic geometry,
  and source-text assertions.
- Run the full repository gates and perform the real user operation after the
  tests pass. CI success does not prove GUI, hardware, or non-current-platform
  behavior.
