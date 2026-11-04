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

### The copy icon and sideways scrolling (2026-11-04, spec Decision 9)

Owner request: the command boxes did not fit small windows. `CommandBox` now takes
a copy icon (components never bundle icons, so the caller passes `icons::COPY`) and
shows a small `IconButton` with the existing `docker_copy` text as its tooltip, as
Copy JSON does (ticket 37). No new text. The command sits in a horizontal
`scrollable` with `scrollbar::horizontal()` and `scrollbar::style`. `CommandBox` is
used only on these cards and in the gallery.

What the old box did: it never wrapped. The text had `Wrapping::None` and no clip,
so in a narrow box the command ran on under the Copy button (seen on a drawing at
278 px). The fault was the overlap, not the height.

- **Height.** The line is `CONTROL_SM` (24 px) high and the box 36 px, for every
  command, locale and width. The old box was 40 px because the text button was
  28 px, so each block is 4 px shorter.
- **The bar.** The kit's bar floats (no `spacing`), so it takes no height and moves
  nothing. It sits in the last 4 px of the 24 px line, under the text. It shows only
  when the command is wider than the room.
- **Wheel.** iced's own rules, so nothing new: a sideways wheel, Shift and the wheel,
  and a touchpad scroll the text. A plain vertical wheel moves nothing; the card has
  no scrolling parent that could want it.
- **Dragging a panel edge (ticket 40).** `ResizableGrid` already hides the pointer
  from the panes during a resize, so nothing changed. The box's padding keeps the
  bar out of both grab zones even when the card fills the main area: 12 px on the
  left, the icon on the right, against a 4 px zone.

Size, measured with a throwaway `iced_test` over `view/no_engine.rs` with the kit
fonts and four searched sockets, which makes three lines of "looked for" at 460 px.
The figures are the card's real height: they add up exactly from the kit's tokens.
The same test on the old code reads 4 px below every figure in the table of the
section above, in all 30 cells, and that difference was not explained. Both columns
here come from this test. Both off / denied Docker, Podman off / denied Podman,
Docker off, in px:

| Locale | Before | After |
| --- | --- | --- |
| en | 479 / 495 / 495 | 471 / 487 / 487 |
| de | 539 / 535 / 535 | 531 / 527 / 527 |
| fr, es, pt-BR, ru | 519 / 535 / 535 | 511 / 527 / 527 |
| ja | 479 / 515 / 495 | 471 / 507 / 487 |
| zh-CN, zh-TW | 451 / 451 / 467 | 443 / 443 / 459 |
| ko | 631 / 667 / 631 | 623 / 659 / 623 |

The main area of an 800x600 window is 537 px (39 to 576, read from the centered card
and the status bar in a root view test), not quite the 539 used above. Every locale
but Korean now fits it. Panels take 16% of the window each at their smallest, so the
card has its full 460 px at 800x600, and all three commands fit (202.5, 217.5 and
322.5 px of 410 px): no bar shows until panels are dragged wide.

Two limits remain, and both are about text that wraps, not the boxes:

- A card narrower than 460 px is taller, because its notes wrap: at 372 px wide 543 to
  579 px, at 278 px wide 607 to 715 px, in English and German. It then overflows the
  537 px and the main area clips the bottom, since it does not scroll.
- Korean does not fit, and the cause is the font. Hangul falls back to Droid Sans
  Fallback on the machine this was measured on, although Noto Sans CJK KR is
  installed. Most syllables come out there as separate letters, 1.9 em wide instead
  of 0.92 em in Noto Sans CJK KR. A drawing shows it plainly. Added up from each text laid
  out with Noto Sans CJK KR, the same cards would be about 471 / 487 / 487 px, so
  they would fit. The simulator and the app use the same text code and load the same
  fonts, so the app may well look the same on this machine. Not checked in the real
  app. Neither is changed here.

The bar's idle color is the kit's `surface-hover`. On the box's `surface-chrome` that
is 1.47:1 in dark and 1.07:1 in light, so in OTTY Light the bar is hard to see until
the pointer is on it. The cut-off text is then the only sign. Not changed, as it is
the kit's style everywhere.

Not changed: the design system's `CommandBox` card and the canvas's `DockerNoEngine`
board still draw a text button, and the board's box still ends in an ellipsis.

### The card never gets cut off (2026-11-04, spec Decision 9)

Owner request: the same rule as forms. This replaces the first of the "two limits"
above; the main area no longer clips the card.

`EmptyCard::into_scrolling_page` is the card in a `scrollable` with `scrollbar::both()`,
over the form's `FormSheet`. A form only needs the room's width, which a `row` hands
down. Centering the card also needs the height, and a `row` or `column` passes on one
direction only, so `FormSheet::whole_room` around the scrollable hands down both. The
form pages keep their `row` and lay out as before. The start card of Terminal mode
still uses the plain `EmptyCard` in a container.

