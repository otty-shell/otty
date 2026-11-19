# 14 Kimi Code

Status: needs-info
Blocked by: 01, 11

## Why

Spec Decision 10. Kimi Code 2.6.3 is installed but not logged in.

## Needs the owner

- Log in and run one short session, so its files can be read for real.
- An official Kimi logo (the prototype's one was assembled and must not be
  shipped).

## What to build

Profile (`~/.kimi-code`, `KIMI_CODE_HOME`); sessions
(`sessions/<workspace>/<id>/agents/main/wire.jsonl`); tokens; accounts from
`credentials/*.json` and `config.toml` keys; limits: find in Kimi Code's
source which request `/usage` makes, and use it like the Anthropic usage
API if it takes the stored login; Moonshot balance for a key; resume
commands.

## Done when

- Tests first on made-up data shaped like the owner's real files.
- The checks of `AGENTS.md` pass.
