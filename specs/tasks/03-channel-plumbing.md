# Task 03 – Channel plumbing (roadmap item 3)

Goal: Deliver channel-based event delivery and request handles so embedders consume terminal events without callbacks.

What to do:
- Introduce `TerminalEvents` receivers (sync + async variants) and `TerminalHandle` senders for `TerminalRequest`.
- Wire `TerminalEngine` to emit `TerminalEvent::Frame(FrameOwned)` and other events into channels.
- Ensure frames/events are `'static` and cheaply cloneable via `Arc`.
- Provide fallible `send` semantics (wake mechanisms for runtimes can come in Task 04).
- Add docs explaining usage patterns (UI thread, async tasks).

Deliverables:
- Channel abstractions in `otty-libterm`.
- Engine integration producing events through channels.
- Tests covering event ordering (frame before child-exit), backpressure behavior (e.g., bounded/unbounded decision), and basic send/recv flows.

Dependencies:
- Task 02 (engine emits frames).
