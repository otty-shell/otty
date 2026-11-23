# 11 Prices load before the first count; no profile↔account link inside OTTY

Status: ready
Blocked by: 07

## Why

Two leftovers of issue 06 (see its comment):

- At start the first token count runs before the price list is loaded,
  so once prices arrive every shown profile is counted again: the
  session files are read twice at start-up.
- With the connection gone from the agent contract, OTTY still decides
  that a profile "pays with an API key" when the accounts found in it are
  only keys or organizations. Nothing on screen shows it, but it is a
  profile↔account link inside OTTY, against Decision 1 of the spec ("two
  independent slots, no link between them").

## What to build

1. **Prices before the first count.** At start OTTY loads the price list
   (from its cache when there is one) before it starts the first token
   count, so a start reads the session files once. A count started with
   no prices at all (first run, offline, empty cache) still works and is
   redone once prices arrive, as today.
2. **No inner link.** Find every place in `otty-agents` and the agents
   app feature that still reads which accounts were found in a profile
   to decide something about the profile (the "pays with an API key"
   check, and anything else of the kind, e.g. how an organization bill
   is read), and remove it or move the decision to the provider part,
   which owns its accounts and their bills. Internal scheduling that
   reads an account's numbers through a folder it was found in may stay
   (the account knows its own folders), but no profile decision may
   depend on accounts.

## Done when

- Tests first: a start with cached prices counts each shown profile once;
  a start with no prices counts once and once more when prices arrive; no
  profile state or view model depends on the accounts found in it (a test
  that a profile's page model is the same with and without accounts).
- The checks of `AGENTS.md` pass.

## Comments
