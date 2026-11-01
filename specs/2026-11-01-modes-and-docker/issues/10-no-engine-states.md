# 10 — No engine states, open links, copy command

Status: resolved
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 06, 07
Spec: [../spec.md](../spec.md)

## What

When discovery finds no engine, Docker mode shows why and what to do
(spec Decision 9). Screens: `DockerNoEngine`, `DockerEngineStopped`,
`DockerPodmanOff`, `DockerNoAccess`.

- Kit `EmptyCard` in the main area with the title, text, command and buttons of each case; the "looked for" line under it; left panel text "No engine found." and the like; empty inspector.
- Check again reruns discovery without a restart.
- Copy writes the command with the iced clipboard.
- `open_url` in `app/src/services.rs`: `xdg-open` on Linux, `open` on macOS, through `std::process::Command`; a failure is logged.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- View model tests for each case (texts, command, links).
- `iced_test`: Install, How to fix, Copy and Check again emit their messages.
- Manual: each case on a real machine (no engine; `systemctl stop docker`; `systemctl --user stop podman.socket`; a user outside the docker group).

## Answer

The Docker main area shows one card per no-engine case (spec Decision 9).

### Files

- `app/src/widgets/docker/no_engine.rs`: `NoEngineCase`, `NoEngineViewModel` (title, description, left panel text, command with label and hint, install links, "How to fix" page, "looked for" line). `no_engine(&Discovery)` maps `Diagnosis` to a card (`None` when an engine answered); `view_model(case, socket, places)` holds the texts, commands and URLs. The "looked for" line lists every probed socket (`$DOCKER_HOST (...)` when it came from that variable).
- `model.rs`: `MainViewModel::NoEngine(NoEngineViewModel)`; `panel_note` gives the left panel text.
- `event.rs` / `reducer.rs`: intents `CheckAgain` (new search plus `Discover`, the same path as `Enter`), `CopyCommand`, `OpenLink`; effects `CopyToClipboard`, `OpenUrl`.
- `events/docker.rs`: `CopyToClipboard` uses `iced::clipboard::write`; `OpenUrl` calls `services::open_url`.
- `services.rs`: `open_url` runs `xdg-open` (Linux) or `open` (macOS) with `std::process::Command`; a failure is logged.
- `view/no_engine.rs`: the card (kit `EmptyCard`, `ActionTile`, `Button`, `CommandBox`, `divider`). `view/engine_tree.rs` shows the left panel text; `view/inspector.rs` shows "Nothing to inspect."
- Kit: new `CommandBox` (mono command with a Copy button) and `ActionTile::note` (muted mono site name), both in the gallery with `iced_test`s. New icon `external-link.svg`.
- i18n: 24 keys in the ten locales; `docker_no_engine` removed; `Key::ALL` is 251.

### Tests (written first, seen failing)

View model per case (texts, command, links, looked-for line, socket in the no-access text), `no_engine(&Discovery)`, model (card and panel text, none while searching), reducer (Check again starts a search, finds an engine started meanwhile, copy and open effects), `iced_test` (Install Docker/Podman, How to fix per case, Copy per case, Check again on every case, main area, left note, inspector hint), kit (`CommandBox`, `ActionTile` note).

### Deviations

- "How to fix" has no arrow icon (kit `Button` has none); the install tiles have it.
- The macOS notes show on every platform (as on the screens).
- The status bar text of the screens ("docker is not running") is not done: the Docker status bar is ticket 18.
- Only `NotInstalled` can be built from `otty_docker::fixtures`, so the other three cases are tested through `view_model`; `NoPermission` from a `Discovery` is not tested (`SocketCandidate::new` is crate-private).
- The card is 460 px wide (kit `EmptyCard`), the screens use 520 px.
- With no probes at all the line reads "OTTY looked for , and for ..."; a real search always has probes.
