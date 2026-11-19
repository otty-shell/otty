# WASM plugins spike: measurements

Done on 2026-11-19 on branch `spike/wasm-plugins`, to check the numbers that
[wasm-plugins.md](./wasm-plugins.md) marks "not measured". The code is a
throwaway prototype and is not meant for merging.

## 1. What was built

- **`crates/plugins/host`** (`otty-plugin-host`, PROTOTYPE). wasmtime 48.0.10 (LTS) with only
  the `runtime`, `cranelift`, `component-model` and `std` features, plus `wasmtime-wasi` 48 with
  only `p2`. Set up as the research recommends: the sync API; epoch interruption with a ticker
  thread that moves the epoch on every 10 ms and a deadline set before each call; `StoreLimits`
  with a memory cap (256 MiB), at most 4 instances, and a trap when memory can not grow; an empty
  `WasiCtx` (no files, no environment, no network, stdio closed); the default on-demand
  allocator; compiled components kept on disk with `Engine::precompile_component` and read back
  with `Component::deserialize_file`.
- **WIT world `otty:agent@0.1.0`** (`crates/plugins/host/wit/agent.wit`). The host gives
  `read-lines(file, offset, max-bytes, markers) -> result<batch, string>`: complete lines from a
  byte offset, about 1 MiB per call, in one buffer, and only the lines that hold one of the
  markers. The plugin gives `read-spend(file, host-markers) -> file-spend` (tokens per model per
  day, plus records seen and understood) and `read-answers(file) -> list<answer>` (one record per
  answer, the shape the research's world returns). A `misbehave` interface (`spin`, `hold`) is
  there only for the limit checks. The host refuses a file outside the folder it was given (the
  spike's stand-in for the manifest's read globs).
- **`crates/plugins/claude-code`** (`otty-plugin-claude-code`, PROTOTYPE). The per-record reading
  of `crates/agents/src/claude_code/spend_file.rs` ported to a component with `wit-bindgen`
  0.57.1, built with plain `cargo build --target wasm32-wasip2` (no `cargo-component`). It has its
  own `[workspace]`, so a workspace build never compiles it for the host. Left out of the port:
  the project folder, the agent version and the field names of a changed format; none changes the
  totals. Codex was not ported (see section 5).
- **Bench** (`crates/plugins/host/examples/bench.rs`). Runs today's native reader (opened through
  a `spike` cargo feature of `otty-agents`) and the plugin on the same files and compares totals
  per (day, model). It prints only times, counts, byte counts and token totals.
- **Link into the app**: a `wasm-plugins` cargo feature of `otty`. With it, `main` reads
  `OTTY_SPIKE_PLUGIN` and, when set, loads the plugin and counts one file. Because the check is at
  run time, wasmtime can not be optimized out (13,206 wasmtime/cranelift symbols in the binary).

## 2. How it was measured

- Machine: 24 logical CPUs, 30 GiB RAM, Linux 7.2 (Fedora 43), Rust 1.96.1.
- Every build ran with `nice -n 15` and `-j 2`, one cargo job at a time. **Build times are
  therefore longer than on a normal `-j 24` build**; read them as relative, not absolute.
- Release profile of the workspace is cargo's default (no LTO, no strip). The plugin's release
  profile: `opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = true`.
- JSONL runs read every file once first, so both sides read from the page cache; each run was
  done twice and round 2 is reported (round 1 differed by less than 1%).
- Real data: the owner's Claude Code session files under `~/.claude/projects`, opened read-only.
  Synthetic data: 20 made-up files, about 100 MB, written by `bench gen` into `target/spike/`.

## 3. Numbers

### Binary size of `otty` (release, x86_64 Linux)

| | Bytes | Stripped |
| --- | ---: | ---: |
| Before (branch `agents`) | 76,683,208 | 62,623,872 |
| With `wasm-plugins` | 98,467,712 | 79,116,288 |
| Difference | **+21,784,504 (+28%)** | **+16,492,416 (+26%)** |

The dependency tree of `otty` grows from 641 to 738 crates. `tokio` gets no new features: `iced`
already turns on the multi-thread runtime and `net` that `wasmtime-wasi` asks for.
`Cargo.lock` grows by 859 lines.

### Build time (`nice -n 15`, `-j 2`)

