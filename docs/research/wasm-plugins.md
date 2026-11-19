# WASM plugins for agents and providers

Checked on 2026-11-19. Every claim links to the doc page or source file it comes from (section 9).
Source links point at the exact commit read that day. Numbers marked "not measured" are not
backed by a source and must be checked in a spike before any decision depends on them.

## 1. Summary and recommendation

The owner has already decided on WASM plugins modelled on Zed extensions, with `wasmtime`. This
research covers how to do that safely and simply.

- **What Zed does well, and OTTY should copy:** a plugin is a folder with a manifest; most
  features are declarative and need no code; code is a WebAssembly *component* built against a
  versioned WIT world (WIT is the interface language of the component model: it describes typed
  functions and records that cross the plugin boundary). Zed keeps every old world version in the
  host and converts old types to new ones, so old extensions keep working
  ([zed-wit], [zed-v060]). The registry is one Git repo of submodules; Zed's CI builds every
  extension from source and serves the archive, so users never compile anything
  ([zed-pub-guide], [zed-blog]).
- **What Zed does that OTTY must not do:** Zed grants every capability by default
  (`process:exec`, `download_file` with `*`) ([zed-defaults]); its HTTP `fetch` has no host check
  at all ([zed-v080]); it gives extensions stdio and a read-write folder through WASI
  ([zed-wasmhost]); and the downloaded archive is checked only against its content length, with no
  signature or hash ([zed-host]). One big world holds every feature, so every extension
  implements every export ([zed-world]).
- **Recommendation for OTTY:**
  1. **Deny by default.** The manifest lists every file glob, HTTP host, program, keyring entry
     and database the plugin may touch. The user sees that list before install and again when an
     update asks for more.
  2. **Plugins never see a key.** Keys and tokens are opaque handles (WIT `resource`s). The
     plugin can fingerprint a handle, read the claims of a JWT, and put it into an HTTP request;
     the host fills it in only for a host of the account's own provider.
  3. **The host owns the heavy and risky parts:** walking files, reading big JSONL files in
     batches, read-only SQLite, HTTP, the keyring, running programs and the hidden terminal, the
     per-file cache, adding up tokens, and the "format changed" verdict. The plugin owns what
     differs per agent: where things are, how to read one record, how to read one answer.
  4. **One WIT world per extension point** (`otty:agent`, `otty:provider`), each with its own
     semver. Additions use WIT `@since` gates; a breaking change gets a new major version, kept
     side by side in the host with converters, as Zed does.
  5. **wasmtime with the sync API**, epoch interruption with a trap per call (CPU bound), a
     `StoreLimits` memory cap, the default on-demand allocator, compiled components cached on disk
     with `Component::serialize`, and WASI 0.2 linked with an empty context so Rust's standard
     library works but sees no files, no network and no environment.
  6. **A catalog repo** in the Zed style, but with a SHA-256 per archive and a signed index.
     Claude Code and Codex ship inside OTTY as ordinary plugins and can be updated from the
     catalog without a new OTTY release, which is the main win when an agent changes its files.
  7. **Declarative first.** A provider whose balance is one `GET` with a key and one number in
     JSON (DeepSeek, Moonshot, OpenRouter, Kilo) needs no WASM. The same goes for an agent whose
     records are simple JSONL; this is also what a "click the fields" editor in OTTY writes.
  8. **Rust is the supported plugin language** at first. JavaScript and Python work through the
     component tools but each plugin then carries a whole engine (about 8 MB for JS, about 35 MB
     for Python) ([cjs], [cpy-98]).

The open questions for the owner are in section 8.

## 2. How others do it

### Zed extensions

**The package.** "Zed extensions are Git repositories containing an `extension.toml` manifest"
([zed-dev]). The manifest has `id`, `name`, `version`, `schema_version`, `authors`,
`description`, `repository`, and then tables for each feature: `themes`, `languages`, `grammars`,
`language_servers`, `context_servers`, `slash_commands`, `snippets`, `debug_adapters`,
`capabilities` and others ([zed-manifest], lines 88-127). Themes, languages and snippets are pure
data. "Only language server, context server and debugger extensions require the presence of
custom Rust" ([zed-dev]). The Rust part is a `cdylib` crate on the `zed_extension_api` crate,
built for `wasm32-wasip2` ([zed-dev], [zed-builder]).

**Versioned worlds.** The WIT files live in `crates/extension_api/wit/since_v0.0.1` …
`since_v0.8.0`, ten versions in all ([zed-wit]). The SDK writes the API version it was built
against into a 6-byte custom section `zed:api-version` of the `.wasm` ([zed-api], lines 350-353);
the host reads it before compiling ([zed-wasmhost], lines 819-856) and instantiates the matching
generated bindings: `if version >= latest::MIN_VERSION … else if version >= since_v0_6_0::MIN_VERSION …`
down to 0.0.1 ([zed-wit], lines 108-200). Each old version's host module reuses the newest types
where they did not change (`with: { "zed:extension/common": latest::… }`) and converts the rest
with `From` impls ([zed-v060]). A new world version is first open only on Dev and Nightly builds
([zed-wit], lines 54-90). Every call into an extension is a `match` over all ten versions
([zed-wit], from line 235); this is the price of keeping old extensions alive.

