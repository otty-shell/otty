# 07 The project page and the statistics popups

Status: ready-for-agent
Blocked by: 06

## Why

Spec Decision 6: projects show profiles and accounts; statistics stay
popups.

## What to build

1. Project page: Project info (Path, Profiles chips, Accounts chips,
   Sessions, First and Last activity), then the sessions table with a
   Profile column (agent icon, link). Keep every interaction of today:
   click, double click and Enter = Continue, right-click and the row "⋯"
   (Continue, Continue in a split, Copy resume command, Copy session ID,
   Show statistics), search, sort.
2. Project statistics popup (the button stays): facts plus the usage block
   with By profile | By model | By account. Money only from known prices;
   a line names the subscription profiles left out.
3. Session statistics popup: facts plus an Account line (written in the
   session; the profile's account; "one of 2 OpenAI accounts") and the
   usage block with By model | By account. No "By session" tab.
4. The Projects section of the left panel is unchanged.

## Done when

- Tests first: the Account line cases; the project sums by profile and by
  account; every session interaction still reaches its command (existing
  tests stay green).
- The checks of `AGENTS.md` pass.

## Files

Feature core `view/project.rs`, `stats_dialog.rs`, `sessions_table.rs`,
`menus.rs`; i18n.

## Owner checks by hand

Open a project, continue a session by double click, open both popups.

## Comments
- 2026-11-19, issue 06: the usage block is `model/usage_block.rs` +
  `model/usage_groups.rs` + `view/usage_block.rs`. It reads one profile's
  `Spend` (per day `DaySpend::parts`, today `Spend::today_by_hour`), so
  the project and session popups need their history in the same shape
  (by day and project/model, today by hour) and a `UsageTab::ByProfile`
  (the tabs are fixed to `UsageTab::ALL` today). The choices live in
  `AgentsState::usage()`.

Built (2026-11-19):

- `otty-agents`: `SessionStats::spend()`, the session's own tokens by day
  over 30 days and today by the hour, counted by the same `tally` as a
  profile's tokens (Claude Code with its helper agents, Codex from its
  file). `StatsSource::session_stats` and `read_session_stats` take the
  user's `LocalNow`; io passes it as it does for the tokens.
- Usage block for several profiles: `UsageSource` (name, `Spend`,
  accounts, `is_priced`), an optional project filter, the tabs of each
  block (`UsageTab::PROFILE_PAGE`, `PROJECT_POPUP` = By profile | By model
  | By account, `SESSION_POPUP` = By model | By account; no By session),
  money only for profiles that pay per token. Profiles counted over fewer
  days line up at today.
- Project popup: facts (Sessions, First and Last activity, Profiles,
  Tokens, Cost of the API key profiles as before) and the usage block of
  every profile with a session there, only what was spent in the project,
  read from the profiles' counts (no new read). With Money, a line "Left
  out of the money, paid by a subscription: Codex"; Money off with its
  reason when no profile pays per token or the prices are not read.
- Session popup: facts with Tokens and the Account line
  (`model/session_account.rs`: written in the session, the profile's
  account, "one of 2 OpenAI accounts", "—"), the Codex windows, and the
  usage block of the session. It opens on the shortest period that holds
  the session's last tokens.
- The popup has its own usage choices (`StatsDialog::usage`); the usage
  intents go to it while it is open, and the binary lets them through the
  dialog guard. Its drop-downs open their lists under the fields (kit
  `NoteSelect`), since no menu of the mode shows over a dialog; Money off
  is a muted "Money: reason" line under the title.
- Project page: Project info (Path, Profiles chips, Accounts chips with
  their number, Sessions, First and Last activity) above the toolbar and
  the table. Columns Title | Profile | Agent | Updated; the Profile cell is
  the agent icon and a link (new kit cell `TableCell::IconLink`) that sends
  `OpenSessionProfile(row)`. Every interaction of the table is kept.
- The old models and profiles tables of the dialog, `ProfileSummary` and 9
  i18n keys are removed; 6 new keys in the 10 languages. The account chips
  are drawn once in `view/chips.rs` (`account_chips`).

Decided differently:

- First activity on the project page is the last change of the oldest
  session: the session list has no start times, and reading the whole
  history on every project press is too slow. The popup keeps the exact
  first and last answer from its read.
- The Money reason in a popup is a line under the title, not a disabled
  item: the kit `NoteSelect` list is as narrow as its field and has no
  disabled options. Picking Money there changes nothing.
- The facts of both popups gained Tokens (the whole history), since the
  usage block shows only the last 30 days.
