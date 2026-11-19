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

## Comments

Built (2026-11-19):

- `otty-agents`, `record_format.rs`: each spend reader counts, per file,
  the records of the kind it reads that it saw and understood, the
  agent's version written in the file, and what it could not read: the
  needed field that was missing or of another shape (`message.usage`,
  `timestamp`, `payload.info`, "JSON record") and the names of the fields
  found there (short plain keys only, at most 20). Claude Code: records
  of type `assistant` (also found by that marker now, so a renamed
  `usage` is still seen); understood with a time and at least one token
  count in `message.usage`. Codex: `token_count` events; understood with
  a time and a usage with a known count, or `info: null` (limits only).
  The version: Claude Code `version`, Codex `session_meta.cli_version`.
- Tested versions, next to each reader (`spend_file.rs`): Claude Code
  `TESTED_VERSION = "2.1.328"`, Codex `"0.169.0"` (the versions on the
  owner's machine today).
- The rule (`format_change`): the newest files (by change time) are taken
  until they hold 5 such records; when fewer than half of those were
  understood, the count fails with `SourceError::FormatChanged` (boxed
  `FormatChange`) instead of giving fewer tokens. A few odd lines, an
  old odd file, or fewer than 5 records never do.
- Feature: the failure keeps the last good count greyed with its time
  (the usage block's "Last good numbers"); the profile number is "format
  changed" (left panel, All profiles); the Tokens source row is "format
  changed" (warn) with the time of the last good count; the profile need
  "Claude Code 2.2.0 changed its session files" with "This OTTY reads
  them up to Claude Code 2.1.328. Tokens since <time> are missing; …" and
  the button Copy report (`NeedAction::CopyReport`, puts the text on the
  clipboard). It shows on the profile page, All profiles and Found by
  OTTY. Limits, bills and other profiles are untouched.
- Copy report (English plain text, `FormatChange::report`): agent,
  version, tested up to, folder (`~/…`), file kind, records seen and
  understood, missing fields, fields found there. No value of a record.
- i18n: 7 new keys in the 10 languages.

Decided differently:

- The report is in English in every language: it is for an OTTY issue.
- "format changed" uses the error color and a warn badge, like the other
  profile needs of the app ("needs login"), not the prototype's amber.
- The need says "reads them up to <exact tested version>", not "2.1.x".
- Only the token counts are judged. The statistics popups (session and
  project) read the same files and still skip what they do not
  understand; a renamed record type (`assistant`, `token_count`) is not
  seen at all, so it is not caught.
