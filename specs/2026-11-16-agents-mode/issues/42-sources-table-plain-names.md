# 42 "Where the numbers come from" as a table with plain names

Status: ready-for-agent
Blocked by: 41

## Why

The owner (2026-11-18): the block "Where the numbers come from" should be
a table too, and the sources need clear, plain names; today it is not
always clear where a number comes from (spec Decision 18).

Today (`view/profile_column.rs`, `model/sources.rs`) it is one numbered
list mixing the sources of the limits, the money and the prices, with
names such as "Codex app-server", "Session files at list prices" or
"Kept LiteLLM prices"; it never says which number a row gives, and the
time of the prices sits inside the name.

## What to do

1. **A kit `DataTable`** in place of the list, as tall as its rows (no
   scroll, `data_table::height_for_rows`), with the columns:
   - **Number**: what the row gives — Limits, Tokens, Money, Prices.
     Rows of one number stay together, in the order the walk tries them;
     the number is written on the first row of its group only.
   - **Source**: the plain name (table below).
   - **State**: the badge it has today (answered, not needed, login
     expired, not available, failed), with today's tooltip saying why.
   - **Updated**: when the data of that source is from, when it is known
     (the price file's time, the time of the latest read that answered);
     else empty. The time leaves the name.
2. **A Tokens row**: the tokens and the session counts come from the
   session files; show that source too, so every number on the dashboard
   has its row.
3. **Plain names**, in all ten locales. A name says where the number is
   taken from and, where two sources give the same number, how; the
   tooltip of the name says more. The owner approved these names on
   2026-11-18 (the other names stay as they are):

   | Source id | Name now | New name |
   |---|---|---|
   | `AnthropicUsageApi` | Anthropic usage API | Anthropic usage API (direct request) |
   | `ClaudeUsageCommand` | Claude Code /usage | Claude Code /usage (runs claude) |
   | `SessionFilesAtListPrices` | Session files at list prices | Estimate: tokens × LiteLLM prices |
   | `KeptPrices` | Kept LiteLLM prices | LiteLLM prices cache |

   Tooltips of the names, for every source (plain words; check each
   against what the code really does):

   | Source id | Tooltip of the name |
   |---|---|
   | `AnthropicUsageApi` | Asks anthropic.com directly for the limits of the account, with the login Claude Code keeps in the profile folder |
   | `ClaudeUsageCommand` | Runs `claude` in a hidden terminal and reads its /usage screen; used when the direct request gives no answer |
   | `ChatGptUsageApi` | Asks chatgpt.com directly for the limits of the account, with the login Codex keeps in the profile folder |
   | `CodexAppServer` | Asks the `codex` program for the limits |
   | `LatestCodexSession` | The limits Codex wrote in its newest session file |
   | `SessionFiles` | The agent's session files on this computer |
   | `AnthropicCostReport` | The cost report of the organization, read with the admin key |
   | `SessionFilesAtListPrices` | The tokens of the session files at the LiteLLM prices; an estimate |
   | `LiteLlmPrices` | The price table of the LiteLLM project on GitHub, read at most once a day |
   | `KeptPrices` | The copy of the LiteLLM prices saved on this computer at the last good read |

4. **The same names everywhere** a source is named (notifications of real
   failures, issue 24; tooltips; the statistics dialog if it names one).
5. Keep the source walk and the states as they are; only how they are
   shown changes.
6. **A public documentation page** (owner, 2026-11-18): "where we take
   the numbers from and how". Add it to `docs/public/` in the style of the
   pages there (front matter with `title`, `description`, `sidebar.order`;
   a folder per section, such as `docs/public/Agents/`), and link it from
   `docs/public/index.md`. For a user, in plain words:
   - what each number of Agents mode is (limits, tokens, money, prices)
     and which periods it covers (the rolling 30 days, the calendar month);
   - for each agent and kind of profile (subscription, API key with and
     without an admin key), the sources in the order OTTY tries them, with
     the names the table shows, what each one does, what it sends and to
     whom (nothing about the user beyond the agent's own login; the prices
     are a plain GET), and when it is used;
   - what "answered", "not needed", "login expired", "not available" and
     "failed" mean, and what Refresh does;
   - that money from tokens is an estimate at list prices, where the
     prices come from, how often they are read, where they are kept
     (`~/.cache/otty/prices/`), and what shows when there are none;
   - that OTTY only reads the agents' files and never changes them.
   Check every statement against the code; say nothing the code does not
   do. The page names the sources exactly as the app shows them in
   English.

## Done when

- Tests, written first and seen failing, cover: the columns; the groups
  and their order for a Claude Code subscription, a Codex subscription,
  an API key profile with and without an admin key, and with no prices;
  the Number written once per group; the Tokens row; Updated from the
  price file and from the latest read; the names and their tooltips; the
  table as tall as its rows.
- The documentation page exists, is linked from the index, and matches
  the code and the names on screen.
- Run by hand: each kind of profile shows a table whose every row says
  which number it gives and where it comes from.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-18 — built

**What was built**

- **Kit: a cell with a tooltip.** `TableCell::with_tooltip(tip)` wraps any
  cell of a `DataTable` (the new `TableCell::Tipped`). A badge shows the
  tooltip as `StatusBadge::tooltip` does; any other cell shows it over its
  content, in the same bubble, as wide as a badge's at most. The gallery's
  plain table has one.
- **The table** (`view/sources.rs`, new): the title, then a kit
  `DataTable` with Number, Source, State and Updated, given the height of
  `data_table::height_for_rows(rows)`, so it never scrolls. The source name
  carries its tooltip (what the source does); the state badge keeps today's
  tooltip (why, what to do, or what the answer is). The list in
  `view/profile_column.rs` is gone.
- **The rows** (`model/sources.rs`): `SourceRowViewModel` now has `number`
  (on the first row of a group only), `name` (the plain name, no time),
  `name_tip` and `updated`. The groups and their order:
  - a subscription: Limits (the agent's limit sources in walk order), then
    Tokens;
  - an API key: Money (the cost report when the agent has one, then the
    estimate), Prices (LiteLLM prices, then the cache; only listed with the
    estimate, as before), then Tokens. With the bill answered there is no
    Prices group, as before.
- **The Tokens row** (`token_sources`): Session files, answered with the
  time of the count, or failed with why; left out before the first count
  and for an agent that can not count.
- **Updated** is set on the row that answered: the time of the limits read,
  or of the session record the Codex limits were taken from; the time of
  the token count (also on the estimate, which is as old as its tokens);
  the time of the bill read; the time of the price file. Rows that failed
  or were not needed have none. Written as "Nov 16, 2026, 10:00" in the
  user's time zone, like the old name of the prices.
- **Names and tooltips** in all ten locales: the four new names as
  approved, ten name tooltips, the four column titles and the four number
  names. `TplAgentsSourceUpdated` ("{name}, updated {time}") is removed:
  the time left the name. The notifications and the note of a failed
  sessions read name a source through `source_name`, so they use the new
  names too; the statistics dialog names no source.
- **Documentation**, split by agent as the coordinator's comment below
  asks: `docs/public/Agents/Where the Numbers Come From.md` (the common
  page, linked from `docs/public/index.md`: the numbers, the periods, how a
  source is picked and Updated, the states, when OTTY reads and Refresh,
  the estimate with the LiteLLM prices and their cache, what OTTY sends in
  general, read-only; it links each agent page and names no agent's
  source), `Claude Code.md` and `Codex.md`. Both agent pages have the same
  sections in the same order: Kinds of profiles, Limits, Tokens, Money,
  What it changes in the agent's files; each source in walk order with
  "What it does", "What it sends and to whom" and "When it is used".

**Choices and deviations**

- **Two tooltips differ from the approved text.** "The limits Codex wrote in
  its newest session file" is "The latest limits Codex wrote in its session
  files": the code looks through the 10 newest files for the last record,
  not only the newest one. The backticks around `claude` and `codex` are
  left out on screen, where they would show as backticks.
- **Order of the groups of an API key**: Money, Prices, Tokens; the prices
  stay next to the estimate that uses them. Say if Tokens should come
  before Prices.
- The column title is a new key `AgentsColumnSourceUpdated`, not the
  sessions table's `AgentsColumnUpdated`: several languages write that one
  for a changed session ("Geändert", "Изменён").
- The answered price row keeps its old state tooltip ("Prices from the
  LiteLLM price table…") although the name tooltip now says nearly the
  same, since the issue keeps today's state tooltips.
- The least column widths (64 + 170 + 120 + 150) fit the dashboard of the
  smallest main area, so the table does not scroll sideways there; a longer
  name ends with "…". The narrow dashboard test now finds the Updated time
  at the right end in place of the state badge.
- `TableCell` got a variant, so an exhaustive `match` on it outside the kit
  needed one more arm (the test helper `shown_text` of the containers table
  search).
- **The docs say what OTTY changes.** "OTTY only reads the agents' files
  and never changes them" is not quite true for Claude Code, so the common
  page says an agent's page lists what running its program changes, and
  the Claude Code page lists the exceptions of the hidden `claude` run:
  Claude Code saves the trust of OTTY's own probe folder in `.claude.json`
  (spec Decision 12), OTTY removes the session files of that run, and
  Claude Code may renew its token while it runs. The Codex page says OTTY
  changes nothing in its folder; it does not claim what `codex app-server`
  itself writes, which OTTY does not know.

**Tests seen failing first**

- Kit: `given_cell_with_tooltip_when_hovered_then_its_tooltip_shows` and
  `given_badge_cell_with_tooltip_when_hovered_then_its_tooltip_shows` did
  not compile, then failed on their asserts against a `with_tooltip` that
  dropped the tip.
- i18n: `every_source_has_a_name_a_person_reads` (new names) and
  `every_source_has_a_tooltip_and_every_number_a_name` did not compile (no
  keys).
- Feature, against stubs (no tooltip, no groups, no times, no tokens row):
  the 15 tests of `model/sources.rs` that check names, tooltips, groups,
  Updated and the tokens row, and four dashboard tests, among them
  `the_table_groups_the_sources_of_each_profile_by_their_number` (Claude
  Code and Codex subscriptions, an API key with and without an admin key,
  no prices). The four tests of `view/sources.rs` (the columns, the rows,
  the height, the name tooltip) failed against a view that drew only the
  title.

**Check by hand**

1. A Claude Code subscription: the table shows Limits on the first row
   ("Anthropic usage API (direct request)", then "Claude Code /usage (runs
   claude)") and Tokens on "Session files"; the row that answered and the
   tokens have a time in Updated.
2. A Codex subscription: Limits with ChatGPT usage API, Codex app-server,
   Latest Codex session; Tokens. When the session record answers, Updated
   is the time of the record.
3. A Claude Code API key profile with no admin key: Money (Anthropic cost
   report "not available", "Estimate: tokens × LiteLLM prices"), Prices
   ("LiteLLM prices" with the file's time, "LiteLLM prices cache" not
   needed), Tokens. With an admin key whose bill answers: Money only (cost
   report answered with its time, estimate not needed) and Tokens.
4. Network off and `~/.cache/otty/prices` removed: the estimate is "not
   available", both price rows say why.
5. Point at each name: the tooltip says what the source does; at a failed
   state: why. The table never scrolls, also at the smallest window.
6. A failure notification (for example a broken price file) names the
   source with its new name.
7. Read the three pages in `docs/public/Agents/` against the screen: the
   names match the English ones of the table, and the links between the
   common page and the agent pages work on the built site.

Checks: `cargo +nightly fmt`, clippy with `-D warnings`, `cargo deny check`
and one workspace test run pass (no failures). Coverage was not measured.

### 2026-11-18 — coordinator: the docs are split by agent

The owner: the documentation is split into one common page and one page
per agent (Claude Code, Codex), since the two already work differently
and must read separately, and more providers will be added. So
`docs/public/Agents/` holds:

- a common page (`sidebar.order: 1`): what the numbers are, the periods,
  the states, Refresh, the money estimate and the LiteLLM prices and
  their cache, that OTTY only reads the agents' files; it links each
  agent's page;
- one page per agent (`Claude Code.md`, `Codex.md`): its kinds of
  profiles, its sources in the order OTTY tries them, what each sends and
  to whom, and when it is used. Each agent page has the same sections in
  the same order, so a new provider is one more page of the same shape.
  The common page says nothing that only one agent does.

### 2026-11-18 — coordinator: the owner's answers

1. Tokens stay after Prices for an API key profile.
2. The answered price row loses its state tooltip, which repeated the
   tooltip of its name; the text `agents_prices_from` is removed from the
   ten locales.
3. The Codex tooltip "The latest limits Codex wrote in its session
   files" is approved.
