# 35 Statistics of a session and of a project

Status: ready-for-agent
Blocked by: 34

## Why

The owner wants to see what one session and one project cost: tokens,
limits and the rest the agent's data gives (spec Decision 14,
2026-11-17). Today the dashboard shows only totals per profile.

What the session files hold (read on 2026-11-17):

| | Claude Code | Codex |
|---|---|---|
| Start, end, length | yes | yes |
| Messages and answers | yes | yes |
| Tokens in / cached / out, by model | yes | yes |
| Money | estimate at list prices | estimate at list prices |
| Limits | no | the `rate_limits` of its `token_count` events |

## What to do

1. **In `otty-agents`:** a new source of session statistics, read from the
   agent's own files, read-only, and never through `codex app-server`.
   - An agent gives it or not, like `spend_counter` and `bill_source`
     (`Option` on `Agent`). Both Claude Code and Codex give it. Justify the
     trait in its doc as the others are.
   - **For one session:** start, last activity, length, the number of user
     messages and of answers, tokens in / cached / out by model and in all.
   - **Codex only:** the windows of the first and the last `rate_limits` of
     the session, as "used % at the start → at the end" per window, with
     the window names OTTY already writes ("5 hours", "7 days").
   - **For one project:** every session of the project folder, of every
     profile and every agent, over the whole history (not 30 days). It
     gives the number of sessions, the first and the last, tokens by model
     and by profile, and in all.
   - Reuse the readers of the spend counter where they already parse these
     files. Do not parse the same lines twice in two different ways.
2. **Money:** shown only for profiles that pay with an API key, as on the
   dashboard: the estimate at the price table's prices, by the existing
   money estimate code. A project sums the money of its API key profiles
   only. For a subscription no money line shows.
3. **The io crate** reads in the background, never on the UI thread, and
   shows the kit's spinner while reading. A failure shows its reason in the
   popup, not as a notification.
4. **The session popup:**
   - New item "Show statistics" in the session menu, last, after a
     separator.
   - It is off (faded, no action) when the session's agent gives no
     statistics source.
   - It opens the kit `Dialog` titled with the session title. The dialog
     has a small `KeyValueList` (agent and profile; started; last activity;
     length; messages; answers) and a tokens table by model (Model, In,
     Cached, Out, Total; plus Cost for an API key profile).
   - For Codex, a Limits block lists each window "12 % → 31 %".
   - The dialog closes with Close or Escape.
5. **The project popup:**
   - A new icon button "Project statistics" in the project title row,
     before Open folder in a terminal tab, with a chart-like icon from the
     existing set. Ask in the report if there is none; draw no new icon.
   - It opens a `Dialog` titled with the folder name.
   - The dialog has a `KeyValueList` (sessions; first; last), a tokens
     table by model, and a table by profile (Profile, Sessions, Tokens; plus
     Cost when any of them is an API key).
   - It stays on when the folder is gone, since the history is still
     there.
6. All ten locales; tests of the binary for pressing both buttons through
   the whole window (the guard allow-list, see issue 26).

## Done when

- Tests, written first and seen failing, cover:
  - Reading: a Claude session file and a Codex session file (made-up
    test data, no account data), the Codex limits at start and end, a
    session with no token lines, a broken line skipped.
  - The project sum over two profiles and two agents, money only from API
    key profiles.
  - Popups: the menu item off for an agent with no source; the popups'
    contents; the spinner; a failure in the popup.
  - The binary: pressing both buttons through the window.
- Run by hand:
  - right click a Claude session, Show statistics;
  - the same for a Codex session (with Limits);
  - the project statistics button.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments
