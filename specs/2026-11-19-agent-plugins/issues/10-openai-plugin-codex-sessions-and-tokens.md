# 10 OpenAI plugin, Codex part: profiles, sessions and tokens

Status: ready-for-agent
Blocked by: 07, 09

## Why

Spec Decisions 10 and 11 and "The first part: Codex". Codex is the
hardest reader: state across lines (the session total of the last
`token_count`, the model of the current `turn_context`, the project of
`session_meta`), zstd files, copies of the same answers in plain,
compressed, archived and forked files, and 10.8 GB of files on the
owner's machine. If the agent contract and host operations hold here,
they hold for the others. Codex is the agent part `codex` of the official
OpenAI plugin; its provider part `openai` comes in issue 15.

## What to build

1. `plugins/openai/`: its own cargo workspace on `otty-plugin-sdk` (by
   path), a `README.md`, `otty-plugin.toml` (plugin id `openai`, name
   "OpenAI"; `[api] agent = "0.1"`; one agent part `codex` with
   `tested_version` today's `0.169.0` and its own permissions), the
   Codex icon of `assets/svg/agent-codex.svg`, fixtures of made-up files
   shaped like the real ones. The plugin ends with `export!(OpenAi with
   agent)`.
2. Port from `crates/agents/src/codex/`: profiles (`~/.codex`,
   `CODEX_HOME`, extra `~/.codex*` holding a login or sessions); detect;
   version (`session_meta.cli_version`, else none); sessions
   (`session_index.rs`, `sessions.rs`, `session_file.rs`); spend places
   (session globs, plain and `.zst`, markers `"token_count"`,
   `"turn_context"`, `"session_meta"`); `read-answers` with the same
   rules as `spend_file.rs` (skip a repeated total, growth when no last
   usage, identity of time and numbers, provider per session from
   `model_provider`); record counts as issue 11 of the old spec; stats
   (`stats.rs`); launch, resume and login commands (`agent.rs`).
3. Permissions of the `codex` part: `read` of the Codex folders only;
   `env` `CODEX_HOME`; `run` of `codex --version`.
4. Not here: logins, limits, embedding, deleting core code (issue 11);
   the provider part `openai` (issue 15).

## Done when

- Tests first in the plugin (native, test host): every test of
  `codex/spend_file.rs`, `sessions.rs`, `session_index.rs`, `stats.rs`
  ported with the same made-up data and the same expectations; a `.zst`
  fixture equals its plain copy; renamed fields give seen but not
  understood.
- The plugin builds for `wasm32-wasip2`.
- `thread/list` appears nowhere in the plugin.
- The checks of `AGENTS.md` pass for OTTY; the plugin's own `cargo test`
  passes.

## Owner checks by hand

Build the plugin, add it with **Add plugin from folder…** from
`plugins/openai/`, and pick its Codex part over the built-in reader: the
same profiles, sessions, projects and token numbers as with the built-in
reader.

## Comments
- 2026-11-20, issue 07: plugin session stats have no per-day spend and no limit changes yet; reasoning tokens are dropped (counted in output); `writes_cost` turns true only after a part first gives a cost; a failed `detect` counts as NotLoggedIn. Check these against what the Codex part needs.
- 2026-11-20, issue 10: checked against the Codex part. Reasoning is a share of Codex's output, so dropping it loses nothing (the part still fills it). The part writes no cost, so `writes_cost` stays false, as before. Per-day spend and limit changes of a session (the session popup of the built-in reader) are missing until the contract has them. `detect` reads `auth.json` only through key handles, so the part declares the `auth.json` key fields now, not in issue 11. `HashMap` in a plugin imports `wasi:random`, which OTTY refuses: plugins use `BTreeMap`.
