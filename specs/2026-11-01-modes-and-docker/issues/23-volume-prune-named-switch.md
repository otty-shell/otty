# 23 — Volume prune: anonymous only by default, switch for named volumes

Status: ready-for-agent
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
