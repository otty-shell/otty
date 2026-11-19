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

## Comments

- 2026-11-19, owner: `rusqlite` (bundled) is approved. The owner agreed to run one short Goose session on their machine so its database can be read live.
- 2026-11-19, issue 10: the keyring entry of Goose is read already
  (`otty_agents::keyring_entries`, `read_keyring_accounts`), but Goose's
  folder is found only at `~/.config/goose`. Here: honour
  `GOOSE_PATH_ROOT` (`$root/config`) and `XDG_CONFIG_HOME`, move the
  entry next to the Goose agent, link its keyring accounts to the Goose
  profile ("Used by"), and skip a key whose provider host was changed in
  `config.yaml` (`OPENAI_HOST`, `ANTHROPIC_HOST`): it is not the provider's
  key then.
- 2026-11-19, owner: ran one short Goose session. Paused: Goose will not get its own Rust code. It becomes the first agent read from a description file (shared engine code only, nothing written for one agent by name); see the upcoming spec on agent and provider descriptions.
