# 27 Small cleanups: Kilo leaves the OpenCode plugin; tight places show only the number

Status: ready-for-agent
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
