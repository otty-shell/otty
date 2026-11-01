# 16 — Live updates from engine events

Status: resolved
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

## Answer

While Docker mode is active, OTTY watches `/events` of every reached
engine and reloads the list an event changed. No new dependencies, no new
text (nothing new is shown; a failed reload uses the existing Notice).

### `crates/docker`

- `events.rs`: `ListKind` (`Containers`, `Images`, `Volumes`, `Networks`)
  and the pure `ListKind::from_event(&EventMessage)`. The object type
  decides, with three exceptions: container actions of someone using a
  container (`exec_*`, `attach`, `detach`, `resize`, `top`,
  `archive-path`, `extract-to-dir`, `export`) change nothing, so a Shell
  tab or a resized terminal does not reload anything; network `connect` /
  `disconnect` change the container list (the container's networks, and
  with them "used by" of networks); volume `mount` / `unmount` change
  nothing on their own (the container start or stop around them already
  reloads). `event_filters()` asks the engine only for
  `type=container,image,volume,network`, so Podman's `pod` events (a type
  bollard cannot read, which would break the stream) never arrive.
- `debounce.rs`: the pure `ReloadDebounce`: per list, the first change
  reloads at once; changes inside the 500 ms after a reload wait, and the
  list reloads once more when the 500 ms are over, so the table ends on the
  last state.
- `watch.rs`: `Engine::watch() -> impl Stream<Item = WatchEvent>`, which
  never ends on its own. `WatchEvent` = `Changed(ListKind)` (at most once
  per 500 ms per list), `Reconnected` (the stream broke and is open again;
  events in between were missed) and `Unreachable` (the engine does not
  answer `/_ping`). When the stream ends or breaks, it waits 1 s, 2 s, 4 s,
  … up to 30 s (`reconnect_delay`, pure), pings, and then opens the stream
  again or says `Unreachable` and tries again later. An event that comes
  through sets the delay back to 1 s. The timers live here because the
  crate already has tokio's `time`; the app has no timer to sleep on.
- The watch uses the `Hash` derives on `EngineAddress`, `EngineKind` and
  `EngineInfo` (the subscription key) and the crate-private
  `Engine::client()` that ticket 13 added too; `watch.rs` keeps its own
  `impl Engine` block.
- bollard's 30 s request limit covers only the wait for the answer's
  headers. Docker 29 sends them at once for `/events` (checked with
  `curl`), so the limit does not cut the stream.

### App

- `widgets/docker/subscription.rs`: one `Subscription::run_with` per
  reached engine, keyed by its `EngineInfo`, so a search that finds the
  engine again keeps its watch and one that drops it stops it.
  `watch_intent` maps `Changed` → `DockerIntent::ListChanged { engine,
  resource }`, `Reconnected` → `EngineReconnected`, `Unreachable` →
  `EngineUnreachable`. An engine whose socket file is gone (bollard
  refuses to make a client) counts as unreachable.
- `DockerWidget::subscription()` is one batch of the engine watch
  (`engine_watch`, nothing unless Docker mode is active; `is_active` is no
  longer test-only) and ticket 13's followed logs (`followed_logs`).
  `app/src/subscription.rs` adds it to the app subscriptions.
- With ticket 13's inspector: an event reload that keeps the followed
  container keeps its log lines; one that drops it moves the logs to the
  new selection (tested).
- Reducer (all only while active and for a known engine):
  - `ListChanged(Containers)` → `DockerEffect::LoadList` for the containers.
  - `ListChanged(Images | Volumes | Networks)` → `LoadList` for that list
    with the containers shown now (for "used by").
  - `ListLoaded(Containers(Ok))` → applies it, then `LoadList` for images,
    volumes and networks with the new container list (ticket 08 follow-up:
    "used by" stays right).
  - `ListLoaded(Containers(Err))` (for example a container state bollard
    does not know) → the containers table shows the engine's message in
    the existing `Notice`; the other lists stay as they were and nothing
    more loads.
  - `EngineReconnected` → `LoadLists` (all four).
  - `EngineUnreachable` → a new search (`Discover`), unless one runs;
    ticket 10's cards show the result.
- `types::LoadedList` (one list's result), `DockerState::apply_list`.
- `events/docker.rs`: `LoadList` → `Task::perform(load_list(..))` →
  `ListLoaded`.
- `guards.rs`: the four new intents are background events, so a live
  update does not close an open menu or an inline edit.

### Tests (written first, seen failing)

- Crate: mapping of the captured Docker 29.9.1 events
  (`tests/fixtures/docker-events.jsonl`: `docker run --rm` of
  `busybox:1.37`, a volume and a network created and removed, the image
  removed), terminal-use actions, health status, volume mounts, other
  types, the type filter; debounce (first change at once, changes inside
  the gap give one more reload, a waiting reload starts a new gap, lists
  are independent, order); retry delays; against the fake engine on a unix
  socket (streamed answers added to `test_support`): `changes()` reads and
  ends with the stream and sends the filter, ping, the watch sends a
  trailing reload after the gap, reconnects after a closed stream with
  `/_ping` then `/events` and says `Reconnected`, and says `Unreachable`
  twice with a growing delay for a gone engine (15 repeated runs, no
  flakes).
- App: subscription units (one per engine, none without engines, none
  outside Docker mode through the widget), event → intent mapping, the
  reducer cases above (inactive mode and unknown engines start nothing),
  the effect runs one task, guards.
- Live (`#[ignore]`, `crates/docker/tests/live_engine.rs`
  `watching_a_live_engine_sees_its_changes`): against this machine's
  Docker, `docker run --rm --name otty-fx-evt busybox:1.37 true` gave
  `Changed(Images)` (pull), then `Changed(Containers)` three times 0.5 s
  apart, then `Changed(Images)` for the removal. The image was removed
  again afterwards.

Coverage (lines): 83.83% on `6b6a50f` → 84.02%.

### Deviations

- The 500 ms limit is per list for event reloads. The images, volumes and
  networks reloads that follow a new container list are not limited
  separately, so with steady container and image events the image list can
  load up to four times a second.
- The GUI was not run against a live engine in this session; the manual
  "`docker stop` updates the table within a second" check was done at the
  crate level (the watch sends the change within milliseconds of the
  event).
- Podman's `/events` was not checked live (no Podman service was
  running); if Podman sent its headers only with the first event, the 30 s
  request limit would end an idle stream, and the watch would reconnect
  and reload every 30–60 s instead of failing.
