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

## Comments

- 2026-11-19, owner: agreed to log in to Kimi Code and run one short session, and to provide an official Kimi logo.
- 2026-11-19, owner: Kimi logo source: https://lobehub.com/icons/kimi (LobeHub icons). Take the single-color (mono) SVG, keep it `currentColor` like the other icons in assets/svg, note the source and the icon set's license next to it.
- 2026-11-19, owner: deferred. The owner has no Kimi subscription, so there is no real session to read. Do not start this issue until the owner brings it back.
