# Blocks v2: phase 0 baseline results

Measurement date: 2026-08-10. Report schema: `BLOCKS_BASELINE version=1`.

## Reproduction

Run the same command twice from the workspace root:

```bash
cargo test --release -p otty --test blocks_baseline -- --ignored --nocapture
```

The final line of each run starts with `BLOCKS_BASELINE version=1` and contains only numeric
parameters and boolean assertions. Prompt, command, and output contents are never included in
the report. `BLOCKS_BASELINE_STAGE` lines separate block construction, long output, and PTY
queue latency so a timeout or slowdown can be localized.

The scenario exercises:

- 10,000 small lifecycle blocks with the current model's retention policy;
- 100,000 lines of mutable active output;
- off-screen `ScrollTo` and resize from 80 to 200 to 40 columns;
- snapshot build time and size plus approximate block memory, with separate active and history
  values;
- 50,000 lines through a real PTY without reading from the consumer until the child exits;
- assertions for scroll correctness, replaceable frame depth, and lossless queue depth.

The PTY queue scenario has a separate 30-second timeout. Benchmark failures for memory,
latency, queue depth, or scroll correctness are reported by separate fields and assertions.

## Machine

- OS: Fedora Linux, kernel `7.0.12-101.fc43.x86_64`, `x86_64`.
- CPU: AMD Ryzen 9 7900, 12 cores / 24 threads.
- RAM: 31,942,900 KiB total.
- Rust: `rustc 1.96.1 (31fca3adb 2026-06-26)`, LLVM 22.1.2.
- Cargo: `cargo 1.96.1 (356927216 2026-06-26)`.
- Source HEAD: `8eff7639be53`, branch `blocks-v2`, including phase 0 working-tree changes.
- Profile: Cargo `release`, optimized.

## Results

Both runs emitted the same set of fields.

| Metric | Run 1 | Run 2 |
|---|---:|---:|
| requested blocks | 10,000 | 10,000 |
| retained blocks | 1,000 | 1,000 |
| finished / active blocks | 999 / 1 | 999 / 1 |
| finished / active lines | 23,976 / 10,024 | 23,976 / 10,024 |
| columns / viewport lines | 40 / 24 | 40 / 24 |
| long output lines | 100,000 | 100,000 |
| model duration | 1,647 ms | 1,652 ms |
| snapshot build | 14,952 µs | 16,306 µs |
| snapshot estimated bytes | 273,458 | 273,458 |
| total block memory estimate | 59,570,002 bytes | 59,570,002 bytes |
| active memory estimate | 9,969,651 bytes | 9,969,651 bytes |
| finished memory estimate | 49,600,351 bytes | 49,600,351 bytes |
| PTY queue output lines | 50,000 | 50,000 |
| PTY queue duration | 265 ms | 261 ms |
| replaceable frame depth | 1 | 1 |
| replaced frames | 4,847 | 5,096 |
| lossless queue depth / peak | 1 / 1 | 1 / 1 |
| scroll correct | true | true |

`snapshot estimated bytes` includes the owned frame, cells, block metadata and text
allocations, damage, and hyperlink index. `block memory estimate` includes block structures,
logical grid rows, cells, metadata, and retained text allocations. It is a deterministic
approximation rather than process RSS. Mutable active content and finished history are counted
separately.

## Quality checks and coverage

The required workspace checks passed. Overall line coverage increased from 51.05% to 51.16%,
region coverage from 52.27% to 52.41%, and function coverage from 52.01% to 52.12%.
`cargo deny check` completes successfully while retaining two pre-existing warnings for the
yanked transitive crates `core2 0.4.0` and `spin 0.9.8`.

## GUI and shell smoke test

- `cargo run -p otty` successfully opened a Wayland window and ran for 30 seconds without a
  panic or crash.
- Bash, Zsh, and nested Bash run through a real PTY in `shell_integration`.
- A test-only legacy fixture consistently reproduces lost nested integration and two identical
  `cmd-1` IDs; the production v2 test confirms unique shell and block IDs.
- Model regression tests cover active growth, head retention and truncation fallback, the
  resize matrix, and fully off-screen `ScrollTo`.

The interactive visual scenario using wheel and keyboard scrolling remains documented in
`phase-00-baseline.md`. A person must confirm its results on the target desktop because Wayland
does not allow the test process to synthesize global input.

### Manual GUI verification result

Status: **awaiting confirmation**.

- Bash root/nested: anchor during output and resize not verified; off-screen `ScrollTo` not
  verified.
- Zsh root/nested: anchor during output and resize not verified; off-screen `ScrollTo` not
  verified.
