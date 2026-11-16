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
