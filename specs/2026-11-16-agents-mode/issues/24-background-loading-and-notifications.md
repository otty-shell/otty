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
