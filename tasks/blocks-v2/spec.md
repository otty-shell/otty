# Blocks v2 specification

Status: **implementation in progress; item-level progress is tracked in the phase files**.

This specification defines the shared product invariants, responsibility boundaries, and
completion criteria for Blocks v2. Detailed scope, tests, and manual verification belong to
the corresponding phases and are not duplicated here.

## Goal

Blocks v2 replaces the unstable stitched-history model with a system in which:

1. A single canonical `BlockList` stores blocks with stable IDs, an explicit lifecycle, and
   separate prompt, command, and output sections.
2. The viewport preserves user intent through a block-local logical anchor instead of only an
   offset from the bottom edge.
3. A versioned shell protocol addresses the terminal session, shell instance, and block,
   reports an exact `command_end`, and safely recovers from missing events.
4. Replaceable frames have a strictly bounded backlog, while lifecycle and other critical
   events are delivered separately without loss.

Copy, save, navigation, and presentation operations must use `BlockId` and semantic block
sections rather than geometry from the latest UI frame.

## Fixed decisions

- Protocol v2 is the only production path. There will be no v1 runtime switch, compatibility
  adapter, or migration window.
- Rollback installs the previous application artifact instead of retaining two protocol paths
  in one binary.
- The shared DCS parser safely ignores legacy `otty-dcs;block` messages without creating a
  semantic event or block.
- Shell integration must not be required for a usable terminal. If bootstrap fails, the
  terminal continues as an ordinary terminal and reports a diagnosable `Degraded` or
  `Unsupported` status.
- User shell, tmux, screen, and remote configuration is never changed automatically.
  Persistent or remote bootstrap requires an explicit action and confirmation.
- Persistence is implemented only after frozen content and a stable export schema. New
  storage, compression, and serialization dependencies require separate approval.

## Product invariants

- Finished terminal content is immutable; later PTY output cannot change a completed block.
- Presentation state (`collapsed`, `pinned`, labels, groups, and order) is stored separately
  from the canonical transcript.
- New output does not move the user's viewport after they leave the bottom of history.
- Every UI action targets a stable `BlockId`; the block need not be present in the current
  frame.
- Exactly one block owns ordinary PTY output for the active shell context.
- A duplicate, stale, malformed, or foreign-session event cannot finish or modify an adjacent
  block.
- Prompt, command, and output have independent semantic boundaries; visual wrapping does not
  change logical content.
- Selection addresses stable logical cells and preserves its meaning after output, resize, and
  reflow.
- Replaceable frame backlog never exceeds one frame; a slow UI does not stop PTY reads.
- Commands and output never appear in ordinary logs, diagnostics, or benchmark reports.

## Target architecture

```text
PTY byte stream
  -> VTE parser
     -> printable/control actions --------------------+
     -> protocol v2 + OSC 133 semantic events --------|
                                                       v
                                             lifecycle reducer
                                                       |
                                                       v
                      +---------------- canonical BlockList ----------------+
                      | ID index | height index | ShellContext tree          |
                      | HeaderGrid + ranges | OutputGrid | immutable history |
                      +------------------------------------------------------+
                                  |                         |
                          latest viewport frame       lossless events
                                  |                         |
                                  +----------- UI ----------+
                                             BlockId actions
```

### Layer responsibilities

`otty-escape` safely extracts framing, decodes bounded protocol v2 and OSC 133, and emits typed
semantic events without deciding which block is active.

`otty-surface` exclusively owns the `BlockList`, lifecycle, output routing, height index,
selection, and viewport anchor. It builds snapshots only for the visible range and exposes
separate block queries for off-screen operations.

`otty-libterm` preserves PTY action and lifecycle event ordering, coalesces render
invalidations, and owns the latest-frame mailbox and a separate bounded lossless queue.

`otty-ui/terminal` renders the calculated visible window, performs hit testing against stable
positions and revisions, and sends typed actions. It does not locate off-screen blocks from
viewport rectangles.

The `otty` application prepares and diagnoses shell integration, displays capability status,
performs filesystem export outside the render thread, and manages presentation references
separately from the canonical transcript.

