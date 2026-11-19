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

## Comments

- 2026-11-19, owner: the keyring dependency is approved. Use `keyring`; if it cannot list entries, `secret-service` (Linux) and `security-framework` (macOS). Write the dependency note (what, why) in the report before adding it.

Built (2026-11-19):

- Dependency: `keyring` 4.2 (MIT or Apache-2.0) in `otty-agents`, default
  feature `v1` only: on Linux the Secret Service through
  `zbus-secret-service-keyring-store` (zbus 5 with async-io, which iced
  already builds; pure Rust crypto `aes`, `cbc`, `hkdf`, `sha2`; no
  libdbus, no tokio), on macOS the Keychain through
  `apple-native-keyring-store` (security-framework 3). It needs no search:
  Goose keeps all its secrets in one entry (service `goose`, user
  `secrets`, a JSON map of variable names). `cargo deny check` is green.
- `otty-agents`: `system_keyring.rs`, the boundary `KeyringRead` (one read
  method, no write; mocked with mockall) and `KeyringError` (NotAvailable,
  NotAllowed, Unreadable; it never holds what the keyring holds; only a
  platform failure is logged). `keyring_entry.rs`: `KeyringEntry` (only
  this crate makes one: Goose), `keyring_entries(env)` (the entries of the
  agents whose folder is here, `~/.config/goose`; nothing is read) and
  `read_keyring_accounts(entries)`: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
  `DEEPSEEK_API_KEY`, `OPENROUTER_API_KEY`, `MOONSHOT_API_KEY` become key
  accounts (fingerprint and last 4 characters) found in "system keyring";
  other secrets are skipped. `agents.json` gets `"keyring_allowed": true`.
- io: the profiles read gives `LoadedProfiles::keyring_entries`;
  `load_keyring(entries)` reads them on a thread of its own.
- Feature (`keyring.rs`, model and view `keyring_dialog.rs`): Accounts need
  you has "System keyring · Allow OTTY to read it" while a known agent is
  here, `agents.json` was read and the read is not allowed; Allow keyring
  opens the keyring dialog (Not now / Allow, says which agents and which
  service it reads, that it never changes them, that the system may ask to
  unlock). Allow saves the allowance and reads at once. An allowed keyring
  is read only while Agents mode is on screen (the unlock prompt), once a
  run and again after Refresh. The accounts are listed with the others
  (All accounts, left panel, Found by OTTY); a failed read keeps the
  accounts read before and shows "OTTY could not read it" with Try again.
- Binary: `LoadKeyring` dispatch and the overlay `AgentsKeyringDialog` in
  the guards (Esc = Not now). i18n: 14 keys in the 10 languages.

Decided differently:

- The need shows only when an agent that keeps keys in the keyring is on
  this computer (Goose's folder), not always as in the prototype.
- No key from the keyring is saved or sent anywhere: keyring accounts are
  known by their FNV fingerprint, so Rename and Hide stay off for them
  (issue 08 rule), and no money is read for them yet (issue 09).
- With no Secret Service at the first read, the `keyring` crate keeps that
  answer for the run: Try again helps after a locked or refused read, a
  missing keyring needs a restart of OTTY.
- Goose's folder is looked up in `~/.config/goose` only; `GOOSE_PATH_ROOT`
  and `XDG_CONFIG_HOME` are left to issue 13.
