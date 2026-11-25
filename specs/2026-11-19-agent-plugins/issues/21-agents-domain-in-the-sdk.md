# 21 The `agents` domain: agent and provider parts live under one name

Status: done
Blocked by: 20

## Why

Owner, 2026-11-21: the SDK will hold extension points of several domains,
and each domain may have its own "provider" (a container engine or a
cloud is a provider too). Today everything sits flat: the SDK exports
`otty_plugin_sdk::{Agent, Provider}`, the WIT packages are `otty:agent`
and `otty:provider`, the manifest has `[[agent]]` and `[[provider]]`, and
the shared host package `otty:host/types` carries types only coding
agents need (`identity`, `login-kind`, `limits`, `limit-window`,
`credits` and the like). A second domain would clash with these names.
The owner chose the name `agents`, after Agents mode, which the user
sees in OTTY.

No compatibility with earlier versions: nothing was released, so names
change in place and no alias of an old name stays.

## What to build

1. SDK: a public module `otty_plugin_sdk::agents` with `Agent`,
   `Provider` and every type and helper only that domain uses. The host
   operations stay at the crate root, since no domain owns them
   (`files`, `http`, `keys`, `sqlite`, `programs`, `terminal`,
   `keyring`, `env`, `clock`, `log`). The macro becomes
   `export!(T with agents::agent, agents::provider)`. Nothing of the
   domain is re-exported at the root.
2. WIT: packages `otty:agents-agent@0.1.0` and
   `otty:agents-provider@0.1.0` (each keeps its own version; nested WIT
   names such as `otty:agents:agent` are experimental, do not use them).
   Move the agent-only types out of `otty:host/types` into a types
   interface of the domain (for example
   `otty:agents-agent/types`, shared by the provider package, or a third
   package `otty:agents-types@0.1.0` if one package can not import the
   other cleanly; pick the simpler and say why). `otty:host` keeps only
   what any domain could use (errors of host operations, key handles and
   key places, files, http, programs and the rest). Check each type:
   `read-error`, the `need` and `need-action` of issue 20 and the key
   kinds go where their meaning lives.
3. Manifest (`otty-plugin-manifest`): parts are `[[agents.agent]]` and
   `[[agents.provider]]` with their permissions under them; `[api]`
   becomes `agents.agent = "0.1"`, `agents.provider = "0.1"`. The kind
   of a part is (domain, extension point); the Plugins page shows the
   domain as a plain word where it lists the kinds of parts.
4. Host (`otty-plugin-host`): the bindings and `contract.rs` follow the
   new packages (`agents::agent::v0_1`, `agents::provider::v0_1`). The
   map from `[api]` to bindings keys on the domain and the point.
5. `otty-agents` and the app follow the new names; nothing there
   changes in behaviour.
6. Plugins: the template, the five official plugins and the host's test
   plugins use the new imports, macro and manifest; rebuild the embedded
   copies with `plugins/embed.sh` and the test plugins with
   `scripts/build-test-plugins.sh`.
7. Calling a plugin is no domain's business (owner, 2026-11-21). Move
   out of `otty-agents` into `otty-plugin-host` what any domain needs to
   call a part: `CalledPart` (one read is one batch with the part's
   permissions only, failures and missed deadlines counted),
   `PartHealth` and `LastFailure` (three failed reads in a row pause
   the part until Try again; the last failure for the part's card),
   `PluginFailure` and its kinds and report, and `PluginPart` (how a
   part is named in reports: plugin, version, repository, part). The
   host's failure kinds are general (stopped, too slow, paused, a host
   operation refused, the part said it could not read); the part's
   `read-error` of the agents contract stays in `otty-agents`, which
   turns it into a short tag for the report. `otty-agents` keeps only
   turning the answers of `agents.agent` and `agents.provider` parts
   into its domain types. The Plugins page (`otty-app-plugins` core and
   io) takes these from `otty-plugin-host` and no longer depends on
   `otty-agents`; add that as a crate dependency limit of the app
   architecture checks, and first plant a violation to see the check
   fail (`AGENTS.md`).
8. Docs: `Write a Plugin.mdx`, `Plugins.mdx`, the SDK and template
   READMEs, the `otty-plugin-host` README (contract versions), and
   `spec.md` (the contract listing, the manifest example, Decisions that
   name the packages), so all show the domain.

## Done when

- Tests first: a manifest with `[[agents.agent]]` and
  `[[agents.provider]]` loads with each part's permissions; a manifest
  with a top level `[[agent]]` is refused as an unknown key; `[api]`
  with `agents.agent` picks the right bindings; the template plugin
  builds against `otty_plugin_sdk::agents`.
- No agent-only type left in `otty:host`; no root re-export of the
  domain in the SDK.
