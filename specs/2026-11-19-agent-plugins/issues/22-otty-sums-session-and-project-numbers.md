# 22 OTTY sums the numbers of a session and of a project; a plugin gives only rows

Status: ready-for-agent
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
