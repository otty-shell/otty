# Task 05 – Requests and writes (roadmap item 5)

Goal: Lock down input/request handling now that the raw-bytes pipeline exists, adding batching ergonomics and guidance.

What’s already covered in code:
- `TerminalRequest::WriteBytes` is the only input path; `TerminalEngine` queues bytes (`pending_input`), handles partial writes, and reports `has_pending_output` to runtimes/manual loops.
- Mio runtime toggles writable interest based on `has_pending_output`, so runtime + manual-driver modes already work with the write queue.

What to do:
- Keep `TerminalRequest::WriteBytes` as the sole input path and explicitly document encoding responsibility (front-end turns input into bytes).
- Add an optional helper for batching/coalescing writes (e.g., on `TerminalHandle` or a thin adapter) that safely flushes pending output for large pastes.
- Document recommended buffering patterns: chunking large pastes, pacing writes, and leaving higher-level input encoders outside `libterm`.
- Add tests that exercise partial writes/backpressure and verify `has_pending_output` behavior on both queued and drained writes.

Deliverables:
- Optional batching helper + any small plumbing needed in `otty-libterm`.
- Docs/comments capturing encoding expectations and safe streaming guidance.
- Tests covering partial writes, backpressure, and pending-output reporting.

Dependencies:
- Task 03 (handle/requests exist).
