# Task 08 – Snapshot consolidation (post-core refactor)

Goal: Remove legacy borrowed snapshot APIs from `otty-surface`/`otty-libterm` and adopt the new owned frame snapshot as the primary interface (renamed to `Snapshot`).

What to do:
- Rename `FrameOwned`/`FrameView`/`FrameDamage`/`FrameCell`/`FrameSize` to `Snapshot` equivalents (e.g., `SnapshotOwned`, `SnapshotView`, etc.) for the public API.
- Remove `SurfaceSnapshot`/`SurfaceSnapshotSource` and related methods once the engine consumes the owned snapshot path.
- Update `SurfaceModel` to export the renamed owned snapshot.
- Migrate `otty-libterm` to the new names and drop `TerminalSnapshot<'a>`/borrowed surface snapshot usage.
- Adjust docs/examples/tests to reference the new snapshot type; ensure selection/cursor/damage semantics are preserved.

Dependencies:
- Core refactor and channel/runtime tasks completed (engine in place using owned snapshots).

Notes:
- Coordinate with downstream changes to avoid breaking intermediate work; perform this once `SurfaceSnapshot` is no longer used in code.
