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

## Comments

### The whole way from a lost engine to the card (2026-11-04, spec Decision 9)

Owner request: when the only engine is off, do not open it, show this page. Tests
in `model.rs` now run the whole chain through the reducer: `EngineUnreachable`,
the search it asks for, then `DiscoveryFinished` with Docker not running, the
Podman API off and nothing installed, for Docker and for Podman, with the engine
open and closed in the tree. They check the card, its command or install buttons,
the left note, the status bar, an empty tree and an empty inspector. The fixtures
now build these cases, which the Deviations above said they could not.

The chain held except in two places, both fixed:

- An open Run form hid the card, because `view.rs` draws the form first. The form
  now waits as typed and comes back with an engine.
- A row menu stayed open with nothing drawn, which kept `has_overlay` true, and a
  notice of the lost list showed again on the next table. Both go when the place
  moves.

A confirmation or create dialog stays over the card until the user closes it.

### Two commands when both are installed (2026-11-04, spec Decision 9)

Owner decision: with both CLIs and no answer, the card used to say Docker was not
running, also to someone who only uses Podman. `Diagnosis::DockerAndPodmanOff` and
`NoEngineCase::DockerAndPodmanOff` now cover it.

Design note: the card is the same `EmptyCard` as the others. Under the description
it has two command blocks, Docker first as in the search order, each with a label
that names its engine, a `CommandBox` with its own Copy, and its own macOS note.
The button row holds Check again and one How to fix per engine; it wraps, so long
translations stay inside the card. The view model now has `commands` and `fixes`
lists; every other case has one of each, or none. No board on the canvas shows this
card yet. At the smallest window the longest locales leave no spare room for the
"looked for" line; the iced test simulator measured this with its fallback font,
so check it by eye (checklist step 15e).

The denied-socket cards stay as they are, as asked. They still come first, so
someone with a denied Docker socket and Podman off sees only the docker group
command.

### Looking for an engine while the card shows (2026-11-04, spec Decision 9)

Owner decision: the card watches by itself. `subscription::engine_search_timer`
ticks every 5 seconds only while `DockerState::shows_no_engine_card` holds in active
Docker mode. A tick sets `is_background_search_running` and asks for
`DockerEffect::DiscoverInBackground`; `state.search` stays `Done`, so the view model
keeps building the same card. Its result is a separate intent,
`BackgroundSearchFinished`, because it may only count while the card is on screen:
a result that comes during Check again, with an engine shown or after leaving the
mode is dropped.

What one search costs, read from the code and not measured: `stat` on each candidate
socket (up to about 8 paths); a connect and a `/version` request on each socket that
exists, 2 seconds at most each and all at once; `stat` on `docker` and `podman` in
each `PATH` folder and 5 install folders; one `docker --version` process when
`docker` is found and is not a link to `podman`, killed after 1.5 seconds; one read
of the SELinux file. So at most about 2 seconds per search. The `stat`, `canonicalize`
and SELinux calls are plain blocking calls with no limit: a hung mount in `PATH` or
`$HOME` could hold a search for as long as it hangs. The skip rule covers that: no
search stacks, the timer just does nothing until it returns. Check again still starts
its own search, which would meet the same mount.

### A denied socket with the other engine off (2026-11-04, spec Decision 9)

Owner decision, which replaces the last paragraph of "Two commands when both are
installed": a denied Docker socket with Podman installed and off now also offers
Podman's command, and a denied Podman socket with Docker installed and off also
offers Docker's. One new variant carries both, `Diagnosis::NoPermissionAndOtherOff
{ socket, denied_engine }` and `NoEngineCase::NoPermissionAndOtherOff(EngineKind)`,
so the plain denied cases stay as they were. "Installed" is the same as for
`DockerAndPodmanOff`, so a `docker` command that is really Podman never names Docker
on the Podman card. When both engines refuse, the first denied socket decides alone,
as before. The denied engine's block goes first, as the title reads. The labels,
notes and pages come from the existing cards; the title, text, left note and status
text are new. The 5-second search needs nothing new: it looks at
`shows_no_engine_card`, and a reducer test runs it over both cards.

Size, measured once with a throwaway `iced_test` over `view/no_engine.rs` with the
kit fonts and four searched sockets (so two lines of "looked for"), in px. The main
area of an 800x600 window has about 539, worked out from the centered card and the
status bar:

| Locale | Both off | Denied Docker, Podman off | Denied Podman, Docker off |
| --- | --- | --- | --- |
| en | 483 | 499 | 499 |
| de | 543 | 539 | 539 |
| fr, es, pt-BR, ru | 523 | 539 | 539 |
| ja | 483 | 519 | 499 |
| zh-CN | 455 | 455 | 471 |
| zh-TW | 455 | 455 | 471 |
| ko | 635 | 671 | 635 |

So the new card fills the area with no room to spare in the long Latin locales, and
is up to 16 px taller than the both-off card there, because the docker group warning
takes two lines. It is shorter than both-off in German, the longest before. Korean
does not fit either card; the simulator may use a wider fallback font for it, so
check it by eye (checklist step 15i).
