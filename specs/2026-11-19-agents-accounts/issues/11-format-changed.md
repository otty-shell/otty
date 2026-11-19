# 11 Agents change their files: "format changed"

Status: ready-for-agent
Blocked by: 06

## Why

Today a record OTTY does not understand is dropped quietly
(`claude_code/spend_file.rs`: `serde_json::from_slice(line).ok()`), so an
agent update shows fewer tokens as if true (spec Decision 9).

## What to build

1. Each reader counts records seen and understood per file kind.
2. The agent's version from its records or files; per agent the newest
   version tested (a constant next to the reader, raised with each
   check).
3. The profile state "format changed" when new records arrive and none or
   far fewer than before are understood; the need names the version; last
   good numbers stay greyed with their time; other sources go on.
4. Copy report: agent, version, file kind, counts, the names of unknown
   fields; no keys, tokens or session text.

## Done when

- Tests first: a file with unknown records gives "format changed"; a few
  odd lines do not; the report holds no value of a record.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/` readers (`jsonl.rs`, spend files), feature core
(state, needs), i18n.

## Owner checks by hand

Point a profile at a folder with a session file whose fields were renamed;
see "format changed" and copy the report.
