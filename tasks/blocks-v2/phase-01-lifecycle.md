# Phase 1: typed identity and lifecycle reducer

Status: **complete**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-010–B2-016.

## Goal

Make block lifecycle deterministic and independent of UI heuristics. Every event addresses the
terminal session, shell instance, and block through distinct types. Duplicate, stale, missing,
and out-of-order events are handled by the reducer and cannot finish an adjacent block.

## Current state

`BlockId`, `TerminalSessionId`, `ShellInstanceId`, `ProtocolSequence`, lifecycle states and
outcomes, a separate sparse `MetadataPatch`, safe diagnostics, and one protocol-v2 reducer are
implemented. Lifecycle verifies block ownership against the shell instance, so stale IDs,
cross-shell events, and orphan completions cannot change an adjacent block.

The model is split into focused `id`, `model`, `lifecycle`, and `list` modules. `BlockList`
encapsulates ordered storage and `block_id_to_index`; a deterministic randomized test compares
the index with a linear search after every append, remove, and reorder. Production
synchronization adds reducer records without heuristically finishing the preceding UI block.

## Scope

- [x] **B2-010** Extend tests to cover every transition and recovery case.
- [x] **B2-011** Introduce typed IDs and minimal public accessors.
- [x] **B2-012** Define a separate `MetadataPatch`, lifecycle state and outcome, and sparse
  metadata updates that do not replace known fields.
- [x] **B2-013** Extend index tests through append, remove, reorder, and randomized sequences.
- [x] **B2-014** Remove the v1 `BlockEvent` lifecycle path; `LifecycleInput` and the reducer
  accept only protocol-v2 semantic events. Do not add a compatibility adapter.
- [x] **B2-015** Ensure duplicate or stale IDs cannot complete another block.
- [x] **B2-016** Split the remaining monolith into cohesive `model`, `lifecycle`, `id`, and
  `list` modules without pass-through modules or a larger public API.

## Required lifecycle cases

- normal prompt → command start → command end → next prompt;
- empty Enter without `command_start` finishes its own prepared block as
  `Finished(Exited(0))` without changing the previous finished block;
- Ctrl-C before and after command start;
- duplicate or stale sequences and a sequence gap;
- missing `prompt_prepare` and missing `command_end` with documented recovery;
- sparse completion preserves command, cwd-before, and start time;
- an unknown block ID does not change an adjacent block;
- nested or root shell exit finishes only blocks owned by that shell;
- an invalid transition emits a safe diagnostic without command or output payload.

## Automated verification

```bash
cargo test -p otty-surface block::lifecycle
cargo test -p otty-surface block
cargo clippy -p otty-surface --all-targets --all-features -- -D warnings
```

Before closing the phase, add a table-driven test for every case above and an invariant test
that compares `block_id_to_index` with a simple linear search after arbitrary append, remove,
and reorder sequences.

## Manual verification

1. Launch `cargo run -p otty` with Bash.
2. Run `pwd`, `false`, `cd /tmp`, `true`, and an empty Enter in sequence.
3. Confirm that every executed command has exactly one block. Empty Enter must create and
   finish its own empty block with exit code `0`, without finishing the previous `true` block
   again or changing its metadata.
4. From the repository root, run `source assets/shell-integrations/otty.bash` twice and then
   `printf 'duplicate-check\n'`. Exactly one command block must appear.
5. Start nested `bash --noprofile --norc -i`, run `false`, exit it, and run `true` in the parent
   shell. Child exit must not finish or modify the parent block.
6. Repeat the scenario in Zsh with `assets/shell-integrations/otty.zsh`.
7. Scroll back and visually confirm that commands, cwd values, and finished blocks do not
   change after later prompt events.

The phase is complete when production lifecycle has one protocol-v2 reducer, the old
`BlockEvent` path is absent, every recovery rule is covered by tests, and the manual scenario
creates no duplicate, missing, or cross-shell blocks.
