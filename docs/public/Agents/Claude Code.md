---
title: Claude Code
description: Where Agents mode takes the limits, tokens and money of a Claude Code profile from, what each source sends and to whom, and when it is used.
sidebar:
  order: 2
---

This page lists the sources of the numbers of a Claude Code profile, in the order OTTY tries them.
What the numbers, periods and states mean, and how the prices are read, is on
[Where the numbers come from](../where-the-numbers-come-from/).

## Kinds of profiles

- **Subscription**: a Claude plan, such as Pro or Max. The dashboard shows Limits and Tokens.
- **API key**: the profile's settings have an API key (`ANTHROPIC_API_KEY` or `apiKeyHelper`), its
  `.claude.json` has a `primaryApiKey`, or it is logged in to a Console organization that pays in
  advance. The dashboard shows Money, Prices and Tokens.

## Limits

1. **Anthropic usage API (direct request)**
   - What it does: OTTY reads the login token Claude Code keeps in `.credentials.json` in the
     profile folder and asks Anthropic's server for the limits of the account.
   - What it sends and to whom: one `GET` request to `https://api.anthropic.com/api/oauth/usage`
     with that token, the headers `Accept` and `anthropic-beta`, and OTTY's `User-Agent`. OTTY never
     writes, renews or keeps the token.
   - When it is used: first, at every read of the limits. An expired or refused token is "login
     expired" and the next source is tried. With no token file (on macOS the login may be kept in the Keychain),
     or a token that may not read the usage, it is "not available".
2. **Claude Code /usage (runs claude)**
   - What it does: OTTY starts `claude` in a hidden terminal, through your login shell, with the
     profile folder, types `/usage`, reads the limits from the screen and stops the program. It
     starts it with no tools, no MCP servers of yours, no hooks, no remote control and no automatic
     update, in a folder of its own, `~/.cache/otty/agent-probe`.
   - What it sends and to whom: OTTY itself sends nothing over the network. To show `/usage`, Claude
     Code makes its own requests with its own login; while it runs it may renew its login token, as
     it does whenever you run it.
   - When it is used: when the direct request gives no answer, and only while Agents mode is on
     screen. It gives up after 30 seconds. Its answer is reused for four and a half minutes, so
     pressing Refresh again in that time does not start `claude` again. When `claude` asks to log
     in, or asks any question other than whether to trust its own folder, OTTY answers nothing and
     stops it.

## Tokens

1. **Session files**
   - What it does: OTTY counts the tokens of every session file in `projects/` in the profile
     folder, the files of helper agents too. The sessions listed in Agents mode are only the ones
     you started.
   - What it sends and to whom: nothing; the files are read on your computer.
   - When it is used: at every read of the tokens.

## Money

1. **Anthropic cost report**
   - What it does: reads the bill of your whole organization, not only of this profile. You point
     OTTY at an admin key in the profile's Edit dialog: a file that holds it, or the name of an
     environment variable. OTTY keeps only that path or name, never the key.
   - What it sends and to whom: `GET` requests to
     `https://api.anthropic.com/v1/organizations/cost_report` for the 31 days up to today, with the
     admin key in `x-api-key`, the headers `anthropic-version` and `Accept`, and OTTY's
     `User-Agent`. The key is read from its file or variable for the read and dropped right after.
   - When it is used: first, when the profile has an admin key; with none it is "not available".
     The days of the bill are the days of Anthropic's report, in UTC.
2. **Estimate: tokens × LiteLLM prices**
   - What it does: the tokens of the session files at the LiteLLM list prices, an estimate. Its
     prices come from **LiteLLM prices** and **LiteLLM prices cache**, described on
     [Where the numbers come from](../where-the-numbers-come-from/#the-money-estimate-and-the-prices).
   - What it sends and to whom: nothing but the price table request described there.
   - When it is used: when the cost report gives no answer or there is no admin key; "not needed"
     while the cost report answers.

## What it changes in the agent's files

Only the hidden run of **Claude Code /usage (runs claude)** changes something in the profile
folder:

- Claude Code asks whether to trust the folder it runs in. OTTY answers "Yes" for its own folder,
  `~/.cache/otty/agent-probe`, and only for it. Claude Code saves that answer in the profile's
  `.claude.json`.
- Claude Code leaves a session file of that run in the profile folder. OTTY removes the session
  files of its own folder, and nothing else.
- Claude Code may renew its own login token while it runs.

The other sources only read.
