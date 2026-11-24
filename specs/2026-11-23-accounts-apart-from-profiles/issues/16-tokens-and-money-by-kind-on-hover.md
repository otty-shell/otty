# 16 Tokens and money say what they are made of on hover

Status: done
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

2026-11-25, done. Tokens and money tell what they are made of on hover,
and no text of the UI holds " · " any more.

Contract (changed in place): `agent.wit` has a new record `priced-cost`
(`total`, `input`, `cache-written`, `cached`, `output`, millionths of a
dollar), and `answer-cost::at-api-prices` carries it. `written(u64)`
stays one number. SDK: `cost_at` gives a `PricedCost`, each kind rounded
on its own and the total their sum, so the parts always add up, and
`answer_cost` gives it as `AtApiPrices`. The host exports `PricedCost`.
`otty-agents`: a new `MoneyByKind` (input, cache write, cached, output),
`AnswerCost::AtApiPrices(MoneyByKind)`, and `MoneyTotal` keeps the
written cost apart from the money at API prices by kind
(`written_cost`, `priced_by_kind`, and `cost` adds both).
`MoneyTotal::at_api_prices` takes a `MoneyByKind`, and the sum adds kind
by kind.

Screen: the usage block keeps the tokens of each row kind by kind
(`UsageGroup::tokens` is a `TokenCount` per point). A new `Breakdown`
(`model/breakdown.rs`) lists the kinds: tokens as in (the input not
written to the cache, so the kinds add up to the total shown), cache
write, cached and out, money as the same kinds at API prices with "~",
then "written by the agent" for the written share, which has no parts. A
kind with nothing is left out, and there is no money before the prices
are read or for money with no price. The line under the title, each bar
and the Tokens and Money cells show it: the line and a bar in one
tooltip each (the line of the bar first, then the kinds of the tokens,
then their money), each cell its own. The line has one tooltip for its
tokens and its money, since the words of the line differ by language and
are not cut in two. The kit tooltip takes a `TooltipText` (lines, then
groups of rows, each name at the left and its number at the right),
which `tooltip`, `DayBars`, `TableCell::with_tooltip` and
`StatusBadge::tooltip` take, a `String` being a one-line one.

No middle dot: `agents_failure_separator` is gone and failures are one
per line, a bar's lit row is a second line of its tooltip, and "400K
tokens, ~$12.40", "$42.10, at API prices", "3 760 left, Resets …" and "7
days, Opus" take a comma (the comma of each language, "、" in Japanese,
"，" in Chinese). An account's full name is "OpenAI (you@example.com)",
the heading of a need "Codex, needs login" (new
`tpl_agents_need_heading`), the note of a plugin row "Official, failed"
(new `tpl_plugins_trust_and_sign`), and the Anthropic plugin names a
subscription of an organization "you@example.com (Example Org)". Test
names follow ("Codex (Work)"). New words in all ten locales: in, cache
write, cached, out, written by the agent. A test over the locales fails
on "·". Kept on purpose: the "·" of the DEC special graphics charset in
`otty-escape`, and the Claude Code screens the Anthropic plugin reads in
its tests, which are the agent's own words. Left as it is: the full name
of an Anthropic organization login nests, "Anthropic (you@example.com
(Example Org))", and "Resets" keeps its capital after the comma.

Plugins: `anthropic` and `openai` 0.2.14 -> 0.2.15, `opencode` 0.1.10 ->
0.1.11, `deepseek` and `openrouter` 0.1.6 -> 0.1.7, template `example`
0.1.8 -> 0.1.9, all rebuilt into `assets/plugins`, and the host test
plugins and `template.wasm` rebuilt. Docs: Overview.mdx (what a hover
shows, commas), Write a Plugin (the `PricedCost` shape, template 0.1.9),
the versions of the Official Plugins pages, the SDK README.

Tests first: they were written and committed before the code and failed
to build (no `PricedCost`, `MoneyByKind`, `Breakdown`, `TooltipText` or
new keys yet). They cover the SDK parts adding up to the total and a
written cost with none, `MoneyTotal` adding kind by kind with the written
share apart, the breakdown of tokens, of money, of a written cost and of
a mix, the total, a bar and a row carrying it, the cells and a bar
showing it as tooltips, the kit rows with the name left and the number
right, failures one per line, and no "·" in any locale.
