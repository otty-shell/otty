# Task 07 – Validation and quality (roadmap item 7)

Goal: Ensure correctness, performance, and debuggability of the new API.

Current state:
- Core refactor + builders are in and examples use `TerminalBuilder` / mio runtime.
- Unit tests exist for channel backpressure/order, runtime request handling/interest toggling, pending-output tracking, and surface snapshot/damage export.
- No benchmarks, no fuzzing, and no documented validation regimen beyond unit tests.

What to do (focus on gaps):
- Tests: add end-to-end coverage that exercises parser→surface→frame export with damage, verifies event ordering (frame before exit, title/bell/cursor/hyperlink propagation), runtime request plumbing with bounded channels, and pending-output reporting across partial writes.
- Benches: add Criterion benchmarks for snapshot/damage export and parser→surface→frame throughput; record baseline results in docs.
- Fuzz: add cargo-fuzz (or similar) target that drives escape parsing into surface actions to catch regressions.
- Diagnostics: optional feature-gated metrics/tracing hooks for throughput/latency and configurable sync-mode limits; keep disabled by default.
- Docs: describe how to run tests/benches/fuzz (per-crate or workspace) and what constitutes “pass” (e.g., benches tracked via git-committed baseline or README table).

Deliverables:
- In-tree test/bench/fuzz artifacts.
- Brief docs outlining validation steps and pass criteria.

Dependencies:
- Tasks 01–06 are implemented; use the new builder/runtime plumbing for end-to-end validation.
