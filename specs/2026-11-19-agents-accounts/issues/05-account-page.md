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

## Comments

Built (2026-11-19):

- Pressing an account (left panel, All accounts) opens its page
  (`model/account_page.rs`, `view/account_page.rs`), not the dashboard of
  its first profile any more. Header: provider icon, name, Refresh; no
  line under the title.
- Needs from you (only when there is any): the needs of this account
  (`needs_of_account`). New need: when the latest limits read failed
  because of the login (`NotLoggedIn`, or `LoginExpired` that the agent
  renews), the row number is "login expired" in the error color (left
  panel, All accounts, Found by OTTY too), and the need "The login expired"
  has the button "Open <agent>" (`NeedAction::OpenAgent`: the agent of the
  profile the limits are read through, in a terminal tab in the home
  folder, only on the user's press).
- Numbers block (`model/account_numbers.rs`) titled Limits, Credits or
  Money: the usage windows or the credits as read (the last good numbers
  greyed after a failed read), or "Spent this month" from the bill, else
  the estimate over the tokens of its profiles as "~ $42.10". The limits
  and money views take their title, so the dashboard keeps "Usage".
- Used by (`model/used_by.rs`): a chip per profile the account was found
  in, with the agent icon, the name and "1.2M today" (tokens the account
  paid today), "no tokens yet", "uses 2 OpenAI accounts" (several accounts
  of the provider and no token names its account), or nothing before the
  count. A chip opens the profile. `otty-agents`: the rule of which tokens
  an account paid moved from `AccountMoney` to `Account::pays_for`, so the
  chips and the money use the same rule.
- Account info: Provider, Email or Key ("key ••••a1f3", "—" when the key
  is too short to show), Plan (from the files, else the limits answer),
  Found in (one row per place: folder, "system keyring", "added by you";
  shared with the Found by OTTY report in `model/found_in.rs`).
- Where the numbers come from: the limit sources of the profile the limits
  are read through, or the money sources (bill, estimate, prices) of the
  first profile the account was found in.
- i18n: 12 new keys in the 10 languages.

Decided differently:

- Money shows only "Spent this month": Left and Key limit have no source
  yet; they come with the sources of issue 09.
- The chips are drawn in this page's view; issue 06 can move them to a
  shared view for Pays through.
- No "⋯" on the page yet: its menu is issue 08.
