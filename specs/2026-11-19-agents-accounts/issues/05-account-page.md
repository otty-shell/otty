# 05 The account page

Status: ready-for-agent
Blocked by: 04

## Why

Spec Decisions 5 and 6: one page per account.

## What to build

Needs from you (when any); the Limits, Credits or Money block (Left, Spent
this month, Key limit); Used by chips (agent icon, profile, tokens today
via this provider, "uses 2 OpenAI accounts", "no tokens yet"; a chip opens
the profile); Account info (Provider, Email or Key, Plan, Found in as
folders, "system keyring" or "added by you"); Where the numbers come from.
An expired login shows "login expired" and the need "Open <agent>".

## Done when

- Tests first: the block rows for each kind; chips with and without a
  number; Found in texts; the expired-login need.
- The checks of `AGENTS.md` pass.

## Files

Feature core views (new `account_page.rs`), reducer, i18n.

## Owner checks by hand

Open Claude Max, the ChatGPT account shared by Codex and Kilo (after issue
12), an organization.
