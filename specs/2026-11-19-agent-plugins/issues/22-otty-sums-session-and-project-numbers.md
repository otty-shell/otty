# 22 OTTY sums the numbers of a session and of a project; a plugin gives only rows

Status: done
Blocked by: 21

## Why

Owner, 2026-11-21, accepted the rule: a plugin gives facts, OTTY makes
totals. A fact is one row of what happened, as the agent wrote it: one
answer of the model with its time, model, who served it, project and
tokens; a limit or a balance a provider told; a bill. A total is a sum
of rows: today, this week, by project, by model, money estimates,
accounts joined across agents.

The main count already works this way: a part reads one file into
answers (`read-answers`) and OTTY sums them. Two calls break the rule:
`read-session-stats` and `read-project-stats`. For the panel of one
session or one project the part sums the tokens by model itself. So two
places sum the same tokens, every plugin author writes that sum again,
and the panel of a session may disagree with the totals.

## What to build

1. Each `answer` names its session (for example `session: option<string>`,
   the id `sessions` gives), so OTTY can tell which rows belong to which
   session. The part fills it where the agent records it.
2. OTTY makes the numbers of a session (start, last activity, messages,
   tokens by model) and of a project (first and last activity, tokens by
   model) from the answers it already has, with the same dedupe and the
   same prices as the totals.
3. `read-session-stats`, `read-project-stats`, `session-stats` and
   `project-stats` leave the contract, the SDK, the host, `otty-agents`,
   the template and the official plugins. If a number of the panel can
   not come from the rows (for example a message count when the agent
   writes messages that are not answers), say so in the issue first and
   ask the owner before keeping a call for it.
4. Write the rule into `spec.md` (Decision 6) and the docs (`Write a
   Plugin.mdx`): a plugin gives rows, OTTY sums.

## Done when

- Tests first: the panel of a session and of a project shows the sums of
  that session's and that project's answers; an answer counted twice in
  the files counts once there too; the panel agrees with the totals.
- No `read-session-stats` or `read-project-stats` left outside `specs/`
  and git history.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-21, done. Each `answer` names its `session`; the template and
  the official plugins fill it (Claude Code: the session file, a helper
  agent's file counts in the session that ran it; Codex: the file's own
  `session_meta` id; OpenCode: the session nobody started, so helper
  sessions count in it). OTTY builds both panels from the answers it
  reads through `spend-places` / `read-answers`, with the same file
  memory as the totals: a panel reads every file of the whole history
  once, the totals keep what it read. Session: first and last answer,
  length, answers, tokens and tokens by model, its tokens by day and
  hour (the usage block, empty before), the account. Project: first and
  last answer, tokens and money by model. Same dedupe (one identity once,
  the most output) and the same tokens by provider, account and model
  as the totals, so the same prices. `read-session-stats`,
  `read-project-stats`, `session-stats`, `project-stats` and
  `model-tokens` left the contract, the SDK, the host and the plugins.
  OpenCode now reads every answer of its database, not only 33 days.

  For the owner (item 3), what the rows do not give:
  - **Messages** of the user in a session: an answer is not a message,
    and no row tells the prompts. The line is gone from the session
    panel (and its text key). To bring it back, a plugin must tell the
    prompts as rows of their own; that is a contract change, not made.
  - **Started** and **Last activity** of a session are now the first and
    the last answer. Before, Claude Code counted the first and last
    prompt, and OpenCode its session's created and updated times, which
    could be a little earlier or later.


- 2026-11-21, measured on the owner's real folders, read-only, release
  build, through `otty_agents::read_session_stats` / `read_project_stats`
  (the ignored `measure_*` tests in `crates/app/features/agents/io/src/stats.rs`).
  Each first panel uses agents that had read nothing yet. Old =
  the per-session plugin calls at 075373aa.

  | | Codex (~23 GB, 1437 sessions) | Claude Code (1.6 GB, 85 sessions) |
  | --- | --- | --- |
  | (a) first session panel | 53.8 s (old 0.015 s) | 3.8 s (old 0.04 s) |
  | (b) first project panel | 53.7 s (old 4.4 s) | 2.3 s (old 1.7 s) |
  | (c) second panel after (b) | 0.03 s (old 0.009 s) | 0.011 s (old 0.35 s) |
  | VmRSS after, two cold agents | 224 MB (old 26 MB) | 124 MB (old 26 MB) |

  About 90 MB (Codex) and 50 MB (Claude Code) of answers stay in memory
  per agent. The real OpenCode database holds no answers of the last 30
  days; a whole count of it takes 14 ms, an unchanged one 0.2 ms.
  Codex's first panel is too slow and its memory too large. The
  simplest fix: read for a panel only the answer files that the session
  memory already ties to the session or the project (what
  `read-sessions` said about each file), and the whole history only for
  an agent whose answers are not in its session files.

- 2026-11-21, panels read only their files. A session panel reads only
  the answer files the sessions read (issue 25) ties to that session; a
  project panel only those of the sessions whose folder is the project.
  Only while an answer file is no session file does a panel read the
  whole history (`PanelFiles::of`). Claude Code now gives the files of
  helper agents as session places: each names only the session that ran
  it, with no folder and no title. Codex tells no session for a helper
  agent's file, so as before its panels leave helper sessions out.
  Measured again the same way, after the sessions read that Agents mode
  does at start (Codex 5.1 s, Claude Code 0.06 s, both cold):

  | | Codex | Claude Code |
  | --- | --- | --- |
  | (a) first session panel | 0.055 s (old 0.015 s) | 0.059 s (old 0.04 s) |
  | (b) first project panel | 0.053 s (old 4.4 s) | 1.79 s (old 1.73 s) |
  | (c) second panel | 0.046 s | 0.015 s (old 0.35 s) |
  | VmRSS after | 36 MB (old 26 MB) | 76 MB (old 26 MB, peak 68 MB) |

  Each panel also lists and stamps the session files again (Codex: 2850
  files) to bring the sessions read up to date; that is most of its
  0.05 s. Claude Code's memory holds the answers of the project read in
  (b) for the rest of the run.
