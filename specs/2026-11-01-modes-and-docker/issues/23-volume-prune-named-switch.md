# 23 — Volume prune: anonymous only by default, switch for named volumes

Status: resolved
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 12
Spec: [../spec.md](../spec.md)

## What

Ticket 12 made "Prune unused volumes" remove named unused volumes too (it
sends `all=true` to Docker 1.42+, and Podman removes all unused volumes by
default). A named volume of a stopped project, for example a database volume,
is user data. The owner chose the Docker CLI behaviour (spec Decision 11,
2026-11-01). Tests first.

- By default Prune removes only anonymous unused volumes, on Docker and on
  Podman. The confirm dialog gets a switch "Also remove named volumes", off
  by default; on, it removes every unused volume (like `docker volume prune
  -a`).
- Docker: without the switch send no `all` filter on 1.42+ (anonymous only is
  its default); with it send `all=true`. Older Docker: anonymous only is not
  possible to tell apart on the engine side — research and note in
  `## Comments` what Docker before API 1.42 removes, and choose the safe
  behaviour.
- Podman removes all unused volumes on prune. Without the switch, remove only
  the anonymous unused ones one by one (`Volume::is_anonymous()` and no
  container uses it) instead of calling prune; with it, call prune.
- The dialog's count follows the switch: it counts what will really go.

All new text goes through `i18n` in the ten locales (spec Decision 17). Run
fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the
merge base before the commit.

## Done when

- Crate tests against the fake engine: Docker request with and without
  `all=true`; Podman without the switch removes only anonymous unused volumes
  and never calls prune; with the switch calls prune.
- Reducer and view model tests: the switch is off by default, the count
  follows it, the effect carries the choice.
- `iced_test`: the switch in the dialog emits its intent.

## Comments

Research: what Docker before API 1.42 removes on volume prune.

- Docker 20.10 (`volume/service/service.go`, `Prune`): removes every local
  volume with no container, named or not. It only accepts the `label` and
  `label!` filters (`acceptedPruneFilters`), so `all=true` is rejected as an
  invalid filter. There is no way to ask for "anonymous only".
- Docker 23 (API 1.42) added the `all` filter and made the default
  anonymous only. In `api/server/router/volume/volume_routes.go` the
  daemon adds `all=true` itself when the client's API version is below
  1.42, to keep the old behaviour. So an old API version never narrows,
  whatever the filters say.
- Podman removes all unused volumes on prune (ticket 12).

Safe choice: when the engine cannot narrow the prune (Podman, Docker with
API below 1.42, or an unknown API version) and named volumes are not
included, OTTY does not call prune. It lists containers and volumes and
removes the unused anonymous volumes one by one. Calling prune there
would delete named volumes, which is user data. An unknown API version
counts as old, which is the safe side.

## Answer

Volume prune keeps named volumes unless the user asks for them.

### `otty-docker`

- `Engine::prune_volumes(include_named)` in
  `connection/resource_actions.rs`:
  - Docker with API 1.42+: `POST /volumes/prune` with no filter, or with
    `all=true` when `include_named` is set.
  - Podman, older Docker, unknown API version, `include_named` off: no
    prune call; unused anonymous volumes (`Volume::is_anonymous()` and no
    container mounts them) are removed one by one.
  - Same engines, `include_named` on: prune with no filter (it removes
    all unused volumes there anyway, and old Docker rejects `all`).
- `volume_prune_options` became `prune_can_skip_named_volumes`.

### App

- `EngineAction::PruneVolumes { include_named }`; the confirm switch sets
  it (`with_switch`). The run effect calls `prune_volumes(include_named)`.
- The Prune dialog of Volumes has the kit `Switch` "Also remove named
  volumes", off by default. `Confirm` keeps the count for both switch
  states and `count()` follows the switch: unused anonymous volumes when
  off, all unused volumes when on.
- New i18n key `DockerAlsoRemoveNamedVolumes` in the ten locales;
  `Key::ALL` is 341.

### Tests (written first, seen failing)

- Crate, fake engine: Docker 1.44 without and with `all=true`; Podman,
  Docker 1.41 and Docker with unknown version remove only the unused
  anonymous volume and never call prune; Podman and Docker 1.41 with the
  switch call prune without filters.
- Reducer and view model: switch off by default, effect carries the choice,
  count follows the switch both ways.
- `iced_test`: the switch in the dialog emits `ConfirmSwitchToggled`.
