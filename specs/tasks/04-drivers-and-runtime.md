# Task 04 – Drivers and runtime (roadmap item 4)

Goal: Define a driver interface and refit the bundled runtime while allowing manual/tokio loops.

What to do:
- Define `Driver` (or similar) trait on top of `TerminalEngine`: `on_readable`, `on_writable`, `tick`, `queue(request)`, `next_deadline`.
- Port existing mio-based `Runtime` to use the driver abstraction, keeping the simple `Runtime::run(engine)` API.
- Add a tokio driver example (readiness + writeability + tick) and a manual-loop helper showcasing custom pollers.
- Document how the runtime hides readiness handling (for users who don’t want manual poller wiring) and how to integrate custom loops.
- Ensure wake/registration tokens remain compatible with `Session`/`Pollable` traits.

Deliverables:
- Updated runtime code in `otty-libterm` using the new driver interface.
- Tokio example plus notes for custom pollers.
- Tests for runtime request handling, shutdown, child-exit detection, and deadline-driven ticks.

Dependencies:
- Task 03 (channels) and Task 02 (engine ready).
