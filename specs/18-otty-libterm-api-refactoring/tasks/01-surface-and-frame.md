# Task 01 – Surface snapshots and damage (roadmap item 2)

Goal: Give `otty-surface` owned snapshot export that produces `SnapshotOwned`/`SnapshotView` for renderers, reusing existing damage tracking.

What to do:
- Introduce `SnapshotOwned`/`SnapshotView` types (owned data, `Arc`-cloneable, `'static`) containing size, cursor, mode, display offset, damage information, and visible cells/spans.
- Reuse current damage machinery (`SurfaceDamageState`, `SurfaceDamage`, `LineDamageBounds`) to populate damage in frames; add any missing flags (e.g., title/full clear markers) if needed.
- Replace `SurfaceActor + SurfaceSnapshotSource` with a new `SurfaceModel` trait that applies actions and exports owned snapshots (keeping existing `Surface` as default impl).
- Ensure owned snapshot export accounts for wide chars, zero-width, selection, cursor visibility/mode, scrollback display offset, and includes the current damage view.

Deliverables:
- New/updated types in `otty-surface` with docs.
- Updated surface implementation using `SurfaceModel` and emitting damage into owned snapshots.
- Unit tests for owned snapshot correctness and damage propagation (basic ops: print, scroll, resize, selection).

Dependencies:
- None ahead; foundations for later tasks.
