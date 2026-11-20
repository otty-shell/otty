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
