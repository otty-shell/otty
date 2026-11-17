# 25 The loading screen and the first profile

Status: ready-for-agent
Blocked by: 24

## Why

Spec Decision 10: Agents mode should not show an empty screen while it
reads, and should open on something useful.

## What to do

1. Until the first reading of the profiles is done, the main area shows the
   kit `Spinner` with "Looking for agents…", built the way Containers mode
   shows its spinner while it looks for engines (`crates/app/features/
   containers/core/src/view/main_area.rs`).
2. When it is done and nothing is selected, the first AI profile of the
   list is selected and its dashboard opens. A later refresh never changes
   the user's selection. With no profile at all, the existing "no agent was
   found" card shows.

## Done when

- Tests, written first, cover: the spinner before the first reading; the
  first profile selected after it; a selection the user made kept across a
  refresh; the card with no profile.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-17 (coordinator)

From issue 24: like the hidden `claude`, `codex app-server` must not be
started by a background read (startup, or a read while Agents mode is away):
give the background limits read a program starter that refuses, the way
`load_limits_in_background` already refuses the terminal, and read those
limits again when the mode opens (as issue 24 does for the terminal).

### 2026-11-17

Built in `otty-app-agents`, `otty-app-agents-io` and the binary's tests.

Loading screen: until the first read of the profiles ends, the main area
shows the kit `Spinner` with "Looking for agents…" under it in muted text,
in the middle of the screen. It is no longer a card with the mode icon.
Like the Containers spinner it sits in a container with an id,
`LOADING_SPINNER_ID`, which the tests look for. The spinner turns by
itself, frame by frame, so it needs no subscription.

First profile: when a read of the profiles ends and nothing is selected,
the first profile of the list is selected (`select_first_profile_if_none`),
so the mode opens on its dashboard. This happens in the background read at
start too, with the mode away. A profile or project the user selected is
never replaced. With no profile, nothing is selected and the "no agent was
found" card shows. If a later Refresh finds a profile, that profile is
selected then. The "what the mode is for" card now shows only when the
selected project or profile is no longer in the list.

`codex app-server` (coordinator's note): `load_limits_in_background` now
gives the sources a program starter that refuses (`NoPrograms`), the way
it already refused the hidden terminal. Both refuse with the same reason,
"OTTY starts it only while Agents mode is on screen". The code names say
"programs" now, not "terminal": `ProgramStarts`, `Which::WaitingForPrograms`,
`waits_for_programs`, `may_start_programs`. Limits read with the mode away
wait for the program, and are read again when the mode opens, in two
cases: every source failed (as in issue 24), or a source failed before
the one that answered. The second case is new. Without it, a Codex profile
whose usage request failed would show the numbers of its latest session
file and never try `app-server` when the mode opens. For Claude Code
nothing changes: its only source after the usage request is `/usage`.

Tests written first and seen failing: the six new tests of
`otty-app-agents` (spinner in the middle, first profile's dashboard, first
read selects the first profile, no profile until one is found, the wait
after a later source's answer, read again on screen), the io test "a read
in the background starts no codex app-server", and the binary's two tests
(the spinner before the first read, the mode opens on the first
profile). Seen passing at once: "a refresh keeps the profile the user
selected" and "a refresh keeps the project the user selected". Nothing
changed the selection before, so they guard against a regression. The
left panel test of the profile rows now expects the first row to be
selected.

Checks: fmt, clippy, deny and one workspace test run (5558 passed) pass.
Coverage was not measured: AGENTS.md asks for it only before a push.

Follow-up (coordinator's answer): after a read of the profiles or of a
profile's sessions, a selected profile or project that is no longer in the
list is forgotten (`forget_selection_that_left`), and the first profile is
selected by the same rule. One exception: a profiles read that another
read must follow keeps the selection, because a profile just saved and
selected may not be in the older read yet. Tests written first: "a
selected profile that left the list gives way to the first one" and "a
selected project that left the list gives way to the first profile" were
seen failing. "A profile selected before it is read stays until the read
after it" passed at once and guards the exception. Checks: fmt, clippy and
one workspace test run (5561 passed) pass.
