---
title: Codex
description: Where Agents mode takes the limits, tokens and money of a Codex profile from, what each source sends and to whom, and when it is used.
sidebar:
  order: 3
---

This page lists the sources of the numbers of a Codex profile, in the order OTTY tries them. What
the numbers, periods and states mean, and how the prices are read, is on
[Where the numbers come from](../where-the-numbers-come-from/).

## Kinds of profiles

- **Subscription**: a ChatGPT login, such as Plus or Pro, or a workspace that pays with credits.
  The dashboard shows Limits (usage windows or credits) and Tokens.
- **API key**: the profile's `auth.json` says it logs in with an API key. The dashboard shows Money,
  Prices and Tokens.

## Limits

1. **ChatGPT usage API**
   - What it does: OTTY reads the login token Codex keeps in `auth.json` in the profile folder and
     asks ChatGPT's server for the limits of the account.
   - What it sends and to whom: one `GET` request to `https://chatgpt.com/backend-api/wham/usage`,
     or to the server named by `chatgpt_base_url` in the profile's `config.toml`, with that token,
     the workspace id of the login, the header `Accept` and OTTY's `User-Agent`. For a team or
     enterprise workspace whose answer says credits are on but gives no numbers, up to two more
     `GET` requests go to the same server with the same headers, for the credits used this month and
     the balance of the workspace. OTTY never writes or renews the token.
   - When it is used: first, at every read of the limits. A missing, expired or refused login is
     "login expired".
2. **Codex app-server**
   - What it does: OTTY starts `codex app-server`, through your login shell, with the profile folder
     and in read-only mode, and asks it for the limits of its login. It asks nothing else; in
     particular it never asks for the list of sessions. The program is stopped right after.
   - What it sends and to whom: OTTY talks only to that program on your computer. How Codex gets the
     answer is up to Codex.
   - When it is used: when the request above gives no answer, and only while Agents mode is on
     screen. It gives up after 10 seconds.
3. **Latest Codex session**
   - What it does: after each answer of the model, Codex writes the limits it got into the session
     file. OTTY takes the latest record from the 10 newest session files. These limits are as old as
     the record, and Updated shows its time.
   - What it sends and to whom: nothing; the files are read on your computer.
   - When it is used: when neither source above answers.

## Tokens

1. **Session files**
   - What it does: OTTY counts the tokens of every session file in `sessions/` and
     `archived_sessions/` in the profile folder, plain and compressed, the files of helper agents
     too. The sessions listed in Agents mode are only the ones you started.
   - What it sends and to whom: nothing; the files are read on your computer.
   - When it is used: at every read of the tokens.

## Money

1. **Estimate: tokens × LiteLLM prices**
   - What it does: the tokens of the session files at the LiteLLM list prices, an estimate. Its
     prices come from **LiteLLM prices** and **LiteLLM prices cache**, described on
     [Where the numbers come from](../where-the-numbers-come-from/#the-money-estimate-and-the-prices).
     OTTY does not read the bill of an OpenAI organization.
   - What it sends and to whom: nothing but the price table request described there.
   - When it is used: at every read of the money of an API key profile.

## What it changes in the agent's files

OTTY changes nothing in the profile folder: the files are only read, and `codex app-server` runs
in read-only mode and is asked only for the limits.
