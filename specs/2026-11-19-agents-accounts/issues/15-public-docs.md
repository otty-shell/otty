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
- 2026-11-19, issue 11: "format changed": when the newest session files hold at least 5 records of the kind OTTY reads (Claude Code `assistant` records, Codex `token_count` events) and fewer than half are understood, the profile shows "format changed", keeps its last good numbers greyed, and has the need with Copy report. The report (English) holds agent, version, tested-up-to version, folder, file kind, counts and field names; never a value. Tested versions: Claude Code 2.1.328, Codex 0.169.0 (`TESTED_VERSION` in each `spend_file.rs`).
- 2026-11-19, owner: Kimi Code (issue 14) is deferred. The docs describe only what works: no Kimi Code agent page and no Kimi in the agent lists. A Moonshot API key added by hand is still a provider account (issue 09).
- 2026-11-19, issue 09: an account added by hand is kept in `agents.json` `accounts` as provider + `key` (`file` or `env_var`) with its `name`/`hidden`; never the key. New requests, each with the account's own key read right before and dropped after: DeepSeek `GET https://api.deepseek.com/user/balance`, OpenRouter `GET https://openrouter.ai/api/v1/key`, Moonshot `GET https://api.moonshot.ai/v1/users/me/balance`, Kilo `GET https://api.kilo.ai/api/profile/balance` (Kilo login token from `auth.json`, team in `x-kilocode-organizationid`), OpenAI `GET https://api.openai.com/v1/organization/costs` (admin key), the Anthropic cost report for an added organization, and the ChatGPT usage request (`chatgpt.com/backend-api/wham/usage`) with the token of a ChatGPT login found only in Kilo/OpenCode. Keys of Claude Code `settings.json` and of the keyring are read again for these requests too.