| Build | Wall | CPU |
| --- | ---: | ---: |
| Clean release build of `otty-plugin-host` and its dependencies, empty target folder (160 crates) | 2 min 50 s | 301 s |
| Release `otty` with `wasm-plugins` on top of an up-to-date release build | 8 min 32 s | 970 s |
| Release `otty` incremental after touching one file of the host crate | 25 s | 43 s |
| Release `otty` without the feature, for comparison (after the branch switch) | 1 min 20 s | 151 s |
| Plugin, clean, `wasm32-wasip2` release | 18 s | 27 s |

Slowest crates of the clean host build: `cranelift-codegen` 87 s, `wasmtime` 26 s,
`wasmtime-wasi` 20 s, `wasmtime-environ` 20 s, `wasmtime-internal-cranelift` 19 s, `wasmparser`
17 s, `otty-plugin-host` (the `bindgen!` output) 12 s. The 8.5 minutes of the second row are more
than the wasmtime crates: turning the feature on changes the features of crates shared with the
rest of the app, so cargo rebuilds them too. A debug build was not measured.

### Plugin

| | Value |
| --- | ---: |
| `.wasm` size, `opt-level = 3` | 510,717 bytes |
| `.wasm` size, `opt-level = "s"` | 363,629 bytes (but 22% to 2x slower, see below) |
| First load: read, compile with Cranelift, write the cache | 476-498 ms (`opt-level = "s"`: 340 ms) |
| Compiled `.cwasm` on disk | 1,421,232 bytes |
| Cached load (`Component::deserialize_file`), median of 20 | 277-285 µs |
| Instantiate (linker, WASI, store, instance), median of 10 | 114-123 µs |
| Linear memory of a fresh instance | 1,152 KiB |
| Process memory (RSS) of 10 idle instances, in all | 560-616 KiB |
| Peak linear memory reading the 100 MB fixture | 3,200 KiB |
| Peak linear memory reading the real files (largest file 32.5 MB) | 7,488 KiB |

Compiling is single-threaded because the `parallel-compilation` feature was left out; with the
cache it happens once per plugin version and wasmtime version.

The plugin imports only `otty:agent/files@0.1.0`, `wasi:io/{poll,error,streams}@0.2.6` and
`wasi:cli/{environment,exit,stdin,stdout,stderr,terminal-*}@0.2.6`. Rust's standard library, for
this code, imports no `wasi:filesystem`, `wasi:sockets`, `wasi:clocks` or `wasi:random`.

### JSONL speed: native reader vs plugin

Real Claude Code files: 538 files, 1,455,913,326 bytes.

