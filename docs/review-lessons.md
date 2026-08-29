# Review-derived development lessons

This document records the conclusions from the repository review threads and
PR discussions for #85, #87, #89, #95, #96, #97, and #98 as of 2026-08-29.
It is an operational checklist for future work, not a list of reviewer
preferences.

## Conclusions by pull request

### #85: pane splitting and equalization

- Closing a pane did not rebalance the surviving layout. The result depended
  on the tree shape and the pane selected for closing; a passing split test did
  not prove the close path.
- A global keyboard handler could split a terminal while Settings had focus.
  Application shortcuts belong to the focused terminal widget and must route
  using the reporting terminal identifier.
- A held shortcut can create repeated panes and shell processes unless repeat
  events are rejected at the widget boundary.
- A shortcut must consume its application binding before the PTY text fallback,
  while unbound modified keys must retain the platform fallback. A non-macOS
  `COMMAND` mapping must not replace Ctrl+D EOF behavior.
- Shortcut-driven and context-menu-driven operations must keep dependent views,
  such as the explorer, synchronized through the same business path.
- Equalization was a product default question, not only an implementation
  detail. The behavior became configurable, with the issue's default retained.
- Unrelated lint fixes, README edits, test-data rewrites, skill files, and
  generated or moved documents made the PR harder to review and had to be
  removed or justified separately.
- Layout tests must use production spacing and an explicit pixel tolerance;
  ideal zero-spacing ratios are insufficient evidence.

### #87 and #89: platform cleanup and Finder paste

- Rebase stacked branches onto current `main` before asking for review. A
  branch that is logically correct can still present stale or duplicate files.
- Keep platform-specific code and imports minimal. Separate macOS and
  non-macOS implementations when that makes ownership and compilation clear.
- When a reviewer asks for a structural simplification, verify all callers and
  behavior before applying it; do not preserve speculative abstractions.

### #95: local shell working directory

- The original issue named macOS, but the reviewer identified the same useful
  behavior on Linux. The accepted scope is now macOS and Linux: local shell
  sessions use `HOME`; other platforms retain their existing inherited
  directory behavior.
- Platform scope must be reflected in both implementation and regression tests.
  A macOS-only test cannot validate the Linux branch.
- A short issue with no reproduction steps leaves product scope ambiguous.
  Resolve that ambiguity explicitly before changing additional platforms.

### #96: asynchronous settings saves

- Save completion must carry the revision it persisted. A completion must not
  replace a newer draft, and failed saves must remain retryable.
- Happy-path tests are insufficient for asynchronous state. Cover in-flight
  saves, newer edits, stale completions, reset or reload, failure, and retry.
- A full green CI run proves compilation and the checked test cases, not that
  every event ordering is correct.

### #97: double-click coverage stabilization

- The test was redundant after equivalent stabilization landed in `main` via
  #89. Close obsolete PRs instead of preserving duplicate behavior.
- Tests that construct a double click through wall-clock scheduling are fragile
  under coverage and host load. Build the intended event deterministically and
  bound retries.

### #98: Windows SSH readiness

- Raw libssh2 socket I/O and mio readiness are separate layers on Windows.
  A `WouldBlock` path must re-arm readiness without consuming pending data.
- Unix and Windows behavior require separate evidence. macOS/Linux CI does not
  prove a Windows interactive SSH session.
- Keep real Windows runtime verification explicitly listed as unverified when
  no Windows host or equivalent CI job ran it.

## Mandatory pre-review checklist

1. **Contract:** record the issue, supported platforms, defaults, operation
   order, and a minimal reproduction.
2. **Scope:** compare with current `main`; remove unrelated fixes and classify
   every changed file by issue lineage.
3. **Threads:** query all review threads through GraphQL and classify each
   comment as `ACCEPT`, `REJECT`, or `CLARIFY` before editing.
4. **Behavior:** trace focus, ownership, routing, side effects, shutdown, and
   alternate user paths; add behavior tests for each load-bearing branch.
5. **Concurrency:** test stale asynchronous results, newer state, reset or
   reload, failure, and retry.
6. **Platforms:** test every target branch through CI or a portable selector;
   report real-OS and real-device gaps separately.
7. **Stability:** remove ambient environment, wall-clock scheduling, and
   synthetic production parameters from tests.
8. **Evidence:** run all repository gates, execute the real user operation, and
   do not present unit or CI success as GUI, hardware, or production proof.
9. **Handoff:** push the verified diff before replying to review threads; do
   not resolve threads without explicit authorization.

## Root cause of repeated review discoveries

The recurring failure mode was treating green tests and a plausible diff as the
completion boundary. The reviews exposed behavior outside that boundary:
alternate user actions, focus ownership, platform-specific semantics, stale
async results, actual rendered geometry, and maintainer scope expectations.
The checklist above makes those dimensions explicit before review so human
review is used for product judgment rather than basic regression discovery.
