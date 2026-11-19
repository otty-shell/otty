# 15 Public docs

Status: ready-for-agent
Blocked by: 09, 10, 11

## Why

The docs say exactly what OTTY reads and sends (spec, Docs).

## What to build

Update `docs/public/Agents/`: the Overview explains accounts and profiles,
the needs, hiding, Add account, the keyring permission and "format
changed"; one page per provider and per agent under `Providers/` (rename
the group if a better name fits both) with every file read and every
request sent and to whom, including logins found in another agent. Remove
the budget. Check each claim against the code (as the 2026-11-19 audit
did).

## Done when

- Every page matches the code; the website builds the docs
  (`make dev` in `../website`).
- No Rust change, so no cargo checks.

## Comments
- 2026-11-19, issue 08: `agents.json` now also holds the user's account names and hidden accounts (`accounts`: provider + `account_id` / `organization_id` / `email`, `name`, `hidden`) and `hidden_profiles` (agent + folder); a hidden account or profile is not read. Open usage page opens the provider page listed in `Account::usage_page`.