## Cross-cutting contracts

### Identity and lifecycle

- `TerminalSessionId`, `ShellInstanceId`, `BlockId`, and `ProtocolSequence` are distinct types;
  public APIs do not accept an arbitrary string in place of an ID.
- A block is created for future input on `prompt_prepare`; the prompt, command, and output for
  one command retain the same `BlockId`.
- Only the reducer changes lifecycle state. Completion applies a sparse metadata patch and
  does not replace known command, cwd, or timestamp fields.
- An old sequence is ignored, a duplicate is idempotent, a gap enables safe recovery, and an
  orphan completion remains a diagnostic rather than being applied to a similar ID.
- Shell exit finishes only active blocks owned by that context and returns routing to the
  parent context.

### Content and routing

- `HeaderGrid` stores the rendered prompt and command echo with semantic ranges;
  `CommandRecord` stores the canonical command; `OutputGrid` receives data after
  `command_start`.
- Command-source priority is the OTTY input buffer, the shell event, and then extraction from
  the command range as an explicitly marked fallback.
- On completion, mutable terminal state is converted into compact read-only logical lines.
- Background and alternate-screen output have explicit owners and are never appended to the
  last finished block by heuristic.
- Per-block and global budgets produce visible truncation metadata and safely recover anchors
  and selections when logical lines are removed.

### Viewport and snapshots

- `FollowTail` keeps the bottom visible; `Anchored` stores a `BlockId`, logical line, and
  position within the viewport.
- Append, freeze, truncate, resize, collapse, and reorder all pass through one viewport-state
  change operation.
- The height index supports prefix sums, range lookup, and height updates without scanning the
  entire history.
- Resize changes wrapping but preserves the anchored logical line and selection endpoints.
- A snapshot contains the viewport plus bounded overhang, not cells from the entire history.

### Transport and revisions

- A frame is replaceable state and may replace an unread frame; lifecycle, child exit, and
  errors are lossless events.
- Frames and coordinate requests carry model and viewport revisions. A stale request is
  rejected or resolved again through a stable ID or position.
- PTY reads are coalesced into render ticks; resize, scroll, and selection may request an
  immediate render.
- Resize, reset, and alternate-screen transitions produce full damage; ordinary output uses
  partial damage where possible.

### Actions, export, and persistence

- Internal controls, overlay, keyboard, and context menu use one exhaustive action API with
  state-aware availability.
- Move, group, split, collapse, pin, and hide modify presentation references rather than the
  canonical transcript. Rerun creates a new execution block.
- Copy and Save use one backend `ExportBlock` pipeline for Prompt, Command, Output, Whole, and
  Selection in Plain, ANSI, Markdown, or versioned JSON.
- Plain export does not turn a soft wrap into a hard newline, duplicate a wide-character
  spacer, or include the prompt in Output. Truncated content includes an explicit marker.
- Save atomically writes the same result outside the render thread. Internal `Surface` state
  is not a persistence format.

### Protocol, shell, and security

- The protocol has a required major version, a registered terminal session, shell and block
  identities, and a monotonic per-shell sequence.
- Encoded and decoded payloads, commands, and identifiers have explicit limits. Oversized or
  malformed input enters bounded discard or recovery state without stopping the parser.
- Diagnostics contain the reason, safe IDs, and revisions, but never command or output
  payloads.
- Shell hooks preserve exit and pipeline status before helper calls, retain existing hooks, do
  not require `jq`, Python, or Perl on each prompt, and use process-scoped idempotency.
- Every session explicitly reports `Pending`, `Active(v2)`, `Degraded(reason)`, or
  `Unsupported(shell)`.

## Phases

Phase files are the single source of truth for item-level status, checklists, automated
verification, and manual verification.

