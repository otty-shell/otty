---
title: Where the Numbers Come From
description: What each number of Agents mode means, which periods it covers, how OTTY picks a source, and how the money is estimated.
sidebar:
  order: 1
---

Agents mode shows how much of your coding agents you have used: the limits of a subscription, the
tokens spent and, for an API key, the money. This page explains what these numbers are and how OTTY
reads them. Each agent has its own page with its sources, in the order OTTY tries them, and what each
one sends and to whom:

- [Claude Code](../claude-code/)
- [Codex](../codex/)

The table "Where the numbers come from" at the bottom of a profile's dashboard shows the same thing
for that profile: one row per source, the number it gives, its state and when its data is from.
Point at a source name to see what it does, and at a state to see why it is what it is.

## The numbers

- **Limits**: how much of the usage windows of a subscription is used and when each window resets,
  or how many credits are left. The windows are the ones the agent reports; OTTY adds none of its
  own.
- **Tokens**: the tokens spent today, in the last 7 days and in the last 30 days, a bar for each of
  the last 30 days, and the top projects and models of the last 30 days. They are counted from the
  agent's session files on your computer, which also give the number of sessions and projects in
  the Profile list.
- **Money**: for a profile that pays with an API key, the money spent today, in the last 7 days and
  this month.
- **Prices**: the list prices the money is estimated at, when it is an estimate.

A subscription profile shows Limits and Tokens. An API key profile shows Money, Prices and Tokens;
it has no limits to read. Each agent's page says how OTTY tells the two apart.

## The periods

- **Today** is the current day in your time zone.
- **Last 7 days** and **last 30 days** are rolling: they end today and include it.
- **This month** is the calendar month, from its first day to today.

The days of the tokens and of the estimated money follow your time zone. A bill read from the
provider has the days of the provider's report.

## How OTTY picks a source

Each number can come from several sources. OTTY tries them in order, best first, and the first one
that answers wins; the ones after it are not asked. When a source fails, OTTY goes on to the next
one. When none answers, the last good numbers stay on screen, greyed, with their time.

The Updated column shows when the data of the source that answered is from: the time of the read, or
an older time when the source gives older data, such as a record in a file or a kept copy.

## What the states mean

- **answered**: this source gave the number on screen.
- **not needed**: a source before it answered, so this one was not asked.
- **login expired**: the login is missing, expired or refused. The tooltip says what to do: open the
  agent to log in again and press Refresh, or, when the agent renews its login by itself when it
  runs, says so.
- **not available**: this source can not be used here, such as a missing file, a missing key, a
  program that is not installed or that OTTY does not start while Agents mode is away, or no prices.
- **failed**: the source was tried and something went wrong: the network, an answer OTTY does not
  understand, a program that stopped, or a key that was refused. The tooltip says what.

## When OTTY reads, and Refresh

OTTY starts reading when it starts: the profiles, then the sessions, the limits, the tokens, the
prices and the bills, one after another. While Agents mode is away, it never starts an agent's
program to read the limits. While the mode is on screen, the limits and the bills are read again
every five minutes, and opening the mode reads again what is older than five minutes.

**Refresh** reads the profiles again and then everything else. The prices are read again only when
the last read of the LiteLLM file failed or is a day old; a good read from earlier today is kept.

## The money estimate and the prices

The money comes from the bill of the organization when the agent's page lists such a source and the
profile has what it needs to read it. Otherwise it is an estimate: OTTY counts the tokens of each
model in the session files and multiplies them by the list prices of that model, for input, cached
input, writing to the cache and output, with the higher prices of a long prompt where the model has
them. It knows nothing about discounts. A model with no price adds nothing, and a line under the
money says how many requests of this month have no price.

The providers give no prices through an API, so the prices come from the price table of the open
source LiteLLM project, a public file its community keeps up to date:
`https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json`.
Two sources give them, in this order:

1. **LiteLLM prices**: the file itself.
   - OTTY reads it only while a profile pays with an API key.
   - The request is a plain `GET` with no login and no account data; the only header OTTY adds is
     `User-Agent: otty/<version>`.
   - It is read at most once a day. A copy of the last good read is kept in
     `~/.cache/otty/prices/litellm.json`, and its time is the time it was read. Within the day,
     OTTY uses that copy instead of reading the file again.
2. **LiteLLM prices cache**: that copy, however old it is, when the read fails. Updated shows its
   time.

When there are no prices at all, the money shows "—" in place of each amount and a line under it
says "Could not get the prices. Press Refresh to try again."

## What OTTY sends

The requests each agent's page lists, and the price table request above, are the only network
requests OTTY makes for Agents mode. They follow no redirect, so a login token goes only to the
address named for it. Each request has the header `User-Agent: otty/<version>`; the agent's page
says what else it carries.

## OTTY only reads the agents' files

OTTY reads the agents' files and does not change them. Where running an agent's program to read its
limits changes something in its folder, that agent's page says what.
