# 12 Money for API key profiles: the price table and the estimate

Status: ready-for-agent
Blocked by: 09, 11

## Why

A profile billed by an API key has no limits; what matters is the money it
spent (spec Decision 9). Without an admin key OTTY can only estimate it from
the tokens of the session files and the list prices.

## What to do

1. A price table in `otty-agents`, as data in one file: for each model id
   (or model family prefix, when the provider prices a family the same), the
   price per million input, cached input (read and write where they differ)
   and output tokens, in US dollars, and the date the prices were taken.
   Fill it from the providers' public price pages for the Claude and OpenAI
   models the agents use; put each page's address in a comment. Read
   CodexBar's `CostUsagePricing` for the model ids and the rules it uses
   (for example the Codex "fast" tier) and follow it where it fits.
2. A function that turns the per-model tokens of `Spend` (issue 09) into
   money: today, last 7 days, this month, per day for the last 30 days, per
   model this month. A model with no price keeps its tokens and is counted
   as "no price"; it adds nothing to the money and the dashboard says how
   many requests had no price. Nothing is guessed from a similar model.
3. The money result says it is an estimate and from which price table date.
4. It runs only for API key profiles (issue 11).

## Done when

- Tests, written first, cover: a known model; a family prefix; cached input
  priced apart; a model with no price; the day and month edges in the user's
  time zone (the time is passed in, as in issue 09).
- The checks of `AGENTS.md` pass.

## Comments