**Capabilities.** Three kinds: `process:exec` (command and argument globs), `download_file` (host
and path globs) and `npm:install` (package) ([zed-cap]). A command must be listed both in the
extension's manifest and in the user's `granted_extension_capabilities` setting
([zed-granter], lines 23-47; [zed-manifest], lines 168-187). Downloads and npm are checked only
against the user setting ([zed-granter], lines 49-83). The default setting grants all three with
`*` ([zed-defaults], lines 2410-2417): the user restricts, rather than approves. HTTP `fetch` and
`fetch_stream` are not checked at all ([zed-v080], lines 633-663). The WASI context gives the
extension inherited stdio and its own work folder read-write ([zed-wasmhost], lines 748-763).

**Runtime settings.** One `Engine` for all extensions: component model on, incremental
compilation cache in memory (capped at 32 MB; "our extensions turn into roughly 512kb in the
cache"), and epoch interruption with a timer that bumps the epoch every 100 ms; each store yields
after one tick ([zed-wasmhost], lines 556-603, 686-687, 1003-1021). No fuel, no memory limit.

**Registry, install, update.** Publishing is a pull request to `zed-industries/extensions` that
adds the extension's repository as a Git submodule and an entry with its `version` to
`extensions.toml`; one extension per PR, at most three open PRs per author ([zed-pub-guide]).
Updating is the same PR with a newer submodule commit ([zed-pub-update]). Since 2025-10-01 every
extension needs an accepted open source licence, because Zed distributes "the resulting binary
produced from your extension code" ([zed-pub-license]). CI compiles each extension and uploads
`archive.tar.gz` and `manifest.json` to S3; zed.dev serves them; no Rust is compiled on the user's
machine ([zed-blog]). The client asks the API for updates with its supported
`min/max_schema_version` and `min/max_wasm_api_version`, so it is only offered versions it can
run ([zed-host], lines 691-711). The download is checked only against `Content-Length`
([zed-host], lines 885-910). Zed can also suppress an extension by id when its feature moved
into the core ([zed-host], lines 105-128).

**Review.** Maintainers review by hand against written prerequisites: one extension per use case,
no misuse of the API, "do not read or modify anything outside the environment Zed designates"
([zed-pub-prereq]). First feedback takes "a few weeks … up to one or two months" ([zed-pub-faq]).
An unmaintained extension can be forked and replaced after six weeks of documented silence
([zed-pub-faq]). There is no code signing.

**Local development.** "Install Dev Extension" points Zed at a folder; Zed runs
`cargo build --target wasm32-wasip2` itself and the dev extension overrides the published one
([zed-dev], [zed-builder]).

### Zellij

Zellij plugins are WebAssembly modules (not components) with a permission list of 17 kinds, such
as `RunCommands`, `WebAccess`, `FullHdAccess`, `ReadPaneContents`; plugins ask for them at run time
with `request_permission` and the user answers a prompt ([zellij-perm]). Zellij moved from Wasmer
to Wasmtime in 2024 because of an unpatchable Cranelift CVE ([zellij-3349]), then from Wasmtime to
the **wasmi** interpreter in October 2025 ([zellij-4449], [zellij-cargo]). The reasons it gives:
compiling took seconds, interpreting starts in "single-digit milliseconds"; no disk cache of
compiled plugins and no cache-busting during development; smaller memory and binary; packaging
for Debian was hard with wasmtime. The cost: "a minor hit for compute-heavy operations … scanning
large amounts of text" ([zellij-4449]). That last case is exactly OTTY's JSONL reading; see 3.5.

### Extism

Extism is a plugin framework on top of wasmtime with WASI 0.1 only (`wasmtime-wasi` feature `p1`)
([extism-runtime]). Plugins are core modules that take and return bytes; there is no WIT. Its
manifest has the right ideas: `allowed_hosts` ("if this is empty then no hosts may be accessed"),
`allowed_paths`, `timeout_ms`, `memory.max_pages`, `max_http_response_bytes`
([extism-manifest]). OTTY gets the same limits from wasmtime directly and typed records from WIT,
so Extism adds a layer without adding much.

### Lapce and wezterm

Lapce runs plugins on wasmtime with WASI 0.1 and talks to them over stdio as JSON-RPC; it gives
each plugin **the whole environment** (`inherit_env()`) and its folder as `/`
([lapce-wasi], lines 466-494). Environment variables are where agents keep keys
(`ANTHROPIC_API_KEY`), so OTTY must not do this. wezterm plugins are Lua files cloned from a Git
URL, updated only by `wezterm.plugin.update_all()`, with no sandbox described ([wezterm]).

### What to copy and what to avoid

| Copy | Avoid |
| --- | --- |
| Manifest + declarative parts + optional WASM (Zed) | One world holding every feature (Zed) |
| Versioned WIT worlds, old ones kept with converters (Zed) | Default-allow capabilities; unchecked HTTP (Zed) |
| Catalog repo, CI builds from source, licence check (Zed) | Archive checked only by length; no signature (Zed) |
| Client asks only for versions it can run (Zed) | Whole environment passed to the plugin (Lapce) |
| Dev plugin overrides the installed one (Zed) | Run-time permission prompts in the middle of work (Zellij) |
| Explicit host list, timeout, memory cap (Extism) | Byte-only interfaces without types (Extism) |
| Epoch interruption (Zed) | Unbounded memory (Zed) |

## 3. wasmtime for OTTY

### 3.1 Cargo features

`wasmtime` 50.0.3 turns on 26 default features, including `cranelift`, `component-model`,
`component-model-async`, `async`, `cache`, `gc`, `threads`, `pooling-allocator`, `wat`,
`profiling`, `coredump`, `debug` and `wit-parser` ([wt-features]). OTTY needs few of them:

```toml
# Sketch only: nothing is added until the owner approves.
wasmtime = { version = "…", default-features = false, features = [
    "runtime",            # run code
    "cranelift",          # compile .wasm on the user's machine
    "component-model",    # WIT worlds, records, resources, bindgen!
    "std",
    "parallel-compilation", # optional: rayon, already in OTTY's Cargo.lock
    "addr2line",          # optional: readable plugin crash traces for authors
] }
wasmtime-wasi = { version = "…", default-features = false, features = ["p2"] }
```

- `wasmtime-wasi`'s `p2` feature turns on `wasmtime/component-model` and `wasmtime/async`
  ([wt-wasi-features]); it also always depends on `tokio` with the multi-thread runtime and
  `net` features; `tokio` is already in OTTY's `Cargo.lock`, but maybe not with those features.
  WASI is needed because Rust's `wasm32-wasip2` standard library imports WASI interfaces.
- `WasiCtxBuilder::new()` by default has stdin closed, stdout and stderr discarded, no
  environment, no arguments, no preopened folders, and all socket addresses denied
  ([wt-wasi-ctx]). OTTY should use exactly that, with stderr routed to a capped plugin log, and
  check the component's imports at install time with `Component::component_type().imports()`
  ([wt-component]): only `wasi:cli`, `wasi:io`, `wasi:clocks`, `wasi:random` and `otty:*` are
  allowed, so a plugin that imports `wasi:sockets` or `wasi:filesystem/preopens` beyond what Rust
  std needs is refused with a clear reason (which interfaces Rust std imports must be checked in
  the spike).
- `p2::add_to_linker_sync` exists next to the async one ([wt-wasi-linker]). OTTY's agents crate is
  synchronous today (`ureq`, `rusqlite`), so the sync API fits; Zed needs async only because it
  runs extensions inside its async executor ([zed-wasmhost], lines 565-571).
- Leave out `cache` (pulls `wasmtime-cache`; OTTY can store `Component::serialize` output itself),
  `pooling-allocator`, `gc`, `threads`, `wat`, `profiling`, `coredump`, `debug`, `winch`.

### 3.2 Bounding CPU: epochs, not fuel

wasmtime's own guidance: "epoch-based interruption results in faster execution … up to 2-3x";
fuel is for when "*deterministic* yielding or trapping is needed" ([wt-epoch]). OTTY does not need
determinism. Plan: `Config::epoch_interruption(true)`, one timer thread calling
`Engine::increment_epoch()` (Zed uses 100 ms), and `Store::set_epoch_deadline(n)` with the
default trap before each call, so a stuck plugin call fails after about `n` ticks. Two limits of
epochs to design around ([wt-epoch]):

- Epochs do not interrupt a host call. Every host function (HTTP, SQL, program, terminal) needs
  its own timeout. Today's HTTPS already gives up after 30 seconds (`account_numbers/mod.rs`).
- A large `memory.copy` checks the epoch only once, so memory must also be bounded (3.3).

Both epochs and fuel are "not compatible with the Winch compiler" ([wt-config], lines 442, 780),
so Winch (the fast baseline compiler) is out.

### 3.3 Bounding memory

`StoreLimitsBuilder::memory_size(bytes)` caps each linear memory; by default memory "will not be
limited"; `instances(n)` caps instances (default 10,000); `trap_on_grow_failure` turns a failed
grow into a trap ([wt-limits]). Set it through `Store::limiter`. Proposal: 256 MiB per plugin
store, a handful of instances, trap on grow failure (not measured; a Claude Code line with a large
tool result can be several MB, so the cap must leave room).

### 3.4 Allocator and compile cache

- **On-demand allocator (default).** The pooling allocator reserves about 4 GiB of virtual memory
  per memory slot up front, "may keep memory alive when nothing is using it", and needs fixed
  counts in advance ([wt-pooling]). It pays off for servers creating thousands of instances a
  second, not for a desktop app with a dozen plugins.
- **Compile once, keep the result.** Without a cache "every call to `Module::new(my_wasm)` will
  recompile" ([wt-config], lines 1717-1731). Zellij reported compile times in seconds
  ([zellij-4449]). Plan: on install, `Engine::precompile_component` → write
  `~/.cache/otty/plugins/<sha256 of wasm>-<Engine::precompile_compatibility_hash>.cwasm`; on start,
  `Component::deserialize_file`. Deserializing is `unsafe` because the bytes must be wasmtime's
  own output; wasmtime safely rejects output of any other wasmtime version
  ([wt-deserialize], [wt-engine]). So only OTTY writes that folder, and an OTTY upgrade simply
  recompiles.

### 3.5 Size, build time, and wasmi

- **Binary size.** The only official numbers are for the C API: 19 MB for a default release
  build, 2.1 MB without default features (no compiler, so it can only load precompiled
  `.cwasm`), down to 698 KB with every trick ([wt-minimal]). OTTY's set (Cranelift + component
  model + WASI 0.2) sits in between. **Not measured.** WASI alone "accounts for 1M+" in the
  wasmtime CLI ([wt-minimal]).
