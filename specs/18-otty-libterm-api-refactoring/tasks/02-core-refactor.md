# Task 02 – Core refactor (roadmap item 1)

Goal: Rebase `otty-libterm` core to `TerminalEngine` that owns session/parser/surface, emits owned frames, and contains no runtime glue.

What to do:
- Rename `Terminal` → `TerminalEngine`; drop `TerminalClient` storage and trait-based callbacks.
- Replace `TerminalSnapshot` usage with `SnapshotOwned`/`SnapshotView` from Task 01.
- Keep request processing (resize, scroll, selection, shutdown, write) but make it independent of any runtime API.
- Provide clear methods: `on_readable`, `on_writable`, `tick`, `has_pending_output`, `queue_request` (or equivalent), all returning `Result`.
- Maintain sync-update buffering logic, reworked to operate with the new surface snapshot model.
- Revise errors/types re-exports as needed; update docs explaining thread/ownership expectations.

Deliverables:
- New `TerminalEngine` type and supporting modules in `otty-libterm`.
- Removed legacy snapshot/client plumbing.
- Updated examples/tests to compile with the new core API (stub runtime hook kept for later tasks).

Dependencies:
- Task 01 (SnapshotOwned/SurfaceModel ready).
