# 07 Plugin parts in Agents mode

Status: ready-for-agent
Blocked by: 01, 04, 05

## Why

Spec Decisions 4, 6 and 11. The generic work stays in `otty-agents`
(`walk.rs`, `spend_files.rs`, `spend_tally.rs`, `record_format.rs`,
`merge_accounts`, prices); a plugin part only replaces the per-agent
closures and sources. Each plugin issue then deletes the in-core code
its part replaces; the owner checks it by hand (no parity check).

## What to build

1. `otty-agents` depends on `otty-plugin-host`. `PluginAgent` implements
   today's `Agent` trait (as reshaped by issue 01) for one agent part by
   calling it with its agent id: profiles, detect, version, logins →
   accounts with key places, sessions, launch and login commands, limit
   sources, stats. Its `SpendCounter` walks the part's spend places with
   today's walk and `FileMemory`, calls `read-answers` only for changed
   items, counts answers by identity once, and gives the "format changed"
   verdict from the part's `RecordCounts` and the part's
   `tested_version`.
2. `PluginProvider` for one provider part: for each joined account whose
   provider id (or host) that part serves while it is on, open a key
   handle at one of the account's places and call `account`, `numbers`,
   `bill` once per account per refresh, as `account_numbers/mod.rs`
   does. A part gets handles only for accounts of its own id or hosts.
3. Registry: the parts that are on add their agents and providers, one
   part per agent id, provider id and provider host. When a part and a
   built-in reader serve the same id, the built-in reader stays until the
   user picks the plugin (the same choice as between two plugins, issue
   08), and until its issue deletes it.
4. Report: `FormatChange::report` and the new "plugin could not read"
   report add the plugin id, version, repository and the part id; the
   need names the plugin (spec table).
5. A trap or missed deadline fails that read only and keeps the last good
   numbers greyed; three in a row pause only the part whose calls failed,
   until **Try again** or restart. The plugin's other parts keep working.

## Done when

- Tests first: the template plugin (issue 03) as a `PluginAgent` gives
  the expected `Spend` over fixtures; an unchanged file is not read
  again; the same answer in two files counts once; a part that
  understands under half of the newest records gives "format changed"
  naming the plugin; a trapping part keeps last good numbers and is
  paused after three, while the template's other part keeps reading; one
  provider call per account even with two profiles; a provider part gets
  no handle for an account of another provider.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/` (new `plugin_agent.rs`, `plugin_provider.rs`,
changes in `spend_files.rs`, `record_format.rs`, `account_numbers/`),
`crates/agents/Cargo.toml`,
`crates/app/features/agents/io/` (load plugin parts into the registry).

## Comments