| Phase | IDs | Result |
|---|---|---|
| [0 — Baseline](phase-00-baseline.md) | B2-001–B2-005 | Regression scenarios and reproducible metrics |
| [1 — Lifecycle](phase-01-lifecycle.md) | B2-010–B2-016 | Typed identity and a deterministic reducer |
| [2 — Protocol v2](phase-02-protocol-v2.md) | B2-020–B2-027 | Bounded parser and semantic events |
| [3 — Shell bootstrap](phase-03-shell-bootstrap.md) | B2-030–B2-039 | Bash/Zsh lifecycle and integration status |
| [4 — Content freeze](phase-04-content-freeze.md) | B2-040–B2-047 | Sectioned content, routing, and immutable history |
| [5 — Viewport](phase-05-viewport.md) | B2-050–B2-058 | Height index, stable anchor, and selection |
| [6 — Transport](phase-06-transport.md) | B2-060–B2-066 | Latest frame, revisions, and lossless events |
| [7 — Presentation](phase-07-presentation.md) | B2-070–B2-078 | Unified actions and presentation model |
| [8 — Export](phase-08-export.md) | B2-080–B2-085 | Copy/save pipeline and versioned formats |
| [9 — Shells](phase-09-shells.md) | B2-090–B2-094 | Additional capability-gated environments |
| [10 — Rollout](phase-10-rollout.md) | B2-100–B2-105 | Final acceptance and removal of legacy architecture |

### Dependency order

Phases are completed in order from 0 through 10. In particular:

- sectioned content depends on lifecycle and prompt markers;
- anchored viewport depends on typed identity, logical lines, and the height index;
- UI actions depend on a stable query API and sectioned content;
- nested-shell routing depends on protocol v2 and `ShellContext`;
- persistence depends on frozen content and a stable export schema;
- final rollout depends on acceptance of preceding phases and does not introduce a dual path.

A later phase must not mask an unfulfilled invariant from an earlier phase. At every boundary,
the terminal remains launchable and a shell without integration remains usable.

## Shared implementation rules

1. For business logic, add a failing test before implementing the change.
2. Do not add a dependency without prior user approval.
3. Do not include command or output contents in logs, diagnostics, or benchmark reports.
4. Update item-level status only in the corresponding phase file; update this specification
   only when the shared contract, dependency order, or Definition of Done changes.
5. Before closing a phase, complete its manual scenario in the real application.
6. Run `cargo llvm-cov --workspace --all-features` with the same command before and after a
   change; overall line coverage must not decrease.

## Load criteria

- With 10,000 small finished blocks, visible-range lookup does not scan every grid.
- With 1,000,000 output lines and an artificially slow UI, replaceable frame backlog does not
  exceed one frame and PTY reads continue.
- Memory is bounded by a global budget and stabilizes after truncation and freeze.
- A snapshot contains `viewport rows * columns` plus bounded overhang rather than cells from
  the entire history.
- One hundred consecutive resizes do not process the full `Surface` of every historical block
  for each frame.
- The viewport regression matrix has no scroll jumps.
- Copy and Save for an off-screen block work without scrolling first.
- Every invalid lifecycle transition produces a safe diagnostic rather than a panic.

## Definition of Done

Blocks v2 is complete only when all of the following are true:

- applicable checklist items in phases 0–10 are closed and their manual scenarios confirmed;
- root and nested Bash/Zsh lifecycle tests pass without duplicate or missing blocks;
- exit code, cwd, and other known metadata survive sparse updates;
- prompt, command, and output export independently and correctly for multiline cases;
- an off-screen block can be found, scrolled to, copied, and saved;
- manual scroll and logical selection remain stable during long-running output, truncation, and
  resize;
- burst output creates no unbounded frame backlog and loses no lossless events;
- finished history does not retain a full mutable `Surface` for every block;
- the UI reports integration status and the exact reason for degraded mode;
- presentation reorder, split, and group operations do not change the canonical transcript;
- malformed, spoofed, and stale protocol events do not corrupt adjacent blocks;
- the load criteria above pass and final results are compared with the
  [baseline](baseline-results.md);
- the production v1 path and old stitched-history architecture are removed, while legacy DCS
  is safely ignored;
- formatting, Clippy with `-D warnings`, deny, all workspace tests, and coverage pass without
  reducing overall line coverage.
