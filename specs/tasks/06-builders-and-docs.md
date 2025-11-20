# Task 06 – Builders and documentation (roadmap item 6)

Goal: Provide ergonomic builders/presets and update user-facing docs/examples for the new API.

What to do:
- Implement `TerminalBuilder` with presets (`unix`, `ssh`, custom session/surface/parser injection) and config knobs (size, buffers, damage options, driver choice).
- Ensure builder returns `(engine, events, handle)` bundles and can optionally hand back a runtime-ready tuple.
- Refresh README/guide with threading/ownership guidance, runtime-vs-manual driver guidance, and detailed examples (sync runtime, tokio, manual, custom modules, damage-aware rendering).
- Update existing examples (`simple`, `unix_shell`) to the new API; add a tokio example if not already present.

Deliverables:
- Builder API in `otty-libterm`.
- Updated documentation and examples reflecting the new design.

Dependencies:
- Tasks 01–05 (core pieces available).
