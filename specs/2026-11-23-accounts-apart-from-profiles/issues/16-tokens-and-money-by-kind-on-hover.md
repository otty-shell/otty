# 16 Tokens and money say what they are made of on hover

Status: ready
Blocked by: 15

## Why

Owner, 2026-11-24: the usage shows "400K tokens · ~$12.40", and the
money is priced per kind of token (input, cache write, cache read,
output, `cost_at` in `crates/plugins/sdk/src/agents/prices.rs`). The
token total adds input, cache reads and output, and cache reads are
usually by far the largest part, so the number looks huge while it costs
little. The owner chose to keep the number and show what it is made of
on hover, for the tokens and for the money too: "when I hover the money,
let it show in / out / cached in money as well".

## What to build

1. **Money per kind from the part.** An answer's cost at API prices
   carries its money per kind: input, cache write, cache read, output
   (millionths of a dollar each), next to the total. Change the WIT in
   place (`answer-cost::at-api-prices` of
   `crates/plugins/sdk/wit/agents-agent/0.1.0/`), the SDK (`cost_at`,
   `answer_cost` give the parts), the host, and `otty-agents`
   (`ModelTokens` money, `MoneyTotal` adds the parts). A cost the agent
   wrote (`written`, OpenCode) has no parts: it stays one number.
   Rebuild every official plugin and the template with a patch bump,
   refresh the host test plugins.
2. **Tokens on hover.** The total of the usage block, each bar and each
   Tokens cell of the usage tables (profile page, the statistics popups)
   show a tooltip with one kind per line, its name on the left and its
   number on the right (in, cache write, cached, out). A kind with zero is
   left out.
3. **Money on hover.** The money of the total, each bar's money and each
   Money cell show a tooltip with the same kinds in money, one per line
   ("in ~$1.20", "cached ~$0.40", …). Money the agent wrote shows
   "written by the agent" with no parts, and a mix shows the parts of the
   priced share and the written share as one more line. A kind with no
   money is left out. Before prices, no money tooltip.
4. **No middle dot " · " anywhere in the UI** (owner, 2026-11-24: it is
   too small and unclear everywhere). In all ten locales and every place
   that builds text in code (grep `·` in `crates/`, not in tests only):
   - a list (tooltip breakdowns, the failure reasons of
     `agents_failure_separator`, a bar's lit part) has one item per line,
     no separator;
   - two values side by side take a comma: "400K tokens, ~$12.40",
     "12 credits, renews Dec 1", "5 h, gpt-5";
   - the parts of a name take parentheses: "Codex (Work)", "Claude Code
     (Work)", "OpenAI (you@example.com)", for profile names, account
     full names, need places and every other name built of two parts.
   A test over the locales fails on "·", and the view tests that matched
   names follow. The decimal point of money stays.
5. Words in all ten locales, commas not semicolons. Docs
   (`docs/public/Agents/Overview.mdx`, Write a Plugin for the new cost
   shape) follow.

## Done when

- Tests first: a priced answer carries its money by kind and the parts
  add up to its total; a written cost has no parts; the token tooltip of
  the total, a bar and a cell lists the non-zero kinds; the money
  tooltip lists the kinds in money, says "written by the agent" for a
  written cost, and shows both for a mix.
- The checks of `AGENTS.md` pass.

## Comments
