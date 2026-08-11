# Phase 7: UI actions and presentation model

Status: **partially complete**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-070–B2-078.

## Goal

Give internal BlockUI controls, overlay, keyboard, and context menu one action and query
contract. User reorder, group, split, and collapse operations modify only presentation
references, never canonical PTY transcript or immutable finished content.

## Current state

The existing geometric action button, hover and click handling, and basic semantic
output/whole copy are connected. Off-screen `ScrollToBlock` uses a model request rather than
only the current frame.

Exhaustive `BlockAction`, `available_actions`, backend export queries, collapse/pin/hide/rerun,
presentation order, groups and slices, one summary API, and complete keyboard accessibility
remain unimplemented.

## Scope

- [ ] **B2-070** First test one action path shared by internal controls, overlay, keyboard,
  and context menu.
- [ ] **B2-071** Introduce exhaustive `BlockAction` and state-aware `available_actions()`.
- [x] **B2-072** Connect action-button hover and click; add focus and overlap tests.
- [ ] **B2-073** Move copy prompt/command/output/whole to backend `ExportBlock` queries;
  current snapshot helpers are temporary.
- [ ] **B2-074** Implement collapse/expand, pin/hide, and rerun through stable `BlockId`.
- [ ] **B2-075** Store presentation order, groups, and slices separately from canonical
  transcript.
- [ ] **B2-076** Make move, group, and split modify references; physical mutation of active
  content is forbidden.
- [ ] **B2-077** Use one `BlockSummary` and `BlockAction` API for internal and external UI.
- [ ] **B2-078** Add keyboard focus and accessibility without intercepting ordinary terminal
  input.

## Presentation-model rules

- Canonical transcript is append-only except for documented retention and truncation.
- An active block cannot be physically split or moved; the UI may create only a presentation
  slice.
- Action availability depends on lifecycle and content capability rather than visibility.
- Rerun creates a new command execution and block without changing the old outcome.
- Hidden, collapsed, and pinned state does not change exported content.
- Internal and external controls publish the same semantic action without duplicated business
  logic.

## Automated verification

```bash
cargo test -p otty-ui-term --all-features
cargo test -p otty-surface block
cargo test -p otty terminal_workspace
```

Add table-driven `available_actions` tests for active, finished, static, background, and
truncated states, plus a contract test that sends the same action through all four UI paths.

## Manual verification

1. Launch `cargo run -p otty`, create successful and failed blocks, and hover over the action
   area. Hover and click targets must match the rendered button.
2. Run Copy Prompt, Command, Output, and Whole from the internal button, overlay, context menu,
   and keyboard. All four paths must produce an identical result for the same action.
3. Scroll a block fully outside the viewport and invoke an action from the external list. It
   must target the same `BlockId` without rendering the block first.
4. Collapse and expand an old block; the viewport anchor must not jump. Pin and hide must not
   change canonical transcript or export.
5. Rerun a finished command. A new block with a new ID must appear while the old outcome remains
   unchanged.
6. Create a group, change presentation order, and split a reference. Restore the original view
   and confirm that canonical order and content remain unchanged.
7. Attempt to split or move an active block. The unavailable action must be absent or return an
   explicit rejection without partial mutation.
8. Navigate controls using only the keyboard and verify visible focus and screen-reader labels.
   Printable keys in ordinary terminal mode must still reach the PTY.

The phase is complete when every UI entry point uses one action API, off-screen operations do
not depend on snapshot geometry, and presentation changes never mutate transcript.
