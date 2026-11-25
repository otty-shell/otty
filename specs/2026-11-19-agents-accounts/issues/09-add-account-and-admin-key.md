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
- 2026-11-19, issue 12: the Kilo balance moves here: `GET
  https://api.kilo.ai/api/profile/balance` with the Kilo login token of
  `auth.json` (`kilo` entry; header `x-kilocode-organizationid` for a
  team), as Left of the Kilo account. Kilo's accounts are found by
  `open_code/accounts.rs` (a Kilo login is a key account by the token's
  fingerprint); read the token again at the request and drop it.
- 2026-11-19, coordinator (from issue 12): a ChatGPT login found only in Kilo/OpenCode shows no limits yet. Read its limits with the same ChatGPT usage request Codex uses, with that login's own token (read-only, never renewed), so every found ChatGPT account has its limits.

Built (2026-11-19):

- Key fingerprints are SHA-256 now (`sha2` 0.11.1, the version the
  keyring store already builds; no new crate in `Cargo.lock`), still kept
  in memory only.
- Add account (Accounts "+", `add_account_dialog.rs` with its model and
  view, overlay `AgentsAddAccountDialog` in the binary, Esc cancels): a
  provider (DeepSeek, OpenRouter, Moonshot: money of a key; Anthropic,
  OpenAI: bill of an organization with an admin key) and a key file or an
  environment variable. `agents.json` keeps it in `accounts` as
  `{ "provider": "deepseek", "key": { "file": … } }` (or `env_var`): the
  place only, never the key or its fingerprint. Such an account is known
  by that place (`AccountIdentity::AddedKey`, `Account::added`), so Rename…
  and Hide work for it; it is found "added by you" and named "DeepSeek ·
  $DEEPSEEK_API_KEY" or by the file path. The key field is shared with the
  admin key field of the profile dialog (`key_place_field.rs`).
- Menu of an added account: Edit key… (the same dialog, provider fixed,
  name and hidden flag kept) and Remove (forgets the entry and its name, no
  confirm: nothing on disk but `agents.json` changes).
- Sources (`otty-agents`, `account_numbers/`): DeepSeek `GET /user/balance`
  (Left, in its currency, "¥" for CNY), OpenRouter `GET /api/v1/key` (Key
  limit: used of limit with a bar, or "no limit"), Moonshot
  `GET api.moonshot.ai/v1/users/me/balance` (Left, USD), Kilo
  `GET api.kilo.ai/api/profile/balance` with the Kilo login token of
  `auth.json` and `x-kilocode-organizationid` = its `accountId` (Left),
  OpenAI `GET /v1/organization/costs` with an admin key (Spent this month,
  31 UTC days, pages walked), and the existing Anthropic cost report for an
  added Anthropic organization. Same HTTP rules as before (`UreqClient`).
  The key is read again right before each request from where it was found
  (`KeyPlace`: the place the user gave, the files of the profile through
  the new `Agent::account_key`, or the system keyring, only while Agents
  mode is on screen) and dropped right after it.
- Found key accounts read their money too: a DeepSeek/OpenRouter/Moonshot
  key of Claude Code's `settings.json`, keys and the Kilo login of
  Kilo/OpenCode, and keys of the keyring (Goose).
- A ChatGPT login whose profiles' agents read no limits (only Kilo or
  OpenCode keep it) gets its limits with the Codex usage request and its
  own access token, read-only, never renewed (expired → login expired).
- Feature: `own_reads.rs` keeps the read of each account (last good
  numbers stay greyed after a failure); a new read kind after the bills at
  start, on Refresh, on the timer. Account page Money block: Left, Key
  limit, then Spent this month (profile bill, own bill, or estimate);
  Where the numbers come from lists the own source. Lists show "¥110.00
  left" / "$12.30 used".
- i18n: 27 new keys in 10 languages; the "admin key file … is missing"
  texts now say "key file".

Decided differently:

- A key OTTY found (fingerprint) is still never written to `agents.json`:
  the spec keeps fingerprints in memory only, so its Rename… and Hide stay
  off. Saving SHA-256 fingerprints would make them possible; owner's call.
- An added key is its own account even when an agent uses the same key:
  OTTY does not read the key to match it at load time.
- Moonshot is asked at `api.moonshot.ai` (USD); a `moonshot.cn` key gets an
  error there.
- A ChatGPT login read with its own token shows "login expired" in its
  number when the token expired, but no "The login expired" need yet.
- Admin key… of a found organization still opens the profile dialog.
