# 18 — Status bar per mode

Status: resolved
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Docker mode status bar as on the boards: engine and version, the counts of
the open resource ("6 containers, 3 running", "7 images, 3.0 GB", "4
volumes", "5 networks"), "<other engine> also found" when there are two, and
the sessions button. Terminal mode unchanged.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- View model tests for each resource and for one and two engines.

## Answer

The Docker mode status bar shows the engine and the counts as on the boards.

### Files

- `app/src/widgets/docker/status.rs`: `StatusViewModel` (engine label, info text, "also found" note) and `status_bar(&DockerState)`. Containers: "N containers, M running"; Images: "N images, <size>" (sum of image sizes through `format::size_text`); Volumes and Networks: the count. One item uses the singular. A list that loads shows "Loading…", a failed list shows "failed"; no number is invented. Searching with no engine known: "Looking for engines…". No engine: the case's `status_text`. Other engines: "<label> also found".
- `no_engine.rs`: `NoEngineViewModel::status_text` ("no engine found", "docker is not running", "podman socket is off", "permission denied on <socket>").
- `view/status_bar.rs`: kit `StatusBar` with the engine (local dot) or "Docker mode" on the left, then the text; the right side has "also found", the sessions button and `OTTY <version>`.
- `view.rs`: passes the view model, the session count and the sessions intent. `model.rs`: `no_engine_view` is now `pub(super)`.
- i18n: 15 keys in the ten locales (singular and plural keys for the four counts, status texts, mode name, "also found"); `Key::ALL` is 266. Russian uses the "Containers: N" form, as the existing session count does.

### Tests

View model (every resource, singular, one and two engines, searching, loading, failed, no engine, each no-engine status text); `iced_test` (bar texts with and without an engine, the sessions button sends its message, and in the full view it sends `Select(Terminal)`).

### Deviations

- Terminal mode's session count is plain text, not a button. In Docker mode the screens draw a button, so it is a button that switches to Terminal mode (where the sessions live). Terminal's status bar is unchanged.
- Counts are joined with ", " in every locale.
