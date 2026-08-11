# Phase 4: sectioned block content and freeze

Status: **partially complete**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-040–B2-047.

## Goal

Store prompt, canonical command, and output as independent semantic sections. While a command
is active, mutable terminal state is permitted only for the output owner. On completion, the
block freezes into compact read-only logical lines and no longer retains a full `Surface`.

## Current state

`BlockContent`, `CommandRecord`, basic prompt-range capture, a command header and output, and
semantic prompt/output snapshot queries are implemented. This is a minimal initial slice, not
the completed sectioned-content model.

Independent `HeaderGrid` and `OutputGrid`, output routing, immutable freeze, budgets and
truncation, alternate-screen ownership, and removal of `cached_text` and mutable historical
surfaces remain incomplete.

## Scope

- [ ] **B2-040** First test section boundaries for single-line, multiline, and right prompts,
  edited and empty commands, output, and background output.
- [ ] **B2-041** Introduce explicit `HeaderGrid`, prompt/command ranges, and `OutputGrid`;
  retain current `BlockContent` only as preparation.
- [ ] **B2-042** Implement `OutputRouter` for root and child active blocks, headers, output,
  background streams, and alternate screen.
- [ ] **B2-043** Add explicit confidence and every source case to `CommandRecord`; the shell
  command is already the primary source.
- [ ] **B2-044** Convert completed blocks to read-only logical lines and release runtime-only
  `Surface` state.
- [ ] **B2-045** Add per-block and global budgets, explicit truncation metadata, and safe
  anchor and selection updates.
- [ ] **B2-046** Associate alternate-screen state with the owning command block and restore the
  normal viewport afterward.
- [ ] **B2-047** Remove per-snapshot `cached_text`; every off-screen query must use frozen
  content.

## Data invariants

- Prompt is not part of `Output`, and the primary command path never derives command text from
  the first visual line.
- Soft wrap changes visual rows without changing logical content or section ranges.
- Finished content is immutable; later PTY output routes only to the active owner or a
  documented background destination.
- Truncation is always visible in metadata and export and never leaves an anchor inside the
  removed range.
- Alternate-screen content belongs to the command that launched it and does not pollute normal
  output.

## Automated verification

```bash
cargo test -p otty-surface block::content
cargo test -p otty-surface block
cargo test -p otty-ui-term block
```

Add a memory assertion or metric-based ignored test proving that the number of full mutable
`Surface` instances does not grow linearly with finished blocks after freeze.

## Manual verification

1. Launch `cargo run -p otty` and run commands with a normal prompt, empty output, multiline
   output, and a multiline heredoc.
2. Type a command, edit it with arrow keys before Enter, and confirm that Copy Command returns
   the executed text rather than original keystrokes or the first visual line.
3. Enable a right prompt in Zsh and repeat separate prompt, command, and output copies.
4. Start a background command that prints between two prompts. Its output must not enter an
   arbitrary completed block.
5. Run Copy Prompt, Copy Command, Copy Output, and Copy Whole for one block. Each operation
   must contain only the requested sections in stable order.
6. Run `less` or another full-screen TUI, exit it, and verify restoration of the normal
   viewport and alternate-screen ownership by the correct block.
7. Complete 1,000 small commands, resize, and continue printing. Old block contents must not
   change, and the mutable-surface metric must remain bounded.
8. Exceed per-block and global budgets. The UI and export must show a truncation marker and the
   number of lost lines.

The phase is complete only after finished history no longer stores mutable `Surface` state and
all section-boundary scenarios pass without viewport-based heuristics.