**Minimum: 380 px** (`size::EMPTY_CARD_MIN`), content only: the card has no padding of
its own. Measured with the kit fonts and four searched sockets, as above, over all
eight cases. Lowest to highest height in px:

| Locale | 460 | 420 | 400 | 380 | 360 | 340 | 300 | 278 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| en | 351-487 | 351-507 | 351-547 | 367-563 | 367-563 | 367-563 | 431-611 | 431-631 |
| zh-CN | 331-459 | 351-459 | 351-487 | 367-503 | 367-523 | 367-563 | 367-563 | 367-595 |
| zh-TW | 351-459 | 351-459 | 351-487 | 367-523 | 367-523 | 367-563 | 367-579 | 367-595 |
| ja | 367-507 | 395-539 | 395-599 | 395-615 | 415-631 | 415-631 | 447-675 | 447-675 |
| ko | 415-659 | 443-679 | 443-695 | 463-743 | 511-759 | 539-799 | 551-831 | 615-883 |
| fr | 351-547 | 367-563 | 367-579 | 367-579 | 383-579 | 415-615 | 431-671 | 471-699 |
| de | 351-531 | 367-579 | 387-579 | 387-579 | 387-595 | 427-623 | 447-715 | 467-715 |
| es | 367-547 | 367-547 | 379-563 | 395-579 | 395-579 | 411-619 | 455-671 | 471-715 |
| pt-BR | 367-547 | 379-547 | 379-547 | 395-579 | 395-579 | 411-631 | 427-683 | 431-699 |
| ru | 351-547 | 351-547 | 351-547 | 351-563 | 367-579 | 411-579 | 415-683 | 447-715 |

The tallest card at 380 is the same as at 400 in French and German (579), and 16 to
32 px more in English, Russian, Spanish and Portuguese. Against 460 it grows by 16
(Russian) to 76 px (English). Below that it climbs quickly: 615 to 631 at 340 in the
same locales but Russian and English, and up to 715 at 300. 380 also holds the longest
command whole: it needs 372.5 px (322.5 of text and 50 of box). So no command box
scrolls on today's cards, and that behaviour is only exercised in tests with a longer
command. Korean is not counted, as in Decision 23.

**Wheel over a command box**, read from iced and tested with a longer command: the
plain wheel scrolls the area, since the box scrolls only sideways. A sideways wheel
moves the command, and once at its end the next wheel moves the area. Within one
gesture the wheel stays with the scroller it started on, until the pointer has moved
100 ms after it or 1.5 s have passed. A wheel that started on the area passes over a box
without being caught.

**The search every 5 seconds** builds the same widgets again, so iced keeps the
scroll state; a test builds frames as the app does and shows the offset unchanged
through a tick and an answer. A diagnosis change that shows another card should keep
the same scrollable and so its offset, clamped to the new height; that is read from
iced and not tested. Check again shows "searching", which is another widget, so the
card returns at its top.

Known limits, as the bars float and the card has no padding of its own. A card too
tall for an area narrower than 460 px has the right bar over its last 4 px; the padding
of a command box keeps it off the icon. Scrolled to the far bottom in an area under 380
px, the bottom bar covers the last 4 px of the "looked for" line. Padding would have
made a card that fits scroll, and moved the point at which it starts to shrink.

**Scrollbar colour.** The note above on 1.07:1 no longer holds: see the checklist and
`UiPalette::scrollbar_idle`. OTTY Light's idle bar is now `#a8aeba`; OTTY Dark keeps
`#2e3140`, the value of `surface-hover`. Contrast on the surfaces a bar sits on:

| Theme, bar | canvas (page, tables, forms) | panel (trees) | chrome (command box) | overlay (dialogs, lists) |
| --- | --- | --- | --- | --- |
| Light idle, before `#e0e4eb` | 1.23 | 1.13 | 1.07 | 1.28 |
| Light idle, now `#a8aeba` | 2.15 | 1.97 | 1.87 | 2.23 |
| Light hover and drag `#7a8292`, unchanged | 3.73 | 3.42 | 3.24 | 3.86 |
| Dark idle `#2e3140`, unchanged | 1.37 | 1.44 | 1.47 | 1.18 |
| Dark hover and drag `#6b7280`, unchanged | 3.66 | 3.84 | 3.91 | 3.15 |

Not changed: the design system has no token for this, and its `CommandBox` and
`EmptyCard` cards say nothing of scrolling; the canvas's `DockerNoEngine` board draws a
520 px card with no scrolling.
