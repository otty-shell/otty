# 15 The manifest has no `[api]`: OTTY reads the contract versions from the `.wasm`

Status: done
Blocked by: 14

## Why

Owner, 2026-11-24, about

```toml
[api]
agents.agent = "0.1"
agents.provider = "0.1"
language = "0.1"
```

in every manifest: "we have an SDK, is this needed?" It is not. The SDK
builds the `.wasm` against a WIT version, so the versions of the
contracts are already in its exports. Today the host reads them
(`ExportedApi::of_exports` in `crates/plugins/host/src/engine.rs`) only
to check they match `[api]` (`LoadError::ApiMismatch`). The author writes
by hand what OTTY then checks. One source of truth, the `.wasm`, is
simpler. No backward compatibility: Agents mode is unreleased.

## What to build

1. **No `[api]`.** `otty-plugin-manifest` no longer reads `[api]`; a
   manifest that still has it is refused as an unknown field. `Api`,
   `Api::of_manifest` and the manifest's part/contract checks move to
   where the `.wasm` is known.
2. **The `.wasm` says the versions.** When a plugin is loaded (and when a
   release or folder is installed or an update is checked), OTTY reads
   the exported contracts and their versions from the component
   (`ExportedApi::of_exports`) and uses them where `manifest.api()` is
   used today: picking the bindings (`ContractExports`), whether the
   language receiver is there, and the checks that every declared agent
   part has the agent contract exported and every provider part the
   provider contract, and that no contract is exported with no part.
3. **"Needs a newer OTTY"** comes from an exported contract version this
   OTTY does not know, found when the `.wasm` is read at install or at
   an update check (download it, it is small). The words and where they
   show stay as today.
4. **Plugins.** Remove `[api]` from every official manifest and the
   template, bump each one's patch version, rebuild with
   `plugins/build.sh`, refresh the host test plugins and `template.wasm`.
   Spec, Write a Plugin and the template README drop `[api]` and say the
   SDK version decides the contract versions.

## Done when

- Tests first: a manifest with `[api]` is refused; a plugin's contract
  versions come from its exports; a plugin whose `.wasm` exports an
  unknown contract version is "needs a newer OTTY"; a declared part
  whose contract is not exported is refused, and an exported contract
  with no part is refused; the language receiver is used only when
  exported.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-24: Done. `otty-plugin-manifest` reads no `[api]` any more: a
manifest that has it is refused as an unknown field, and `Api`, its
errors and the part/contract checks are gone from it. `Contract`,
`ContractVersion` (now made from an export's version with
`ContractVersion::of_export`) and the list of known versions stay there.
`ExportedApi` in `otty-plugin-host` holds the contract versions a
component exports: `of_exports` reads them from export names (the engine
gives them at load), `of_wasm` from the bytes of a `.wasm` without
compiling it, and `check(manifest)` refuses a version this OTTY does not
know ("Needs a newer OTTY (agents.agent contract 0.2)", told first), a
part whose contract is not exported and an exported contract with no
part, as `ContractError`. The bindings and the language receiver come
from the exports, and `LoadError::ApiMismatch` became
`LoadError::Contracts`. `of_wasm` uses the parser wasmtime is built with,
through wasmtime's `reexport-wasmparser` feature, so no crate is added.
The releases crate downloads the `.wasm` of each release it looks at,
checks it, keeps it for the install, and skips a release that needs a
newer OTTY, as before; a local folder is checked the same way. Other
misfits show as "The plugin does not run in OTTY: ...". The official
plugins and the template drop `[api]`: anthropic 0.2.14, deepseek 0.1.6,
openai 0.2.14, opencode 0.1.10, openrouter 0.1.6, template 0.1.8, rebuilt
into `assets/plugins/`, with the host test plugins and `template.wasm`.
The spec of 2026-11-19-agent-plugins, Write a Plugin, the template README
and the crate READMEs say the SDK version decides the contract versions.
