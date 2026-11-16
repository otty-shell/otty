# 23 The Usage block and the sources: exact times, names, state labels

Status: ready-for-agent
Blocked by: 22

## Why

Spec Decision 10: "Limits" becomes "Usage" with only the update time under
it, reset times are exact, sources have names a person reads, and a
failure's reason is a tooltip instead of a line of text.

## What to do

1. The billing block is titled "Usage" (all three billings); under the
   title only "Updated 2 min ago" (the greyed last answer keeps "Updated …"
   plus its own note that the refresh failed). The source is no longer
   named there.
2. Reset and renewal times: "Resets in 2 h 10 min, at 15:30" within a day,
   "Resets on Thursday at 09:00" within six days, "Resets on Nov 23 at
   09:00" later; in the user's time zone. The offset is read today by the io
   crate for the token count (issue 09); keep one offset in the core state
   and pass it where times are written. A reset given only as text by the
   agent (issue 10) stays as given.
3. Each source has a stable id in `otty-agents` and the app maps it to a
   translated name: Claude Code "Anthropic usage API", "Claude Code /usage";
   Codex "ChatGPT usage API", "Codex app-server", "Latest Codex session";
   money "Anthropic cost report", "Session files at list prices". Use the
   names wherever a source is named.
4. "Where the numbers come from": each row is the name and a kit
   `StatusBadge`-like label (answered, not needed, not available, login
   expired, failed). The reason of a failure or of "not available" is the
   label's tooltip; nothing is written under the row. Use the kit's tooltip
   style; add a small option to the label in the kit if it needs one.

## Done when

- Tests, written first, cover: the title and the update line, the three
  ranges of reset times at a fixed time and offset, a reset given as text,
  every source name, the tooltip text of each failing state.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16 (coordinator)

From issue 22: drop the "Program" row of the dashboard's Profile list (the
owner found the program path useless; the row now only repeats `claude` or
`codex`). And fix the Chinese names of "AI profile(s)": "AI 个人资料" reads as
"personal data"; use "AI 配置" (zh-CN) and "AI 設定檔" (zh-TW), and check
the other languages read as "AI profile" in the sense of an account setup.

### 2026-11-16

Built in `otty-agents`, the kit, `otty-app-agents`, `otty-app-agents-io`
and `otty-app-i18n`; the binary only got the new field in its test data.

Sources: `otty-agents` has a `SourceId` enum (`source_id.rs`): session
files, Anthropic usage API, Claude Code `/usage`, ChatGPT usage API, Codex
app-server, Latest Codex session, Anthropic cost report, and session files
at list prices. `LimitSource::name()` and `SessionSource::name()` became
`id()`, and `Answer::source()` / `SourceFailure::source()` give the id.
`SpendCounter::name()` is gone: nothing read it. The app turns an id into
its translated name (`model/sources.rs`, `source_name`), everywhere a
source is named, also in the failure notes ("Session files: not
available: …"). The money rows use the cost report and estimate ids
directly, since only Claude Code has a bill.

Usage block: the billing block is "Usage" for all three billings, with
only "Updated 2 min ago" under the title. A greyed answer keeps that line
and its failure note. For an API key the note that says what the money is
(the bill of the organization, or the estimate at the prices of a date)
moved to the lines under the block, because Decision 9 still wants "an
estimate" said. The money of the bill has its own read time now
(`LoadedBill::read_at`).

Reset and renewal times (`model/reset_time.rs`): "Resets in 2 h 10 min,
at 15:30" within 24 hours, "Resets on Thursday at 09:00" when the day is
at most six local days ahead, else "Resets on Nov 23 at 09:00"; under a
minute or past is "Resets now". A reset given only as text keeps its
text. `otty-agents` gained `LocalNow::date_of`, `minutes_of_day`,
`days_until` and `CalendarDate::weekday` for this. Day and month names,
the duration pieces and "{time} UTC" are new texts in all ten languages
(`i18n::agents_weekday`, `agents_month`).

Time zone: the io crate already read `date +%z` for each token count. It
now also reads it with the profiles: `LoadedProfiles::utc_offset_seconds`.
The profiles read always ends before any limits are read, so no reset
time is drawn in UTC by mistake. The state keeps the last known offset;
a read that could not get one keeps the earlier one. When none is known,
times are in UTC and say "UTC". The token count still reads its own
offset for its days. So the offset is not delivered "once": it comes with
each profiles read, which is cheap and follows a summer-time change.

Sources list: each row is the name and a `StatusBadge`; the reason is the
badge's tooltip. That is the kit's tooltip style, through a new
`StatusBadge::tooltip` option, with a gallery entry. Not available
carries its reason, failed carries the error, and login expired carries
"Open the agent to log in again, then refresh." (a choice: a tooltip that
says what to do). Nothing is written under a row anymore.

From the comment: the "Program" row of the Profile list is gone, with its
text. In zh-CN every "个人资料" of Agents mode became "配置" ("AI 配置",
the column, the Profile title, the dialog titles and the messages).
zh-TW keeps "AI 設定檔". The other languages already read as a profile of
a setup (Perfiles de IA, Profils IA, AI プロファイル, AI 프로필, Perfis de
IA, AI-профили, KI-Profile). The old texts "Limits", "Spending", "From
{source}, {time}", the three relative "resets in" texts and the old bill
and estimate source names were removed from all catalogs.

Tests seen failing first: the agents' source ids and the calendar helpers
(compile errors), the i18n tests (missing keys), the kit badge tooltip
(missing method), the five reset-time tests (against a stub that returned
nothing), the io tests (missing fields). The model and view tests of the
feature were rewritten before the code, but the crate did not build until
the whole migration was done, so they were run against the stub only.
The dashboard test of the time zone first passed against that stub (both
sides were empty). The reducer test of the offset, after the offset moved
from the token count to the profiles read, was not run before the change.

Checks: fmt, clippy, deny and one workspace test run (5513 passed) pass.
Coverage was not measured, at the owner's request. It will be measured
once at the end of the whole work. A run before the last change showed
96.00 % lines.
