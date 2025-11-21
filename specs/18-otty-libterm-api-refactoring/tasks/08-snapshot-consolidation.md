# Task 08 – Snapshot consolidation (post-core refactor)

Outcome: Snapshot naming cleanup completed and borrowed snapshot API removed.

Current state after completion:
- Engine emits owned snapshots (`SnapshotOwned`) and no longer uses `TerminalSnapshot`.
- `otty-surface` exposes only the owned snapshot path; `SurfaceSnapshot`/`SurfaceSnapshotSource`/`Surface::snapshot()` were removed.
- Public API now uses `Snapshot*` names; docs/examples/benchmarks reference the new names.

Notes:
- Selection/cursor/damage/display offset semantics preserved during rename.
