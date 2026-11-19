# 09 Add account and admin keys

Status: ready-for-agent
Blocked by: 08

## Why

The last resort of spec Decision 7: a key no agent uses, and the admin key
of an organization bill.

## What to build

1. Add account dialog (Accounts "+"): provider (DeepSeek, OpenRouter,
   Moonshot; Anthropic or OpenAI organization), key file or environment
   variable, path or name. Saved in `agents.json` (path or name only).
2. Sources: DeepSeek `GET /user/balance` (Left), OpenRouter `GET
   /api/v1/key` (Key limit), Moonshot `GET /v1/users/me/balance` (Left),
   OpenAI `GET /v1/organization/costs` with an admin key (Spent this month),
   the existing Anthropic cost report. Same rules as today: OTTY's
   User-Agent, no redirects, 30 s timeout, the key read for the request and
   dropped.
3. Menu items: Admin key… on organization accounts; Edit key… and Remove on
   accounts added by hand (they also have Hide).

## Done when

- Tests first (mocked HTTP, made-up answers): each new source's answer and
  failure states; the saved entry has no key; Remove forgets it.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/` (new provider sources), feature core and io, i18n.

## Owner checks by hand

Add a DeepSeek key file; see Left; Remove it.

## Comments
- 2026-11-19, coordinator: issue 04 left the Accounts section without "+"; this issue adds it (Add account).
- 2026-11-19, issue 05: the account page's Money block shows only "Spent this month" (bill or "~" estimate). Add the rows Left and Key limit to `model/account_numbers.rs` when their sources come here.
- 2026-11-19, issue 08: the account menu is `model/menus.rs` (`AccountMenuViewModel`) + `view/menus.rs` (`account_menu`) + `menu_actions.rs` (`AccountAction`). Add `EditKey` and `Remove` there for accounts added by hand. Admin key… today opens the profile dialog's admin key field (`menu_actions::admin_key_profile`); give it its own dialog here if wanted. An account added by hand can keep its name and hidden flag by its saved entry (path or variable), never by the key fingerprint: `SavedProfiles::with_account_name` / `with_account_hidden` refuse a key-only account (`Account::can_be_saved`).
- 2026-11-19, issue 10: accounts found in the system keyring (Goose's
  `DEEPSEEK_API_KEY`, `OPENROUTER_API_KEY`, …) are known only by their key
  fingerprint and get no money yet. To read their Left or Spent, the key
  has to be read again from the keyring at the time of the request (only
  while the mode is on screen, the keyring may ask to unlock) and dropped
  right after; nothing of it is kept. Rename and Hide of them wait for the
  hash decision (SHA-256) like any key account.
