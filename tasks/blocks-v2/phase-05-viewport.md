# Phase 5: height index, viewport, and selection

Status: **partially complete**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-050–B2-058.

## Goal

Make the viewport stable during active-output growth, resize and reflow, freeze, truncation,
and presentation changes. Scroll and selection must use stable block and logical positions,
while visible-range lookup must not scan the entire history.

## Current state

Basic `ScrollPosition::FollowTail/Anchored`, a block anchor during manual scroll, recovery when
the anchored block is removed, and model-level off-screen `ScrollToBlock` with alignment are
implemented.

`BlockHeightIndex`, a unified `Viewport::apply_change`, stable `LineId` and `BlockPoint`, lazy
reflow, viewport-only snapshots, and a benchmark proving the absence of a full-history scan are
still missing. Selection still depends on stitched visual coordinates.

## Scope

- [ ] **B2-050** Move the required regression matrix below into focused viewport tests;
  existing anchor and `ScrollTo` tests cover only part of it.
- [ ] **B2-051** Implement `BlockHeightIndex` with randomized comparison against a `Vec`
  reference.
- [ ] **B2-052** Complete `FollowTail/Anchored` through one `Viewport::apply_change` path for
  append, resize, freeze, truncate, collapse, and presentation updates.
- [ ] **B2-053** Introduce stable `LineId` and logical-line to wrapped-row mapping.
- [ ] **B2-054** Move selection to `BlockPoint` and remove global stitched-row identity.
- [ ] **B2-055** Build snapshots only for the viewport plus bounded overhang.
- [ ] **B2-056** Connect `ScrollToBlock` to the height index and test start, center, end, and
  nearest alignment; the model request and basic alignment already exist.
- [ ] **B2-057** Stop resizing all historical surfaces, apply lazy frozen reflow, and update
  height cache only for affected blocks.
- [ ] **B2-058** Prove by benchmark that the frame path performs no full-history grid scan.

## Required regression matrix

- the active block grows by 100,000 lines while the user remains in the middle of history;
- head truncation occurs below, above, and directly on the anchored line;
- resizing 80 → 200 → 40 columns preserves the logical anchored line;
- freeze or reflow of a finished block does not change adjacent visible content;
- collapse, expand, insert, remove, and reorder above the viewport cause no visual jump;
- `ScrollToBlock` locates a fully off-screen block;
- selection remains on the same logical cells after output and resize;
- removing the anchored block selects the documented nearest neighbor;
- entering and leaving alternate screen restores the normal scroll state.

## Viewport invariants

- In `FollowTail`, new output keeps the bottom edge visible.
- In `Anchored`, new output below the anchor does not move the user's top logical position.
- Resize changes wrapping while preserving the anchored `LineId` and logical selection
  endpoints.
- Removal or truncation of the anchor chooses a documented nearest neighbor.
- Collapse or reorder above the viewport compensates for the height change without a visual
  jump.
- Off-screen lookup uses an ID and does not depend on presence in the latest snapshot.

## Automated verification

```bash
cargo test -p otty-surface block
cargo test -p otty-ui-term block_layout
```

The randomized test must perform insert, remove, height change, prefix sum, and range lookup,
comparing `BlockHeightIndex` with a simple reference vector after every operation.

## Manual verification

1. Launch `cargo run -p otty`, create at least 100 blocks, and move to the middle of history.
2. Start an active command with long output. The anchored visible line must not move; returning
   to the bottom must switch the viewport back to `FollowTail`.
3. Resize through 80 → 200 → 40 columns. The top logical line and selected text must remain the
   same although visual wrapping changes.
4. Select text in an old block, continue output, and resize again. Copy Selection must return
   the same logical cells.
5. Call `ScrollToBlock` for a fully off-screen block with start, center, end, and nearest
   alignment. Each mode must produce its documented position.
6. After presentation actions exist, collapse, expand, and move a block above the viewport.
   Current visible content must not jump.
7. Exceed the history budget so truncation affects an area before, at, and after the anchor;
   verify all three documented recovery outcomes.
8. Enter and leave alternate screen and confirm restoration of normal scroll state.
9. With 10,000 blocks in history, enable metrics and confirm that lookup and snapshots do not
   visit every grid and snapshot size is limited to viewport plus overhang.

The phase is complete when the entire regression matrix has no scroll jumps, selection has
stable logical identity, and frame cost does not scale linearly with total history.
