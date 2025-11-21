# Task 04 – Drivers and runtime (roadmap item 4)

Goal: Define a driver interface and refit the bundled runtime while allowing manual/tokio loops.

Current state:
- `otty-libterm` already ships a mio-based `Runtime` + `RuntimeClient` (readable/writable/waker/tick plumbing) and `TerminalEngine` exposes `on_readable` / `on_writable` / `tick` / `has_pending_output` / `check_child_exit`.
- There is no driver abstraction that cleanly wraps `TerminalEngine`; the runtime is still bespoke (`RuntimeClient` + `RuntimeEvent`).
- No tokio/manual-loop examples or runtime tests exist yet.

What to do:
- Introduce a `Driver` (or equivalent) trait that directly wraps `TerminalEngine` (readable/writable/tick/request/deadline). Either adapt or replace `RuntimeClient`/`RuntimeEvent` with this abstraction.
- Port the existing mio-based `Runtime` onto that trait, keeping the simple `Runtime::run(engine)` entrypoint and ensuring the PTY registration/waker flow still matches `Session`/`Pollable` tokens.
- Provide examples: a tokio driver (readiness + writable + tick) and a manual-loop helper that shows custom pollers using the new driver API.
- Document how to choose between the built-in runtime and custom loops, noting the role of wake tokens and request plumbing.
- Add tests that cover runtime request handling, shutdown, child-exit detection, deadline-driven ticks, and writable interest toggling when `has_pending_output` is true/false.

Deliverables:
- Updated runtime code in `otty-libterm` using the new driver interface (or a cleaned-up replacement for `RuntimeClient`).
- Tokio example plus notes/manual helper for custom pollers.
- Tests for runtime request handling, shutdown, child-exit detection, writable-interest toggling, and deadline-driven ticks.

Dependencies:
- Task 03 (channels) and Task 02 (engine ready).
