# Task 05 – Requests and writes (roadmap item 5)

Goal: Finalize request/input handling with a raw-bytes pipeline and optional batching.

What to do:
- Keep `TerminalRequest::WriteBytes` as the sole input path; clarify expectations (front-end encodes).
- Add optional helper for batching/coalescing writes and safe flushing of pending output within `TerminalHandle`/engine.
- Document how to structure higher-level input encoders externally (out of scope for the core) and how to stream large pastes safely.
- Ensure write path works with both runtime-driven and manual-driver modes.

Deliverables:
- Updated request handling in `otty-libterm`.
- Docs/comments describing encoding responsibility and recommended buffering patterns.
- Tests covering partial writes, backpressure, and pending-output reporting.

Dependencies:
- Task 03 (handle/requests exist).
