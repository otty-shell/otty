# 24 Loading in the background and notifications for real failures

Status: ready-for-agent
Blocked by: 23

## Why

Spec Decision 10: the owner does not want to wait when opening Agents or
Containers mode, and wants to be told when something is really broken.

## What to do

1. At start, OTTY begins reading what Agents mode shows (profiles, then
   each profile's sessions, limits, tokens and money) and begins looking for
   the container engines of Containers mode, without either mode on screen.
   Look at how both modes start their reads today (`crates/app/otty/src/
   dispatch/agents.rs`, `dispatch/containers.rs`, `dispatch/modes.rs`,
   `startup.rs`, `subscription.rs`) and move the first read to startup.
2. Timers (the five-minute limits read, the engine search of a no-engine
   card, engine event streams) run as today, only while their mode is on
   screen, so a mode in the background costs nothing. Opening a mode reads
   again only what is older than its timer's period. Keep the machine load
   low: the startup reads run one after another per kind, not all at once,
   and never start `claude` (the hidden terminal of issue 10) at startup —
   only when Agents mode is on screen.
3. Real failures show a notification (`NotificationsIntent::Push`): the
   list in spec Decision 10. Expected states (not logged in, not available,
   a plan without numbers) never do. One notification per problem: keep a
   small set of the problems already told and drop a problem from it when
   it goes away, so it is told again only if it comes back.

## Done when

- Tests, written first, cover: the first reads are asked for at start with
  no mode on screen; no timer read while the mode is away; opening the mode
  reads only stale data; each kind of real failure gives one notification,
  a repeat gives none, a recovered and failing again gives one more; an
  expected state gives none; the hidden terminal is not started at startup.
- Run by hand: start OTTY, wait a few seconds in Terminal mode, open
  Agents: the profiles and projects are already there; same for Containers.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-17

Built in `otty-agents`, `otty-app-agents`, `otty-app-agents-io`,
`otty-app-containers`, `otty-app-i18n` and the binary.

At start (`startup.rs`), after the first tab, the binary sends
`ContainersIntent::LoadInBackground`, then `AgentsIntent::LoadInBackground`,
with Terminal mode on screen. Containers looks for the engines and, once,
loads the lists of the engines it found, so the tree and the tables are
there when the mode opens. Agents reads the profiles, then the sessions of
every profile, then the limits, then the tokens, then the bills: the next
kind is asked for only when no read of the kind before is on its way, and a
kind with nothing to read is passed (`reads.rs`). The two features start at
the same time; within Agents mode the kinds go one after another.

The hidden terminal never starts while Agents mode is away. A limits read
asked while the mode is away is `AgentsOutcome::LoadLimitsInBackground`;
`otty-app-agents-io::load_limits_in_background` gives the sources a
terminal starter that refuses ("OTTY starts it only while Agents mode is on
screen"), so that source is "not available". The `/usage` source no longer
remembers a start that failed (only a started program is remembered for
4 min 30 s), so the read on screen does start it. Limits whose sources all
failed in such a read wait for the terminal: opening the mode reads them
again, and a read of the start that lands after the mode opened is read
again at once.

Fresh means younger than the period of the mode's timer, in one place per
feature: `freshness::READ_PERIOD` (five minutes) in Agents mode,
`ENGINE_SEARCH_PERIOD` (five seconds) in Containers mode. Opening Agents
mode: while the first profiles read is on its way, nothing new (its answer
asks for every kind, now with the terminal); profiles older than five
minutes are read again with everything, as before; else only the limits,
tokens and bills that are stale or never read, and the limits that wait for
the terminal. `LoadedProfiles` got `read_at` for this. Opening Containers
mode: a search on its way is waited for; a search that ended less than five
seconds ago is kept and only the lists are loaded again (no engine events
came while the mode was away); an older one is done again, as before, with
the engines found at start on screen meanwhile. The timers are as before:
only while their mode is on screen.

Real failures (`real_failures.rs`): `agents.json` that can not be read; a
config or session folder that is there but can not be read (the new
`SourceError::Unreadable` of both session sources; a file in place of the
folder stays "not available"); every limit source failing with at least one
"unexpected answer"; `codex app-server` or `claude` in the hidden terminal
that ends before it answers (the new `SourceError::ProgramStopped`, shown
as "failed" in the sources list; "claude is not found" stays "not
available"), also when a later source answered. The core state keeps the
problems told (`ToldProblems`); a problem is told as `AgentsOutcome::Failed`
when it is there and was not told, and forgotten when an answer no longer
has it. New texts in all ten languages: `tpl_agents_folder_unreadable`,
`tpl_agents_limits_changed`, `tpl_agents_program_stopped`.

Choices and deviations: a failed save of `agents.json` still answers every
press (in the dialog, or as a toast after Remove), since it answers the
user's own action; only the read is told once. The config folder is checked
through the sessions read: a folder without the permission to enter it
makes `projects/` or `sessions/` unreadable. `codex app-server` may still
start in a background read when the usage request fails; the issue limits
only `claude`. A limits read started on screen that is still running when
the user leaves may reach the hidden terminal after the mode is gone.

Tests seen failing first: the five `otty-agents` tests (unreadable folders,
stopped programs, a failed start asked again), the background read of the
limits in the io crate, the three i18n tests, the freshness tests of the
limits, tokens and bill, the reads and real failures tests of the feature
(against stubs), four of the five Containers tests, and the startup and
background limits tests of the binary. Seen passing at once: the
`freshness.rs` tests (written with the code), the time of the profiles
read in the io crate, "opening the mode reads the profiles again when they
are old", "an expected state is never told", "no read on the timer while
away", "opening Containers mode after the period searches again" and "the
first reads start with no mode on screen" (the features were done first).
Four Run form tests of Containers that leave and enter the mode again now
age the search first, so they still see the search they test around.

Checks: fmt, clippy, deny and one workspace test run (5548 passed) pass.
Coverage was not measured, at the owner's request.
