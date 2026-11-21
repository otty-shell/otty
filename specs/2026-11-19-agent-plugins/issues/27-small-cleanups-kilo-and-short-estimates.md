# 27 Small cleanups: Kilo leaves the OpenCode plugin; tight places show only the number

Status: done
Blocked by: 21

## What to build

1. Owner, 2026-11-21: remove what is left of Kilo from the OpenCode
   plugin (`plugins/opencode/src/`, the `KILO` id and its provider ids,
   tests and README). Kilo logins are not shown at all.
2. Owner, 2026-11-21: where the room is tight, a money estimate shows
   only the number ("$42.10"), without "· estimate, list prices". The
   full words stay where there is room and in the tooltip, if one is
   there today. Docs follow.

## Done when

- Tests first for the short estimate.
- No `kilo` left in `plugins/opencode` outside git history.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-21: done. The OpenCode plugin has no Kilo left: no `KILO` id,
  no `kilo`/`kilocode` provider ids, no gateway-token login; Kilo logins
  are left out like any provider OTTY does not read. Its embedded copy is
  rebuilt. Where the room is tight (the left panel, the chips of
  accounts, the Now column of All accounts and of Found by OTTY) an
  estimate shows only the sum, "$42.10", with the tooltip of the mark it
  had. The mark stays under the sums of the account page and of the usage
  block, and after the sum in the Cost fact of a project, which has room.
  The Overview page of the public docs says so.
