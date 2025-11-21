# Task 06 – Builders and documentation (roadmap item 6)

Goal: Provide ergonomic builders/presets and update user-facing docs/examples for the new API.

Current state:
- `TerminalEngine` + request/event channels exist; mio `Runtime` now uses the `Driver` trait.
- Examples (`simple`, `unix_shell`) still hand-wire session/parser/surface; README documents the manual flow.
- No `TerminalBuilder`, no unix/ssh presets, no tokio example, and docs don’t differentiate runtime vs manual driver choices.

What to do:
- Implement `TerminalBuilder` with presets (`unix`, `ssh`) plus hooks to inject custom session/surface/parser and config knobs (size, buffers/damage, driver choice).
- Builder returns `(engine, events, handle)` and optionally a runtime-ready tuple compatible with the existing mio `Runtime`.
- Refresh README/guide to prefer builder-first usage, explain runtime vs manual driver wiring, and include detailed examples (sync runtime, tokio, manual, custom modules, damage-aware rendering).
- Update existing examples (`simple`, `unix_shell`) to the builder API and add a tokio example.

Deliverables:
- Builder API in `otty-libterm`.
- Updated documentation and examples reflecting the new design.

Dependencies:
- Tasks 01–05 (core pieces available).