- **Build time.** No primary source gives numbers. Cranelift and the `wasmtime-*` crates are a
  large compile; measure it in a spike on this machine (one job, `nice`), not during other work.
- **Release support.** A new major version every month; versions that are a multiple of 12 are
  LTS and get security fixes for 24 months, others for 2 months ([wt-release]). Wasmtime 48 is an
  LTS line (48.0.10 published 2026-11-19, [crates-wasmtime]); pinning to it means one planned
  upgrade a year instead of one a month.
- **wasmi** (2.0.2) is an interpreter with fuel, two security audits and a small footprint, but
  supports only core WebAssembly and WASI 0.1 ([wasmi]). No component model means no WIT records,
  no resources, and hand-written byte encoding at the boundary, which is what Zed's blog says made
  extensions painful before the component model: "all data that crosses the Wasm module boundary
  has to be represented as integers or floats" ([zed-blog]). Zellij's slowdown on "scanning large
  amounts of text" ([zellij-4449]) is OTTY's main workload. Keep wasmtime, as decided.

### 3.6 Why the component model matters here

- Typed records (`session`, `answer`, `token-count`, `account`, `limits`) cross the boundary
  without JSON or a custom encoder on either side; `wasmtime::component::bindgen!` and
  `wit_bindgen::generate!` generate both sides from one `.wit` file ([zed-blog]).
