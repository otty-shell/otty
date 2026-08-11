# Phase 10: final v2 rollout and legacy removal

Status: **partially complete: the v1 decision and verification groundwork are recorded**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-100–B2-105.

## Goal

After the full acceptance matrix passes, make Blocks v2 the only production path and remove v1
emission, parser, actions, and lifecycle plus the old stitched-history architecture before
release.

## Fixed v1 decision

- V1 is not a public compatibility mode and does not coexist with v2 in the production binary.
- No `old/v2` runtime switch, compatibility adapter, or migration window is implemented.
- Shell integration and parser ship in one application artifact, so no user-data migration is
  required between protocol versions.
- Rollback installs the previous application artifact. The new binary does not retain the
  previous path solely for rollback.
- After v1 removal, the shared DCS parser safely ignores old `otty-dcs;block` messages without
  creating a block or event or causing a panic.

## Current state

Formatting, Clippy, deny, workspace tests, and coverage were run for the initial slice and all
commands pass. Recorded global line coverage is 51.16%; there is no fixed minimum, but later
changes must not reduce overall line coverage from their pre-change baseline. `cargo deny`
reports existing warnings for yanked transitive `core2 0.4.0` and `spin 0.9.8` and exits
successfully.

The B2-100 architectural decision is fixed. V1 script emission, parser and schema, actions and
handlers, and lifecycle are removed from production; framing remains only in ignore tests and
historical documentation. The phase 0 baseline is recorded. The complete acceptance matrix,
comparison with the final architecture, and removal of old stitched history are not complete.

## Scope

- [x] **B2-100** Define a single-path release policy: no runtime switch, compatibility adapter,
  or migration window; rollback uses the previous artifact.
- [x] **B2-101** Pass the complete required check set, including `cargo llvm-cov`, confirm that
  overall line coverage does not decrease from the pre-change baseline, and introduce no new
  deny errors or warnings.
- [ ] **B2-102** After the target architecture is complete, repeat B2-004/B2-005 and compare
  with `baseline-results.md`. Do not build two engine paths into one binary for comparison.
  Record memory, frames, latency, and scroll correctness.
- [ ] **B2-103** Confirm that every new supported terminal session uses only v2 after the
  Bash/Zsh, viewport, export, and burst-output acceptance matrix. Bootstrap failure leaves a
  working ordinary terminal in `Degraded`, never a v1 fallback.
- [ ] **B2-104** Before release, complete and confirm removal from B2-014/B2-021: remove any
  remaining v1 script emission, parser/schema, actions/handlers, and tests except the
  legacy-ignore fixture; also remove the old stitched-history snapshot, temporary
  `integration_status_badge`, and related `is_shell` plumbing from pane-grid. Retain final
  integration diagnostics in status/debug UI and confirm no production v1 search matches.
- [ ] **B2-105** Update the `otty-surface`, `otty-escape`, and `otty-ui/terminal` READMEs with
  final contracts, limits, examples, and troubleshooting without v1 migration instructions.

## Finalization conditions

- Phases 0–8 are closed and applicable phase 9 items have honest capability statuses.
- Finished history does not retain a mutable `Surface` per block.
- Off-screen copy/save and stable viewport/selection are manually verified.
- Replaceable backlog is limited to one frame during burst output.
- Protocol spoof, malformed-input, and stale-event matrices do not corrupt the model.
- Final v2 and the saved baseline are measured with identical scenarios on comparable
  machines.

## Automated verification

```bash
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features
```

Run coverage with the same command before and after changes; final overall line coverage must
not be lower than the starting value. No fixed minimum percentage applies.

Also run the phase 0 benchmark and ignored scenarios against final v2 and compare with the
saved baseline report. Results must include not only average time but history and viewport
parameters, peak memory, replaced-frame count, and scroll-assertion status.

## Manual verification

1. Launch the final build and confirm that every new Bash/Zsh terminal session receives v2
   without a user-visible protocol selector.
2. Complete the root/nested Bash/Zsh lifecycle, manual scroll/resize/selection matrix, one
   million lines of burst output, and off-screen copy/save.
3. Compare results with `baseline-results.md`: v2 must satisfy the
   [load criteria](spec.md#load-criteria), and any regression is documented and blocks release.
4. Force bootstrap or protocol failure. The terminal must remain usable, the UI must show the
   `Degraded` reason, and no v1 fallback may start.
5. Send an old v1 DCS event. It must be safely ignored without creating a block/event or panic.
6. Search the repository for `otty-dcs;block`, `BlockEvent`, `BlockPhase`, the old stitched
   snapshot, and runtime switches. Production references must be absent; v1 framing is allowed
   only in the legacy-ignore test fixture and historical notes.
7. From a clean environment, use the final READMEs to configure Bash/Zsh and reproduce active,
   degraded, and unsupported states without knowledge of internals.

This phase and Blocks v2 as a whole are complete only when v2 is the sole production path, old
code is removed before release, every command above passes, and the shared
[Definition of Done](spec.md#definition-of-done) is confirmed by the manual acceptance matrix.
