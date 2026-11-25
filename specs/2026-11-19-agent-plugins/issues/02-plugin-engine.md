# 02 Plugin engine: `otty-plugin-host`

Status: ready-for-agent
Blocked by: none

## Why

Spec Decisions 2, 4, 7 and 8, "What a plugin is", "Engine limits". The
spike's `crates/plugins/host` (commit `adca59f3`, read with `git show
adca59f3:crates/plugins/host/src/engine.rs` and `plugin.rs`) proved the
setup; this issue builds it for real, with the manifest, its parts and
the permissions of each part, and with no host operation yet.

## What to build

1. New crate `otty-plugin-host` in `crates/plugins/host` with a short
   `README.md` (why it exists; it must not depend on `iced`, `crates/app/`
   or `otty-agents`). Add it to the workspace.
2. Dependencies (approved): `wasmtime` 48 with `default-features = false`
   and `runtime`, `cranelift`, `component-model`, `std`; `wasmtime-wasi`
   48 with `p2` only; `toml`; `globset`. Add `Apache-2.0 WITH
   LLVM-exception` to `deny.toml` with a comment naming wasmtime and
   Cranelift.
3. One engine for all plugins: sync API, epoch interruption, a ticker
   thread every 10 ms, stopped when the engine is dropped.
4. Compiled cache: `precompile_component` → file named by SHA-256 of the
   `.wasm` and `precompile_compatibility_hash`; read back with
   `Component::deserialize_file`; a broken or foreign cache file is
   recompiled. Only this crate writes the folder (the `unsafe` is
   documented).
5. Import allowlist: `wasi:cli/*`, `wasi:io/*`, `otty:*`; anything else,
   by name, is refused with the list of refused imports. Read which
   contracts (`otty:agent/agent`, `otty:provider/provider`) and versions
   the component exports from its export names; the manifest's `[api]`
   must name exactly those.
6. Store per call batch: empty `WasiCtx`, stderr into a capped log
   (64 KiB, oldest dropped), `StoreLimits` 64 MiB with trap on grow
   failure, 4 instances; deadlines by call kind (spec table); a host
   operation that waits extends the deadline by its own wait. Every call
   names the part it is for; the store carries that part's permissions
   only, so host operations (issues 04 to 06) check the called part's
   permissions and never another part's.
7. The manifest (`otty-plugin.toml`, spec "What a plugin is"): parse with
   `toml`, check ids (lowercase `a-z0-9-`), semver version, `[api]` (one
   `MAJOR.MINOR` per contract; every part has the contract of its kind;
   no contract without a part), at least one part, agent and provider
   parts with their own ids, hosts and icons, and each part's own
   permissions table. Globs: `~` and `{VAR}` of variables the same part
   lists in `env` only, no `..`; compiled with `globset`. A `Permissions`
   value per part answers "may read/write/query this path, run this
   program with these args, call this host, open this key place" and
   lists itself in plain words for the UI.
8. Contract versions: one place lists the versions this OTTY knows; a
   manifest naming one it does not know gives "Needs a newer OTTY" for
   the whole plugin (the bindings themselves arrive in issue 03).
9. Test plugins: small components built from sources in
   `crates/plugins/host/tests/plugins/` by a script
   (`scripts/build-test-plugins.sh`, needs `rustup target add
   wasm32-wasip2`), committed as `.wasm` fixtures: one that spins, one
   that grabs memory, one that imports `wasi:filesystem`, one minimal
   valid plugin.

## Done when

- Tests first: the spinning plugin stops near its deadline; the memory
  grabber traps at 64 MiB; the filesystem importer is refused with its
  import named; a second load comes from the cache; a damaged cache file
  is recompiled; manifests with a bad id, a `..` glob, a variable not in
  the part's `env`, a missing icon, a non-semver version, a part with no
  contract in `[api]`, a contract with no part, or an unknown contract
  version are refused with a plain reason ("Needs a newer OTTY" for the
  last); `[api]` that does not match the exports is refused; permission
  checks for paths, hosts and programs per part, and a path allowed only
  to another part of the same plugin is refused.
- `cargo deny check` passes with the new licence.
- The checks of `AGENTS.md` pass.

## Files

`crates/plugins/host/` (new), `Cargo.toml` (workspace members),
`deny.toml`, `scripts/build-test-plugins.sh`.

## Comments