- **Resources** are opaque handles owned by the host: the right tool for "a key the plugin may
  use but not read" and for "an open file the plugin reads in batches".
- Interface names carry semver. wasmtime's linker resolves semver-compatible names
  automatically: a component importing `a:b/c@0.2.0` links to a host that defines `a:b/c@0.2.1`
  ([wt-linker]). WIT has `@since(version = …)` and `@unstable(feature = …)` gates for additions
  ([wit-gates]). So OTTY can read a plugin's API version from its import and export names and
  needs no custom section like Zed's `zed:api-version`, which predates this.
- WASI 0.3 (native `stream<T>` and async) shipped on 2026-06-11 and is on by default in wasmtime
  46+ ([wasi-roadmap]); Rust's `wasm32-wasip3` target is tier 2 and, per the nightly docs, "first
  available on stable in Rust 1.100.0" ([rust-wasip3]). OTTY pins Rust 1.96. Start on 0.2 with
  batch resources (4.2); a later major version of the world can switch to streams.

## 4. Host API for agents

### 4.1 Who does what

Today's code already has the right split. `count_spend` in `spend_files.rs` walks the session
files, skips files unchanged since the first counted day, re-reads only files whose size or
change time differ (`FileMemory`), tallies, and asks `format_change` for the verdict. The only
agent-specific part is the `read_file: impl Fn(&Path) -> FileSpend` it is given. That closure is
what a plugin should be. The same holds for `walk.rs` (sources tried best first, every failure
kept), `spend_tally.rs`, `money_estimate.rs`, `price_source.rs`, `limits.rs`,
`account.rs::merge_accounts` and `record_format.rs`: none name an agent, all stay in the host.

| Host (OTTY core) | Plugin |
| --- | --- |
| Profiles, `agents.json`, hidden items, names | Where the agent keeps things (declarative globs) |
| File walk, stamps, per-file cache, incremental reads | How to read one record / one file chunk |
| Tally by day, hour, model, project; dedup by answer identity | The identity of an answer |
| "Format changed" verdict and report | Per-file counts: seen, understood, version, missing fields |
| Merging accounts, fingerprints, provider of a base URL | Which accounts a profile has, as identities and key handles |
| HTTP, SQLite, keyring, programs, hidden terminal | What to ask and how to read the answer |
| Scheduling, timeouts, refresh rate, "once per account" | — |

### 4.2 Big JSONL files

- **Batches, not whole files.** A `line-reader` resource: the plugin opens a file it may read and
  calls `next-batch(max-bytes)` to get a list of complete lines (about 1 MiB per call). Each batch
  is one copy into the plugin's memory; nothing else is copied twice. A partial last line, the one
  the agent is still writing, is not handed out.
- **Markers filter on the host.** Today `jsonl.rs::has_marker` skips most lines before parsing
  them. The plugin passes its markers (for Claude Code, the bytes of `"type":"assistant"`) when it
  opens the reader; the host returns only lines that contain one, plus a count of skipped lines.
  This keeps the copy small without moving any parsing to the host.
- **Compressed files.** Codex keeps some session files compressed (`codex/spend_file.rs`, via
  `ruzstd`). The host decompresses, so plugins never carry a zstd decoder.
- **Incremental reads.** Session files grow at the end. The host keeps, per (plugin id, plugin
  version, file), the last byte offset and the plugin's result so far. When a file only grew, it
  asks the plugin to read from the old offset and adds the new answers; answer identities already
  dedupe repeats (`spend_files.rs::CountedAnswer::identity`). When a file shrank or its start
  changed, it reads from zero. When the plugin version changes, the cache is dropped.
- **Where the cache lives:** in the host, as typed results (numbers, model names, identities;
  never record text). Today it lives in memory only (`file_memory.rs`). Keeping it on disk would
  avoid re-reading gigabytes at each start; that is an owner decision (section 8).

### 4.3 SQLite

OpenCode and Kilo are read with read-only `rusqlite` and plain `SELECT`s
(`open_code/database.rs`). As a host function: the plugin passes a database path it may read, SQL
text and parameters; the host opens the file with `SQLITE_OPEN_READ_ONLY` and
`PRAGMA query_only` as today, refuses a statement whose `Statement::readonly()` is false
([rusqlite-readonly]), refuses `ATTACH` through an authorizer ([rusqlite-auth]; needs the `hooks`
feature) so a plugin cannot open a database outside its permission, stops a slow query with an
`InterruptHandle` ([rusqlite-interrupt]), and returns rows in batches through a `rows` resource.
Columns come back as a WIT `variant` of integer, real, text, blob, null. Large `data` JSON
columns are copied as text; the plugin parses them.

### 4.4 Secrets

