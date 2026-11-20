# 14 OpenCode plugin; core `open_code` and Kilo deleted

Status: ready-for-agent
Blocked by: 07, 09

## Why

Spec Decision 10. OpenCode is its own official plugin with one agent part
and no provider part: the example of a plugin with a single part. Kilo
and Kilo Gateway are not in the official set (owner, 2026-11-20): today's
Kilo variant of the `open_code` reader is deleted with it, not ported.

## What to build

1. `plugins/opencode/`: own cargo workspace, `README.md`, the manifest of
   the spec's OpenCode example (plugin id `opencode`; `[api] agent =
   "0.1"`; one agent part `opencode` with `tested_version` `2.3.2`),
   the icon of `assets/svg/agent-opencode.svg`, made-up fixtures
   including a small SQLite database. The plugin ends with
   `export!(OpenCode with agent)`.
2. Port the OpenCode side of `crates/agents/src/open_code/`: profiles
   and the database place (`location.rs`, the `OPENCODE` entry of
   `fork.rs`: `XDG_DATA_HOME`, `OPENCODE_DB`, a database kept in memory
   has no file to read), sessions, spend from the database as a spend
   place of kind database (`spend.rs`, `message.rs`, the SQL text in the
   plugin), the cost each answer has (written by the agent), stats,
   provider ids (`provider_ids.rs`), commands.
3. Logins (`accounts.rs`): `auth.json` and the `credential` table (its
   `value` column is JSON: key fields declared as fields of a JSON
   column, issue 05); ChatGPT logins by account id with claims through a
   key handle; keys as key places. A Kilo login or key found in OpenCode
   is reported like any other login, for provider `kilo` or its host; no
   official plugin serves it, so it shows as "no provider plugin".
4. Not ported: the `KILO` entry of `fork.rs`, Kilo's `account.json`
   logins (several logins of one provider), the Kilo team of a login,
   and their tests.
5. Embed: the built copy in `assets/plugins/opencode/` with its hashes,
   by the script of issue 11; installed on first start with its part on
   (issue 08). With no OpenCode on the computer it finds nothing and does
   nothing.
6. Delete `open_code/` and the built-in OpenCode and Kilo entries; the
   owner checks OpenCode by hand.

## Done when

- Tests first: the ported tests of the OpenCode side of `open_code/`
  with the same made-up data; key columns and key fields of the JSON
  column come back as handles; the database is never written (opened
  read-only by the host).
- The core folder is gone, and no Kilo agent
  remains in the registry; the app shows OpenCode from the embedded
  plugin.
- The checks of `AGENTS.md` pass; the plugin's own tests pass.

## Owner checks by hand

Before the embed, add the plugin with **Add plugin from folder…** from
`plugins/opencode/` and pick it over the built-in reader; after, see it
as "Official · built in". The same OpenCode profiles, sessions and tokens
as with the built-in reader; a Kilo key found in OpenCode shows "no
provider plugin".

## Comments
- 2026-11-20, issue 13: the embedding script is `plugins/embed.sh <folder>`; how to use it and add the `embedded_plugins()` entry is in the comments of issue 13.
- 2026-11-20, issue 14: done. `plugins/opencode/` (plugin `opencode`, agent part `opencode`, `tested_version` 2.3.2) with the manifest of the spec, the icon of `assets/svg/agent-opencode.svg` in `currentColor`, and made-up fixtures: `auth.json` and `opencode.db` (second shape) in `fixtures/.local/share/opencode/`, made from `fixture-database.sql` with `sqlite3`; the other tests write made-up databases of both shapes in a temporary folder that `XDG_DATA_HOME` names. Embedded in `assets/plugins/opencode/`; the core `open_code/` (OpenCode and Kilo) is deleted.
- 2026-11-20, issue 14, key columns: logins come from `auth.json` (`files.read-json-with-keys`) and from `SELECT rowid, integration_id, value FROM credential` (the `value` comes back as `json-with-keys`; its `type`, `accountId` and `metadata.accountID` stay readable). Key places are `/<provider>/key|access` of `auth.json` and the `credential` field of the row (`/key` or `/access`). A ChatGPT login is told by its account id, with the email and the plan from `jwt-claims` of its access token; a key and a Kilo token (`gateway-token`, provider `kilo`) by the fingerprint OTTY takes; the same login in both places is given once. Providers OTTY does not read (a local model, `github-copilot`) and a Claude login (no id) are left out, as before; `moonshotai` is no longer mapped (Moonshot is dropped).
- 2026-11-20, issue 14, spend: the spend places are the database and its write-ahead log `<db>-wal` (kind database). OpenCode writes to the log first and copies into the database only now and then, so the host, which notices changes only by the size and time of a listed file, would miss new answers with the database alone. Either file reads the whole database (answers of the last 33 days, paged by rowid, 2,000 rows a query); each answer has its message id as identity, so OTTY counts it once. The record counts cover only the 20 newest answers, so old answers in an older format neither hide a change nor make one. A database moved by `OPENCODE_DB` outside the data folder has no listed log (a glob may not be `{OPENCODE_DB}-wal`), so its new answers show only after OpenCode copies them into the database.
- 2026-11-20, issue 14, SDK: the test host listed only files under the fixtures folder; it now walks from the plain start of the glob, as the host does, so a variable a part lists may point outside the fixtures (never `/` itself). A `HashMap` in a plugin makes its `.wasm` import `wasi:random/insecure-seed`, which the engine refuses at load; the installer of embedded copies did not notice. `otty-app-plugins-io` now has a test that installs every built-in copy with the real engine, and `otty-app-agents-io` reads the OpenCode fixture folder through the embedded plugin.
- 2026-11-20, issue 14, deleted with the core reader: `open_code/`, `account_numbers/kilo.rs`, the Kilo agent and Kilo provider entries, `KILO_ICON`, `OPENCODE_ICON`, `assets/svg/agent-kilo.svg` and `agent-opencode.svg`, `SourceId::{AgentDatabase, KiloBalance}` and their texts, `ConfigFolder::{with_database, database_variable}` (the app no longer reads `KILO_DB` / `OPENCODE_DB` into `AgentEnv`), and the Kilo team of `AccountKey`. Statistics of a plugin carry no cost, so a session or a project of OpenCode shows tokens and an estimate, not the cost OpenCode wrote; the profile's money still uses it.
