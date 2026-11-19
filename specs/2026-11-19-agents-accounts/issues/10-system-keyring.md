# 10 The system keyring

Status: ready-for-agent
Blocked by: 02

## Why

Spec Decision 8: some agents keep keys in the system keyring; OTTY may read
them with the user's permission.

## What to build

1. A new dependency, **approved by the owner on 2026-11-19**: the
   `keyring` crate (Linux Secret Service, macOS Keychain) or, if it cannot
   list entries of a service, `secret-service` on Linux and
   `security-framework` on macOS. Write the dependency description and the
   reason in the report before adding it, as `AGENTS.md` asks.
2. Read only the entries of known agents (Goose: service `goose`) and only
   after the user allowed it once: the need "System keyring · Allow OTTY to
   read it" in Accounts need you; the permission saved in `agents.json`.
3. Accounts found there show "Found in: system keyring".

## Done when

- Tests first: no keyring access before permission; the need shown and gone
  after Allow; the read behind a mocked keyring boundary (`mockall`).
- `cargo deny check` passes with the new crate.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/` (new keyring boundary), io, feature core, i18n,
`Cargo.toml`, `deny.toml` if needed.

## Owner checks by hand

Allow the keyring; the desktop asks to unlock once; Goose's keys show as
accounts.
