# Task 07 – Validation and quality (roadmap item 7)

Goal: Ensure correctness, performance, and debuggability of the new API.

What to do:
- Add tests for frame export correctness, damage tracking, channel delivery order, runtime request handling, and pending-output reporting.
- Add benchmarks for snapshot/damage overhead and parser→surface→frame throughput; track baselines.
- Add fuzzing where feasible (escape parsing to surface actions) to catch regressions.
- Optional: add metrics/tracing hooks (feature-gated) for throughput/latency and configurable sync-mode limits.
- Document test/bench targets and how to run them (per-crate or workspace).

Deliverables:
- Test/bench/fuzz artifacts in-tree.
- Brief docs outlining validation steps and pass criteria.

Dependencies:
- Tasks 01–06 implemented to exercise end-to-end flows.
