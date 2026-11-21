# 21 The `agents` domain: agent and provider parts live under one name

Status: ready-for-agent
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
