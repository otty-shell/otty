# 16 — Live updates from engine events

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Spec Decision 14. Tests first.

- A subscription per engine reads `/events` while Docker mode is active.
- Container, image, volume and network events reload that list, at most once per 500 ms per list (pure, tested debounce).
- When the stream ends, reconnect with a growing delay; when the engine is gone, run discovery again (ticket 10 shows the result).

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Tests: the subscription exists only in Docker mode; event type → list mapping; debounce.
- Manual: `docker stop` in another terminal updates the table within a second.
