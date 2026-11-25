# 05 Host operations: key handles, HTTP, keyring

Status: ready-for-agent
Blocked by: 03

## Why

Spec Decision 5: plugins never see a raw key. Today every reader turns a
key into a fingerprint at once (`claude_code/accounts.rs`,
`open_code/accounts.rs`, `keyring_entry.rs`) and reads it again right
before a request (`account_numbers/mod.rs`, `KeyPlace`). The host must
keep both rules for code it does not control.

## What to build

1. The `key` resource: holds where the key is, not the key. `fingerprint`
   (SHA-256 as `key_fingerprint.rs`), `has-prefix`, `jwt-claims` (the
   decoded middle part of a JWT, as `codex/jwt.rs`; never the signature
   or the token). Each use reads the key from its place and drops it
   after.
2. `keys.open(place)` for a key place the called part declares: JSON
   field of a file (pointer with `*` allowed in the manifest),
   environment variable, keyring entry (+ JSON pointer), SQLite field
   (table, column, row id, + JSON pointer for a JSON column such as
   OpenCode's `credential.value`). A provider part gets handles from
   OTTY (issue 07) and opens none itself.
3. `files.read-json-with-keys(path)`: the JSON with the declared key
   fields replaced by `{"$key": <handle>}`; `keyring.read` the same for
   an entry. SQLite: declared key columns come back as `key` values, and
   declared JSON columns with their key fields replaced by handles.
4. `http.request`: HTTPS only, host in the called part's `http` list, no
   redirects, 30 s, 8 MiB answer, OTTY's `User-Agent`; a header with a
   key handle gets `prefix + key` filled in by the host right before
   sending. Built on today's `https.rs` (`ureq` with rustls), moved here.
5. `keyring.read(service, user)`: only entries the called part declares,
   only after the user's Allow (the host gets the allowance from the
   app), through the `KeyringRead` trait of today's `system_keyring.rs`,
   moved here; tests use its `mockall` mock and never touch the real
   keyring. No official plugin reads the keyring now (Goose is dropped,
   spec Decision 10); the operation stays for any plugin that declares
   an entry.
6. No log, error or `Debug` output of the host ever holds a key value.

## Done when

- Tests first: a plugin gets a handle, never the text, from a JSON file,
  a variable, a keyring entry (mock), a SQLite column and a field of a
  JSON column; fingerprints equal today's for the same key; `jwt-claims`
  of a made-up JWT gives its claims and nothing else; a request to an
  unlisted host, or to a host only another part lists, is refused; the
  header carries the key only in the sent request (checked with a mock
  HTTP client); a key place not in the manifest, or declared only by
  another part of the same plugin, is refused; keyring before Allow is
  refused; `Debug` of every host type holds no key.
- The checks of `AGENTS.md` pass.

## Files

`crates/plugins/host/src/` (keys, http, keyring), moved code from
`crates/agents/src/https.rs`, `system_keyring.rs`, `key_fingerprint.rs`,
`codex/jwt.rs` (copied while in-core readers still use them).

## Comments

### 2026-11-20: key columns in `sqlite.query`

- A result column that is a declared key column now comes back as a
  `key` value, or as `json-with-keys` for a column declared with fields,
  instead of being refused; the plugin no longer needs `keys.open` with
  `sqlite-field` to read the other fields of a JSON key column (rusqlite
  feature `column_metadata`, approved by the owner).
- The rule: the plugin's SQL reads every key column as `NULL` (the
  authorizer answers `SQLITE_IGNORE`), so no `WHERE`, join, function,
  subquery or view can learn anything of a key. The statement is first
  only prepared, with key columns allowed, to learn from SQLite where
  each result column comes from; a result column whose origin is a key
  column is filled in by the host, read at the row named by the result's
  rowid column of the same table (`rowid` or its `INTEGER PRIMARY KEY`).
  No rowid column of that table in the result: the query fails. A
  statement that names a key column more often than it has key result
  columns (a key column in `WHERE`, `ORDER BY`, a join or a function) is
  refused with `not-allowed`; uses SQLite does not report this way (a
  subquery or a CTE that filters on the key) are not refused but see
  `NULL`. A key column with no key, or JSON that does not parse, comes
  back as `null`. `ATTACH` and statements that are not read-only stay
  refused. The SDK test host follows the same rule.