| Reader | Time | Speed | vs native |
| --- | ---: | ---: | ---: |
| Native (today's `read_spend`) | 2.07 s | 705 MB/s | 1.00x |
| Plugin, host keeps only lines with a marker | 2.69 s | 541 MB/s | 1.30x |
| Plugin, host copies every line | 5.68 s | 256 MB/s | 2.75x |
| Plugin, list of answers (host filter) | 2.63 s | 555 MB/s | 1.27x |

Synthetic fixture: 20 files, 100,033,074 bytes.

| Reader | Time | Speed | vs native |
| --- | ---: | ---: | ---: |
| Native | 167 ms | 599 MB/s | 1.00x |
| Plugin, host filter | 240 ms | 416 MB/s | 1.44x |
| Plugin, every line | 413 ms | 242 MB/s | 2.47x |
| Plugin, list of answers | 227 ms | 441 MB/s | 1.36x |
| Plugin built with `opt-level = "s"`, host filter | 293 ms | 341 MB/s | 1.75x |
| Plugin built with `opt-level = "s"`, every line | 824 ms | 121 MB/s | 4.98x |

**Totals match exactly** on both data sets, for every plugin variant. Real data: 72,803 answers
in 32 (day, model) rows; input 297,401,510, cached 14,797,543,674, cache-written 297,247,638,
output 98,793,059 tokens; 72,810 answer records seen, 72,810 understood. Fixture: 16,158
answers, 30 rows, all seen and understood. The counts are summed without the cross-file
deduplication of `count_spend`, on both sides alike.

### Limits

| Check | Result |
| --- | --- |
| Endless loop, deadline 100 ticks of 10 ms | Stopped after 1.005 s with `wasm trap: interrupt` |
| Plugin holds 64 MiB under a 256 MiB cap | Allowed; peak linear memory 65.1 MiB |
| Plugin asks for 512 MiB under a 256 MiB cap | Trap after 32 µs: "forcing trap when growing memory to 538050560 bytes" |
| Plugin reads `/etc/hostname`, outside its folder | Host returned an error; the plugin counted nothing |

## 4. Problems hit

- **The host's marker search was slow at first.** The first host filter compared each window
  with a marker whose length is only known at run time, which became a `memcmp` call per byte:
  the "host filter" plugin then ran at 168 MB/s on the fixture, *slower* than copying every line.
  Checking the first byte before the rest brought it to 416 MB/s. Today's native code is fast
  because its markers are constants. A real host should use a proper substring search
  (`memchr::memmem`; `memchr` 2.7 is already in `Cargo.lock`).
- **Turning the feature on rebuilt much of the app** (8.5 minutes at `-j 2`), because of feature
  unification on shared crates, not only because of wasmtime.
- **`cargo deny` refused 31 crates** of wasmtime and Cranelift for their licence,
  `Apache-2.0 WITH LLVM-exception`, which `deny.toml` did not list. The spike adds it to the
  allow list, marked PROTOTYPE; keeping it is the owner's call. Advisories, bans and sources
  passed with no change.
- **Disk.** The spike's target folders take about 1.6 GB (the clean host build alone 839 MB).
- No API trouble: `bindgen!`, `add_to_linker_sync`, `WasiView`, `StoreLimits`, epochs and the
  component cache all worked as the wasmtime 48 docs say, on the first build. `wasmtime-wasi`'s
  `p2` feature still turns on `wasmtime/async` (fibers) though the host is fully sync.

## 5. Not measured

- Codex was not ported. Its reader keeps state across lines (previous total, model of the turn)
  and some files are zstd-compressed; the host would decompress (research 4.2). Its 10.8 GB of
  local files would also make a long run on this machine.
- Debug build time and size; macOS and Windows; `parallel-compilation`; epoch interruption
  turned off (to see its cost); the Pulley interpreter instead of Cranelift.

## 6. Verdict

**wasmtime is workable for OTTY.** The JSONL reading inside a plugin is 1.3x native on real data
(1.27x when it returns one record per answer), with exactly the same totals, under 8 MiB of plugin
memory. A cached plugin loads in 0.3 ms and starts in 0.1 ms. A stuck plugin is stopped on time,
and the memory cap holds. The price is paid in the binary and in the build: **+16.5 MB stripped
(+26%)** and about 3 extra CPU-bound minutes of a clean build at `-j 2`, almost half of it in
`cranelift-codegen`.

What to change in the research recommendation:

1. **Replace "not measured" in 3.5** with the numbers above; the owner should decide whether
   +16.5 MB stripped is acceptable before the spec.
2. **Host-side marker filtering is required, not optional** (4.2): it makes the plugin 2.1x
   faster on real data. The host must search with `memchr::memmem` or similar, not a naive loop.
3. **Return answers, not totals** (4.7 is right): a list of answers costs no more than per-day
   totals, so the tally and deduplication can stay in the host as planned.
4. **A plain function with an offset was enough** for batches (`read-lines(file, offset,
   max-bytes, markers)`); the offset maps directly to the incremental reads of 4.2. A
   `line-reader` resource is still fine, but not needed for speed.
5. **The memory cap can be smaller**: 64 MiB leaves 8x room over the largest real file seen.
6. **The compile cache is required** (0.5 s compile vs 0.3 ms load per plugin). Consider turning
   on `parallel-compilation` (rayon is already in `Cargo.lock`) for the first load.
7. **The import allowlist can be narrower** than 3.1 says: the plugin needs only `wasi:cli` and
   `wasi:io`; refuse `wasi:filesystem` and `wasi:sockets` at install time, and allow
   `wasi:clocks` and `wasi:random` only if a later plugin needs them.
8. **Build plugins with `opt-level = 3`**, not `"s"`: 150 KB saved costs up to 2x speed.
9. **Epochs**: a 10 ms tick gave 1.005 s for a 1 s deadline; Zed's 100 ms would also do.
