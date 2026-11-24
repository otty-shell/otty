# 22 A key's account shows what you spent through it, counted by its plugin

Status: ready
Blocked by: 21

## Why

Owner, 2026-11-25: the Semrush Inc. organization (an Anthropic API key,
the owner is an employee, not an admin) shows no money since issue 01,
and an admin key will never come. The owner wants a number on the
account, done "purely as a change in the provider": the Anthropic plugin
is both Claude Code and the Anthropic provider, so its provider part can
count what went through the key in Claude Code's own sessions and price
it, with no profile↔account link in OTTY (spec Decision 1). The "Needs
from you" admin-key need stays as it is.

## What to build

1. **Prices for a provider part.** The host's `price-of` (today an
   import of the agent contract only, answered during a read of answers,
   `prices.wit` of `otty:agents-agent`) is open to a provider part too,
   during its `numbers` call, so a provider can price tokens the same way
   an agent part does. Change the WIT in place, SDK, host.
2. **Your spend this month.** The Anthropic provider part, for an account
   known by a key (an organization or a key account found in Claude Code
   folders), reads the Claude Code sessions in the folders it was found
   in (its `[[agents.provider.logins]]` entry gets the `read` globs it
   needs, such as `projects/**/*.jsonl`), sums this calendar month's
   tokens of the answers that went to Anthropic, prices each with
   `price-of` (`cost_at`, by kind), and returns it as the account's
   money: "Your spend this month ~$42.10, at API prices, from your Claude
   Code sessions". Use or extend the existing account money shape (the
   "spent this month" figure of issue 01) with the source and the "at
   API prices" mark, so OTTY shows it with the mark and its tooltip by
   kind (issue 16). An account with an admin key keeps the real bill
   first; this figure is for accounts without one. A subscription login
   shows its limits, not this.
3. **The OpenAI plugin** does the same for a key found in Codex folders,
   if its sessions say which provider served each answer; if they can
   not tell a key's answers apart, leave it and say so in the comment.
4. Words in all ten languages, the plugin pages say what the figure is
   and that it only counts this computer's sessions of that agent. Bump
   and rebuild changed plugins and the host test plugins.

## Done when

- Tests first: a provider part can price tokens during `numbers`; the
  Anthropic provider gives a key account this month's spend from the
  Claude Code sessions of its folders, priced by kind, marked at API
  prices; answers of another month or another provider are left out; an
  account with an admin key still shows its bill; the account page shows
  the figure with the mark and the tooltip by kind.
- The checks of `AGENTS.md` pass.

## Comments