Every reader that touches a key today turns it into a fingerprint at once
(`claude_code/accounts.rs`, `open_code/accounts.rs`, `keyring_entry.rs`). In a plugin:

- `read-json-with-secrets(path, pointers)` returns the JSON with the values at the given JSON
  pointers (such as `/claudeAiOauth/accessToken`) replaced by placeholders, plus one `secret`
  handle for each. The plugin sees the shape and the non-secret fields, never the key.
- On a `secret`: `fingerprint()` (SHA-256 as today), `jwt-claims()` (the middle part of a JWT,
  as `codex/jwt.rs` does today; the signature and the token are never returned), and
  `has-prefix(text)` (to tell an admin key from a normal one without reading it).
- An `account` the plugin returns may hold a `secret`. The host keeps the handle only for the
  current refresh, as today ("the key is read again right before the request … and dropped right
  after it", `account_numbers/mod.rs`).
- `http-request` takes headers whose values may contain a secret handle. The host fills it in
  only when the request's host is one of the hosts of **the provider of that secret's account**.
  This puts "a key is sent only to its own provider's host" in the host, not in plugin code.

### 4.5 Programs and the hidden terminal

- `run(program, args, timeout)` for `--version`: allowed only for programs and argument patterns
  in the manifest, with a size cap on output, the profile's config-folder variable set and
  nothing else from OTTY's environment.
- `line-program` resource for `codex app-server` (`send-line`, `next-line(timeout)`, `stop`),
  built on today's `StartProgram`. The host cannot know which JSON-RPC methods are safe; the
  `thread/list` method once deleted rows of Codex's index (`codex/app_server.rs`). See section 8.
- `hidden-terminal` resource for Claude Code's `/usage` (`send-keys`, `next-screen(timeout)`,
  `stop`), built on today's `StartTerminal` with OTTY's own terminal engine. The host chooses the
  working folder. The plugin keeps the timing, the trust-question answer and the screen parser
  (`usage_probe.rs`, `usage_screen.rs`).

### 4.6 Permissions in the manifest

Paths use placeholders the host fills in: `{config}` (the profile's folder), `{data}` (an XDG data
folder), `~` (home). A glob may not leave home or contain `..`. Sketch:

```toml
[permissions]
read = ["{config}/projects/**/*.jsonl", "{config}/.credentials.json", "{config}/settings.json", "~/.claude.json"]
secrets = [{ file = "{config}/.credentials.json", pointers = ["/claudeAiOauth/accessToken"] },
           { file = "~/.claude.json", pointers = ["/primaryApiKey"] }]
env = ["ANTHROPIC_BASE_URL"]
env-secrets = ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"]
http = ["api.anthropic.com"]
run = [{ program = "claude", args = ["--version"] }]
terminal = [{ program = "claude", args = ["--strict-mcp-config", "**"] }]
sqlite = []
keyring = []
```

### 4.7 How results come back

Typed WIT records that mirror today's types, so the host code after the plugin call stays as it
is. A first cut of the `otty:agent` world:

```wit
package otty:agent@1.0.0;

interface types {
  record token-count { input: u64, output: u64, cache-read: u64, cache-write: u64, reasoning: u64 }
  record answer {
    identity: option<string>, at-ms: s64, model: option<string>,
    provider: option<string>, tokens: token-count, cost-microdollars: option<u64>,
  }
  record record-counts { seen: u64, understood: u64, version: option<string>, fields: list<string> }
  record file-spend {
    project: option<string>, provider: option<string>, account: option<account-identity>,
    answers: list<answer>, records: record-counts,
  }
  variant account-identity { account-id(string), organization-id(string), email(string), key(string) }
  record session { id: string, folder: string, title: string, changed-at-ms: s64 }
  // accounts, limits windows, credits, session stats: as in account.rs, limits.rs, stats.rs
}

world agent {
  import otty:host/files@1.0.0;     // list, read-small, read-json-with-secrets, line-reader
  import otty:host/secrets@1.0.0;   // secret resource: fingerprint, jwt-claims, has-prefix
  import otty:host/http@1.0.0;
  import otty:host/sqlite@1.0.0;
  import otty:host/programs@1.0.0;  // run, line-program, hidden-terminal
  export detect: func(profile: profile) -> profile-status;
  export accounts: func(profile: profile) -> list<account>;
  export version: func(profile: profile) -> option<string>;
  export sessions: func(profile: profile) -> result<list<session>, source-error>;
  export read-spend: func(profile: profile, file: borrow<line-reader>) -> file-spend;
  export limits: func(profile: profile, source: string) -> result<limits, source-error>;
  export session-stats: func(profile: profile, session: string) -> result<session-stats, source-error>;
}
```

The `otty:provider` world is smaller: `balance(account)`, `limits(account)` and
`bill(account, days)`, with `http` and `secrets` imports only.

## 5. Distribution

- **Catalog repo** (`otty-shell/otty-plugins`, name open): one entry per plugin with the source
  repository, a pinned commit, the version and the extension points with their world versions.
  CI builds each plugin from source with a pinned toolchain, checks the licence, checks the
  component's imports against the manifest, runs the plugin's fixture tests, and publishes
  `<id>-<version>.tar` with its SHA-256 in a signed `index.json`. OTTY already ships `ring` (via
  `rustls`), which verifies Ed25519 signatures, so checking the index needs no new crate.
- **Versions tied to OTTY.** OTTY asks the index only for plugin versions whose worlds it can
  run, as Zed does with `min/max_wasm_api_version` ([zed-host]). An OTTY too old for a new plugin
  version keeps the last one it can run.
