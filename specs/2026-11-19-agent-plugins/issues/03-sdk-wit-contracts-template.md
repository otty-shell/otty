# 03 SDK, WIT contracts 0.1 and the plugin template

Status: ready-for-agent
Blocked by: 02

## Why

Spec Decision 3, sections "SDK" and "WIT sketch". Plugin authors need one
crate with the traits and helpers; the host needs the same WIT files for
its bindings, and must keep old contract versions.

Decided by the owner on 2026-11-20: `wit-bindgen` 0.57 is approved for
`otty-plugin-sdk` and plugins; the template lives in `plugins/template/`
of this repository for now; plugin authors depend on the SDK by a git tag
of this repository (crates.io with its first stable version), and plugins
inside this repository depend on it by path.

## What to build

1. `otty-plugin-sdk` in `crates/plugins/sdk` with a `README.md`. WIT
   files, one folder per package and version: `wit/host/0.1.0/`
   (`otty:host@0.1.0`: files, sqlite, keys, http, programs, terminal,
   keyring, env, clock, log), `wit/agent/0.1.0/` (`otty:agent@0.1.0`)
   and `wit/provider/0.1.0/` (`otty:provider@0.1.0`). No world joins the
   two contracts. Full types: profile, profile place, login, key place
   (with the optional JSON pointer of a SQLite field), session, spend
   place and item, answer, record counts, limits, credits, money, bill,
   command, stats, read error.
2. The traits `Agent` and `Provider` with default methods for optional
   parts (spec sketch), the plain helper functions over the host
   operations, `Key` with `fingerprint`, `has_prefix`, `jwt_claims`, and
   `RecordCounts` helpers. The one macro `export!` names the plugin type
   and the traits it exports: `export!(Example with agent)`,
   `export!(Example with provider)`, `export!(Example with agent,
   provider)`. Inside, the SDK keeps one `wit-bindgen` binding per set of
   traits; such a set is not a contract and has no version.
3. A test host for native `cargo test` of plugins: host operations served
   from a fixtures folder (files, lines with markers, SQLite files, JSON
   key files with made-up keys) and canned HTTP answers, recording the
   requests made, and checking each call against the permissions of the
   part being tested.
4. Host side in `otty-plugin-host`: `bindgen!` for each contract version,
   kept in its own module per contract and version (`agent::v0_1`,
   `provider::v0_1`), and one place that maps a manifest's `[api]` to
   them (the list of known versions of issue 02). A note in the README on
   how a new version is added next to the old (converters as `From`).
5. `plugins/template/`: its own cargo workspace; a minimal plugin with
   one agent part over a made-up `~/.example-agent/` folder and one
   provider part that reads a made-up balance, each with its own
   permissions; `[api] agent = "0.1"`, `provider = "0.1"`; `fixtures/`; a
   release profile `opt-level = 3`, `lto = true`, `codegen-units = 1`,
   `strip = true`; a GitHub Actions workflow that, on a semver tag,
   builds for `wasm32-wasip2`, checks the manifest version equals the
   tag, and attaches `otty-plugin.toml`, `<id>.wasm` and the icons to the
   release. A README for authors: depend on the SDK by git tag, build,
   test, release, and how parts and `[api]` fit together.

## Done when

- Tests first: the template's tests pass natively against the test host;
  the template builds for `wasm32-wasip2`, loads in `otty-plugin-host`,
  and its `version`, `profiles` and provider `numbers` return the
  fixture values through the real engine (with stub host operations
  until issues 04 to 06); a component that exports only `agent` loads
  with `[api] agent = "0.1"` and is refused with `provider` added; the
  test host refuses the provider part a file of the agent part.
- The checks of `AGENTS.md` pass; the template's own `cargo test` and
  `cargo build --release --target wasm32-wasip2` pass.

## Files

`crates/plugins/sdk/` (new), `crates/plugins/host/src/` (bindings),
`plugins/template/` (new).

## Comments