- Tests first for the moved code: a part pauses after three failed
  reads and starts again on Try again; its last failure is kept; the
  report names the plugin, version, repository, part, call and kind and
  never a value. The architecture check fails when `otty-app-plugins`
  depends on `otty-agents`.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-21 (agent): done. What moved where, and the choices made:
  - WIT: `otty:agents-agent@0.1.0` (`wit/agents-agent/0.1.0/`, files
    `types.wit` and `agent.wit`) and `otty:agents-provider@0.1.0`
    (`wit/agents-provider/0.1.0/`). The agent-only values moved out of
    `otty:host/types` into the interface `otty:agents-agent/types`:
    `read-error`, `identity`, `login-kind`, `limit-window`, `credits`,
    `limits`. The provider package imports that interface. I chose this
    over a third package `otty:agents-types` because it is simpler: one
    package and one version fewer to keep, and `use` of another package's
    interface works cleanly in both `wit-bindgen` and wasmtime (the
    provider bindings reuse the agent bindings' module with `with`). The
    cost: a new major version of the agent contract that changes these
    values also makes a new provider version name it.
  - What stays in `otty:host`, because any domain could use it:
    `host-error`, `command`, `need`, `need-action` and `key-ask`. A
    container engine or a cloud also needs the user to log in, give a
    key or open a page, and OTTY shows every need the same way.
    `given-key` (a key the user gave under the slot a need named) moved
    from the provider package into `otty:host/keys`, next to `key`: it is
    the answer to `ask-key`, which is in `otty:host`. `read-error` went
    to the domain: `login-refused` is about logins coding agents keep,
    and `not-supported` names the optional functions of these contracts.
  - SDK: `otty_plugin_sdk::agents` holds `Agent`, `Provider`, every type
    of both contracts and of `otty:agents-agent/types`, and the
    `RecordCounts` helpers. The root keeps the host operation modules,
    `Key`, `JsonWithKeys`, `GivenKey` and `given_key` (now in `keys`),
    `Command`, `HostError`, `KeyAsk`, `Need`, `NeedAction`, the key places
    and `Language`. Nothing of the domain is at the root. The macro is
    `export!(T with agents::agent, agents::provider, language)`.
  - Manifest: `[[agents.agent]]`, `[[agents.provider]]` with their
    permissions under them, and `[api] agents.agent = "0.1"`,
    `agents.provider = "0.1"`, `language = "0.1"`. A top-level `[[agent]]`
    is an unknown field. `Domain` (now only `Agents`) is new;
    `PartKind::domain()` and `point()` give the kind as (domain, point),
    and `PartKind::key()` / `Contract::key()` are `agents.agent`. The Rust
    names `PartKind::Agent` / `Provider` and `Contract::Agent` / `Provider`
    stay: renaming them now would touch about 150 lines for no change in
    meaning; a second domain is the time to name its own variants.
    `plugins.json` keeps the kind as `agents.agent` (no migration: not
    released).
  - Host: `otty_plugin_host::agents::{agent, provider}` with their
    bindings `agents::agent::v0_1` and `agents::provider::v0_1`; the
    engine links `otty:agents-agent/types`, which a component imports with
    either contract. `contract.rs` keys `[api]` on the domain and the
    point.
  - Calling a part moved from `otty-agents` into `otty-plugin-host`:
    `CalledPart`, `PartHealth`, `LastFailure`, `PAUSE_AFTER`,
    `PluginFailure`, `PluginFailureKind` and `PluginPart`, with a
    `fixtures` feature for the health of a paused or failed part. The
    kinds stay four (stopped, too slow, paused, could not read with a
    short tag); `PluginFailureKind::of_host_error` gives the tag of a host
    operation that failed, and `otty-agents` (`read_failure.rs`) turns the
    domain's `read-error` into the tag, so the reports read as before.
    `otty-agents` does not re-export them; its users import them from the
    host, so `otty-app-agents` now depends on `otty-plugin-host` too.
  - The dependency limit is a `[bans] deny` rule with `wrappers` in
    `deny.toml`: only `otty-app-agents`, `otty-app-agents-io` and `otty`
    may depend on `otty-agents`. `cargo deny check bans` failed with the
    old `otty-app-plugins` dependency before it was removed. Compiler and
    clippy can not tell which crate a `Cargo.toml` may list, so this is
    the check that fails.
  - The Plugins page writes a part as "Codex (Agents: agent)": the domain
    is the name of Agents mode (`mode_agents`), in all ten languages.
  - Rebuilt: the five official plugins and the template with
    `plugins/embed.sh`, and the host's test plugins with
    `scripts/build-test-plugins.sh`. `docs/research/` keeps the old names:
    it records research done before, as the `specs/` history does.