- **Before install** the user sees: name, author, repository, the agents or providers it reads,
  and the permissions in words: "reads `~/.claude/projects/**/*.jsonl`", "sends your Anthropic
  key only to `api.anthropic.com`", "runs `claude --version`". Nothing is granted silently.
- **Auto-update.** On by default; an update that asks for any new permission waits for the user.
  A pinned or locally edited plugin is not updated.
- **Built-in plugins.** Claude Code and Codex are embedded in the OTTY binary as ordinary
  packages. A newer catalog version replaces the embedded one, so a fix for a changed session
  format reaches users without an OTTY release.
- **"Format changed" points at the plugin.** `FormatChange::report` already lists agent,
  version, file kind and field names without values (`record_format.rs`). Add the plugin id,
  version and repository. A "Report to plugin author" button opens a new issue in that repository
  with the report filled in; the user sees the text before sending.
- **Edit or fork locally.** "Edit locally" copies the package to
  `~/.local/share/otty/plugins/local/<id>/`; the local copy overrides the catalog one (Zed shows
  "Overridden by dev extension", [zed-dev]) and stops its updates until the user resets it. For a
  WASM plugin, "Fork" opens the source repository; OTTY does not compile Rust.

## 6. Authoring

- **Rust.** `rustup target add wasm32-wasip2`, then `cargo build --target wasm32-wasip2
  --release` with `wit-bindgen`; the component model book now recommends this plain path and says
  `cargo-component` "is in the process of being deprecated" ([cm-rust], [cargo-component]).
  `wasm32-wasip2` is a tier 2 target with full `std` ([rust-wasip2]).
- **SDK crate** (`otty-plugin-sdk`): the WIT files, the generated bindings, types with helpers
  (`TokenCount`, `RecordCounts`), and a native test harness: the same plugin code compiled for the
  host with the host functions as a `mockall` mock, so plugin tests run with plain `cargo test`
  and need no wasm at all.
- **Template**: `cargo generate` repository with a manifest, `src/lib.rs`, a `fixtures/` folder
  with a made-up config folder, and CI that builds the component.
- **Fixture tests in OTTY**: `otty plugin test <folder>` loads the component in the real host
  against `fixtures/` and compares results with saved JSON. Fixtures are made-up data, never a
  real session.
- **Hot reload in dev**: OTTY watches a dev plugin's folder (`notify` is already in
  `Cargo.lock`) and reloads the manifest and `.wasm` when they change; the author rebuilds with
  cargo in their own terminal.
- **Other languages**: `ComponentizeJS` embeds SpiderMonkey, "around 8MB" per plugin ([cjs]);
  `componentize-py` bundles CPython, a hello world was about 35 MB and "Neither Python … nor
  CPython … were designed to optimize for small, self-contained binaries" ([cpy-98]). They would
  work with the same WIT, but OTTY should document and test Rust only at first.
- **Declarative parts in the same package.** The manifest names the agent (program, default
  folder, folder variable, database variable, extra folders like `~/.claude*`), its launch,
  resume and login commands with `{session}` and `{config}` placeholders, and the newest tested
  version. All of today's `AgentDescriptor` and `Agent::launch/login` fit here. A `records.toml`
  can describe a simple JSONL agent: which files, which marker, and JSON pointers for time,
  model, provider and each token kind. The host reads those with its own code; no WASM. For a
  provider: URL, auth header form, JSON pointers for the amount and currency. OTTY's
  "click the fields" editor writes exactly these files: it shows the field names and numbers of a
  sample record (never the text values), the user picks which is which, and the result is a local
  package that can later be published like any other.

## 7. Migration of today's code

