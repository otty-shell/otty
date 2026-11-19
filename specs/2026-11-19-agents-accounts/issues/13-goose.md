# 13 Goose

Status: needs-info
Blocked by: 10, 12

## Why

Spec Decision 10. The owner has Goose 1.60.1 logged in to ChatGPT, no
sessions yet.

## Needs the owner

One short Goose session on this machine, so the session database can be
read for real; and the SQLite approval of issue 12.

## What to build

Profile (`~/.config/goose`, `~/.local/share/goose`, `GOOSE_PATH_ROOT`);
sessions from `sessions/sessions.db` (provider per session, token totals,
cost); accounts from `chatgpt_codex/tokens.json` and the keyring (issue 10);
resume commands; logo from the official repository.

## Done when

- Tests first on made-up data shaped like the owner's real file.
- The checks of `AGENTS.md` pass.
