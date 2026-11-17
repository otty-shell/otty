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

### 2026-11-17

Built in `otty-agents`, `otty-app-agents`, `otty-app-agents-io`,
`otty-app-i18n` (all ten locales) and the binary, tests first.

1. **The source.** `StatsSource` (in `stats.rs`) with `session_stats` and
   `project_stats`; `Agent::stats_source()` gives it or `None`, like
   `spend_counter` and `bill_source`. Its doc justifies it: two real
   agents, each with its own files, and reading crosses into files. Both
   agents give one; the mode turns Show statistics off for an agent whose
   `stats_source()` is `None` (the core tests use an agent OTTY does not
   know for that: the feature has no mock of `Agent`, and adding `mockall`
   to it would be a new dependency of the crate).
2. **Session.** `SessionStats`: start and last activity (the first and the
   last prompt or answer), length, the user's messages, the answers (the
   requests of every model), tokens by model and in all, and for Codex
   `LimitChange`s, each window of the first and the last `rate_limits` of
   the session paired by name ("5 hours", "7 days"). Claude Code: the file
   `projects/*/<id>.jsonl`, plus the files of its helper agents in
   `projects/*/<id>/`, whose tokens count too. Codex: the copy of the
   session that changed last (plain, compressed or archived).
3. **Project.** `ProjectStats` of one profile: the first and the last
   answer and the tokens by model over the whole history (no 30-day cut);
   `ProjectStats::merged` sums the parts of several profiles. Claude Code
   reads only the folders of `projects/` named after the project folder
   (every character that is not a letter or a digit becomes `-`, or the
   name the session list decodes), and keeps a file whose first `cwd` is
   the folder. Codex finds the files of the folder by their first line,
   taken from what the session list already read and kept
   (`SessionCache::kept_meta`, read only, no file opened again); a file the
   list did not keep has its first line read. Helper sessions of Codex are
   not listed, so they are not counted. The number of sessions is taken
   from the project's session list (the same sessions the table shows).
   Speed: the same line readers as the dashboard, one pass per file, only
   the files of the folder; nothing is kept between two opens of the
   dialog.
4. **No line parsed two ways.** The answers come from the spend readers
   (`answer_on_line` of Claude's `spend_file.rs`, Codex's `Line` and
   `SessionTokens`), the Claude prompts from the session list's
   `Record::prompt` (`prompt_time_on_line`), the Codex limits from
   `session_limits.rs` (`rate_limits_in`, `RateLimitsRecord::limits`).
   Each answer counts once by its identity (`whole_history` in
   `spend_tally.rs`, next to `tally`); `merge_models` came out of
   `spend.rs`.
5. **Money.** `MoneyTotal::of_models` prices tokens with the code of the
   money estimate. A session of an API key profile has a Cost column ("no
   price" for a model the table lacks) and an "All models" row; a project
   has Cost in the profiles table and a Cost fact summing the API key
   profiles only; a subscription shows no money.
6. **Feature.** "Show statistics" is the last item of the session menu,
   after a separator, faded when the agent reads none. A "Project
   statistics" icon button sits before Open folder in a terminal tab and
   stays on for a gone folder. Both open one `Dialog` (form width, Close
   and the cross; Esc through the frame's new `AgentsStatsDialog` overlay):
   the kit spinner with "Reading the session files…" while
   `otty-app-agents-io` reads on a thread of its own, the reason in the
   dialog on a failure (no notification), else a `KeyValueList` and
   `DataTable`s as tall as their rows. An answer for a dialog that was
   closed or opened on something else is dropped.

Tests seen failing first: the reading of a Claude and a Codex session and
project (made-up files: a broken line skipped, a session with no token
lines, the limits at start and end, one record of limits, two copies of a
session, a missing session, a missing `projects` folder), the whole
history tally, `ProjectStats::merged`, `MoneyTotal::of_models`, the
reducer (open, answer, failure, a stale answer dropped, close, a gone
folder), the menu and project view models, the dialog view model (facts,
tokens, money only for an API key, limits, the project sum over two
profiles and two agents with money only from the API key one), the views
(spinner, failure, contents, Close, the menu item, the button), the io
reads, and the binary: pressing Show statistics and the button through the
whole window, Esc, and the guards (seen failing with the new overlay
turned off). Not seen failing: the Claude session with no token lines
(its prompts were read before the tally existed) and the dispatch test
(one task each).

Deviation: **no chart-like icon** is in the set (`assets/svg`), so the
button uses `icons::FILE` for now; the owner is asked which icon to use.
The project dialog's dates are of the first and the last answer.

Checks: fmt, clippy, deny and one workspace test run pass. Coverage was
not measured.