| Today | Goes to | Notes |
| --- | --- | --- |
| `AgentDescriptor`, `Agent::default_config_dir`, `launch`, `login` | Manifest (declarative) | `{config}`, `{session}` placeholders |
| `claude_code/spend_file.rs`, `session_file.rs`, `sessions.rs`, `stats.rs`, `project_folder.rs` | Claude Code plugin, nearly as is | `std::fs` reads become the line reader |
| `claude_code/accounts.rs`, `config_files.rs`, `credentials.rs`, `plan.rs` | Claude Code plugin | Keys become `secret` handles |
| `claude_code/usage.rs`, `usage_answer.rs` | Claude Code plugin | HTTP through the host, token as a handle |
| `claude_code/usage_probe.rs`, `usage_screen.rs` | Claude Code plugin | Terminal through the host |
| `claude_code/probe_sessions.rs` (removes files Claude left in OTTY's probe folder) | Open question | The only write into another agent's folder |
| `claude_code/probe_memory.rs` | Host | "At most once in five minutes" is scheduling |
| `claude_code/cost_report.rs` | Anthropic provider plugin | Admin key as a handle, pages walked in the plugin |
| `codex/*` readers (`spend_file`, `sessions`, `session_index`, `session_limits`, `stats`, `version`, `accounts`, `auth_file`, `base_url`, `rate_limits`, `individual_limit`, `workspace_credits`, `usage`, `app_server`) | Codex plugin, nearly as is | zstd moves to the host |
| `codex/jwt.rs`, `token_expiry.rs` | Host (`secret.jwt-claims`) | Plugin reads claims, never the token |
| `open_code/*` (`database`, `message`, `sessions`, `spend`, `stats`, `accounts`, `fork`, `location`, `provider_ids`) | One OpenCode-family plugin with two agents (OpenCode, Kilo) | SQL text stays in the plugin; opening stays in the host |
| `account_numbers/deepseek.rs`, `moonshot.rs`, `openrouter.rs`, `kilo.rs` | Declarative provider parts | One `GET` and JSON pointers each |
| `account_numbers/openai_costs.rs` | OpenAI provider plugin | Paged; WASM or declarative paging |
| `account_numbers/mod.rs` (which source for which account, key re-read) | Host | Generic once providers are data |
| `keyring_entry.rs` (Goose entry) | Goose plugin manifest + host `keyring` | The entry names move to the manifest |
| `system_keyring.rs`, `https.rs`, `jsonl.rs`, `json_file.rs`, `file_memory.rs`, `file_stamp.rs`, `spend_files.rs`, `spend_tally.rs`, `record_format.rs`, `walk.rs`, `price_source.rs`, `litellm.rs`, `money_estimate.rs`, `key_fingerprint.rs`, `hidden_terminal.rs`, `program.rs` | Host | Already agent-free |
| `provider.rs` (`Provider` enum with six names, `of_base_url`) | Host + provider manifests | Provider ids become strings; hosts and base URLs come from provider packages |
| `account.rs::usage_page` and other per-provider `match`es | Provider manifests | |

Sizes for the effort (lines including tests): `claude_code/` 7,354, `codex/` 6,947,
`open_code/` 2,999, `account_numbers/` 1,374. The app's agents feature crates also name
providers: 71 references to `Provider::Anthropic` … `Provider::Kilo` under `crates/app`
(not checked how many are test data). These must become data before the core holds no agent or
provider by name.

### Host functions and the permission each needs

| Host function | What it does | Permission |
| --- | --- | --- |
| `files.list(glob)` | Paths, sizes, change times | `read` globs |
| `files.read-small(path)` | Whole file up to 1 MiB (config, login files) | `read` |
| `files.read-json-with-secrets(path, pointers)` | JSON with secrets replaced by handles | `read` + `secrets` |
| `files.line-reader(path, from, markers)` + `next-batch` | Complete lines in batches, zstd decoded, marker-filtered | `read` |
| `env.var(name)` / `env.secret(name)` | One variable as text, or as a handle | `env` / `env-secrets` |
| `secret.fingerprint` / `jwt-claims` / `has-prefix` | Use a key without reading it | none (needs a handle) |
| `sqlite.query(path, sql, params)` + `next-rows` | Read-only, `SELECT` only, no `ATTACH`, time limit | `sqlite` globs |
| `keyring.read(service, user)` | One entry, as JSON with secret handles | `keyring` entries + the user's "Allow" (spec Decision 8) |
| `http.request(req)` | HTTPS, no redirects, 30 s, size cap, OTTY `User-Agent`; secrets only to the provider's hosts | `http` hosts |
| `programs.run(program, args, timeout)` | One run, capped output | `run` (program + arg patterns) |
| `programs.line-program(...)` | Talk to a program line by line | `run` |
| `programs.hidden-terminal(...)` | Start in an unseen PTY, send keys, read screens | `terminal` |
| `log(level, text)` | Capped plugin log, shown in dev mode | none |
| WASI clocks, random | Time and random numbers | none |

## 8. Open questions for the owner

1. **New crates.** Besides `wasmtime` and `wasmtime-wasi`: the manifest format needs either
   `toml` (Zed's choice) or JSON (`serde_json` is already in); globs need `globset` or a small
   matcher of our own. Which?
2. **Disk cache of per-file results** (numbers, model names, identities; no text) so a restart
   does not re-read every session file. Allowed?
3. **Probe cleanup.** Claude's `/usage` probe leaves session files that OTTY then deletes
   (`probe_sessions.rs`). Give plugins a narrow "remove only files created by this probe run"
   function, keep it as a host-side feature of the hidden terminal, or stop deleting?
4. **`codex app-server`.** A generic line protocol cannot stop a plugin from sending
   `thread/list`. Trust reviewed plugins, or add a JSON-RPC method allowlist to the manifest?
5. **Who may name a provider and its hosts.** A plugin that finds a key and also declares a
   provider with an unknown host could send the key there. Should provider ids and hosts come
   only from catalog-reviewed provider packages, with agent plugins only tagging keys with those
   ids?
6. **Where built-in plugins live and build.** In this repo as a separate cargo workspace (the
   OTTY build then needs the `wasm32-wasip2` target), or in their own repos with the built
   `.wasm` pulled in at build time?
7. **Catalog hosting and signing key.** GitHub Releases or otty.run; who holds the Ed25519 key.
8. **Review policy.** Who reviews community plugins, and are there trust levels ("official",
   "community", "local") shown in the UI?
9. **wasmtime line.** Pin to LTS 48 (supported to about 2028) or follow monthly releases?
10. **Rust only** for plugin authors at first, with JS and Python left undocumented?
11. **Spike first?** A one-day spike would measure what this research could not: binary size and
    build time with the feature set in 3.1, compile time of a Claude Code plugin, and JSONL
    throughput inside wasmtime against today's native reader.

## 9. Sources

Zed, commit `07278eddc4ac21de2adb5e3e4ca860285c5825e6` (2026-11-19):

- [zed-dev] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/docs/src/extensions/developing-extensions.md
- [zed-cap] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/docs/src/extensions/capabilities.md
- [zed-pub-prereq] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/docs/src/extensions/publishing/prerequisites.md
- [zed-pub-guide] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/docs/src/extensions/publishing/publishing-guide.md
- [zed-pub-update] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/docs/src/extensions/publishing/updating-and-maintenance.md
- [zed-pub-license] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/docs/src/extensions/publishing/license-requirements.md
- [zed-pub-faq] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/docs/src/extensions/publishing/faq.md
- [zed-manifest] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension/src/extension_manifest.rs
- [zed-builder] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension/src/extension_builder.rs
- [zed-api] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_api/src/extension_api.rs
- [zed-world] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_api/wit/since_v0.8.0/extension.wit
- [zed-wit] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_host/src/wasm_host/wit.rs
- [zed-v060] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_host/src/wasm_host/wit/since_v0_6_0.rs
- [zed-v080] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_host/src/wasm_host/wit/since_v0_8_0.rs
- [zed-wasmhost] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_host/src/wasm_host.rs
- [zed-granter] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_host/src/capability_granter.rs
- [zed-host] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/crates/extension_host/src/extension_host.rs
- [zed-defaults] https://github.com/zed-industries/zed/blob/07278eddc4ac21de2adb5e3e4ca860285c5825e6/assets/settings/default.json
- [zed-blog] "Life of a Zed Extension: Rust, WIT, Wasm", 2024-10-21, https://zed.dev/blog/zed-decoded-extensions

Others:

- [zellij-perm] https://zellij.dev/documentation/plugin-api-permissions.html
- [zellij-3349] Switch from Wasmer to Wasmtime, https://github.com/zellij-org/zellij/pull/3349
- [zellij-4449] Migrate from wasmtime to wasmi, https://github.com/zellij-org/zellij/pull/4449
- [zellij-cargo] https://github.com/zellij-org/zellij/blob/088dc456b3576ec5a377c8bcb970ac5451aa6cfd/Cargo.toml
- [extism-manifest] https://github.com/extism/extism/blob/051b765bb6abe4fe95d5e4f489f03ad7b137edbc/manifest/src/lib.rs
- [extism-runtime] https://github.com/extism/extism/blob/051b765bb6abe4fe95d5e4f489f03ad7b137edbc/runtime/Cargo.toml
- [lapce-wasi] https://github.com/lapce/lapce/blob/9e8317c84e9540ac0b09689dd9f9d4fafe44ac00/lapce-proxy/src/plugin/wasi.rs
- [wezterm] https://wezterm.org/config/plugins.html

wasmtime 50.0.3 (crate source read from crates.io; docs.rs pages carry the same text):

- [wt-features] https://docs.rs/crate/wasmtime/50.0.3/features
- [wt-config] https://docs.rs/wasmtime/50.0.3/wasmtime/struct.Config.html (`src/config.rs`)
- [wt-epoch] https://docs.rs/wasmtime/50.0.3/wasmtime/struct.Config.html#method.epoch_interruption
- [wt-limits] https://docs.rs/wasmtime/50.0.3/wasmtime/struct.StoreLimitsBuilder.html
- [wt-pooling] https://docs.rs/wasmtime/50.0.3/wasmtime/struct.PoolingAllocationConfig.html
- [wt-deserialize] https://docs.rs/wasmtime/50.0.3/wasmtime/struct.Module.html#method.deserialize
- [wt-engine] https://docs.rs/wasmtime/50.0.3/wasmtime/struct.Engine.html#method.precompile_compatibility_hash
- [wt-component] https://docs.rs/wasmtime/50.0.3/wasmtime/component/struct.Component.html#method.component_type
- [wt-linker] https://docs.rs/wasmtime/50.0.3/wasmtime/component/struct.Linker.html ("Names and Semver")
- [wt-wasi-features] https://docs.rs/crate/wasmtime-wasi/50.0.3/features
- [wt-wasi-ctx] https://docs.rs/wasmtime-wasi/50.0.3/wasmtime_wasi/struct.WasiCtxBuilder.html#method.new
- [wt-wasi-linker] https://docs.rs/wasmtime-wasi/50.0.3/wasmtime_wasi/p2/fn.add_to_linker_sync.html
- [wt-minimal] https://docs.wasmtime.dev/examples-minimal.html (numbers dated 2024-12-12 on the page)
- [wt-release] https://docs.wasmtime.dev/stability-release.html
- [crates-wasmtime] https://crates.io/crates/wasmtime/versions

Component model, WASI, Rust, guest languages:

- [wasmi] https://github.com/wasmi-labs/wasmi (README of 2.0.2)
- [wit-gates] https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#feature-gates
- [wasi-roadmap] https://wasi.dev/roadmap
- [cm-rust] https://component-model.bytecodealliance.org/language-support/building-a-simple-component/rust.html
- [cargo-component] https://github.com/bytecodealliance/cargo-component
- [rust-wasip2] https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip2.html
- [rust-wasip3] https://doc.rust-lang.org/nightly/rustc/platform-support/wasm32-wasip3.html
- [cjs] https://github.com/bytecodealliance/ComponentizeJS
- [cpy-98] https://github.com/bytecodealliance/componentize-py/issues/98

rusqlite 0.43.0 (OTTY's version):

- [rusqlite-readonly] https://docs.rs/rusqlite/0.43.0/rusqlite/struct.Statement.html#method.readonly
- [rusqlite-auth] https://docs.rs/rusqlite/0.43.0/rusqlite/struct.Connection.html#method.authorizer
- [rusqlite-interrupt] https://docs.rs/rusqlite/0.43.0/rusqlite/struct.InterruptHandle.html

OTTY code read: `crates/agents/src/` on branch `agents` (working tree of 2026-11-19), and
`specs/2026-11-19-agents-accounts/spec.md`.
