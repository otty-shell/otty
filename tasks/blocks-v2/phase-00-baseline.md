# Phase 0: baseline and regression scenarios

Status: **automated work complete; phase acceptance is blocked by manual GUI confirmation**.

Parent document: [Blocks v2 specification](spec.md). IDs: B2-001–B2-005.

## Purpose of this phase

This phase establishes the evidence against which the Blocks v2 architecture will be judged.
It does not implement the final lifecycle, viewport, storage, or transport architecture. It
must leave behind reproducible tests and measurements that answer four questions:

1. Can we reproduce the known failures of the legacy implementation without restoring the
   legacy code to production?
2. Can an automated test detect a viewport jump, broken resize anchor, invalid head-truncation
   fallback, or failed navigation to an off-screen block?
3. Can we measure model cost, snapshot cost, retained block memory, and transport backlog
   without recording terminal contents?
4. Can phase 10 run exactly the same workload and make a meaningful before/after comparison?

The phase result is therefore not a product feature. The result is a versioned baseline
scenario, focused regression tests, a real-shell PTY harness, two recorded benchmark runs, and
a completed manual verification record.

## Expected deliverables

The implementer must produce all of the following:

- model-level regression tests for active output growth, resize, head truncation, and
  off-screen `ScrollToBlock`;
- parser tests proving that legacy `otty-dcs;block` frames are ignored safely;
- a real-PTY Bash/Zsh integration harness and a test-only legacy fixture reproducing nested
  shell loss and colliding block IDs;
- read-only instrumentation for snapshot size/build time, block memory, replaceable frames,
  and lossless queue depth;
- one ignored release-mode integration scenario combining the required load cases;
- `tasks/blocks-v2/baseline-results.md` containing the environment, exact reproduction
  command, two complete result rows, quality-check results, and manual GUI observations.

No new dependency is required or permitted without prior approval. Do not add a benchmark
framework merely to format or time this scenario; `std::time::Instant` and an ignored
integration test are sufficient.

## Inputs and working locations

Use these locations before creating new files or abstractions:

| Responsibility | Primary location | Expected change |
|---|---|---|
| Block model and viewport regression tests | `otty-surface/src/block.rs` | Add focused tests around the existing public/model behavior. |
| Block memory accounting | `otty-surface/src/block/metrics.rs` and block model | Expose deterministic read-only counters; do not expose terminal contents. |
| Legacy DCS behavior | `otty-escape/src/dcs/mod.rs` | Keep legacy framing only as test input and assert that it emits no action. |
| Frame and lossless-event queue counters | `otty-libterm/src/terminal/channel.rs` | Expose current/peak depth and replacement counts without changing delivery semantics. |
| Real Bash/Zsh PTY coverage | `otty/tests/shell_integration.rs` | Use the existing PTY session API; do not substitute pipes. |
| Combined load scenario | `otty/tests/blocks_baseline.rs` | Keep it ignored by default and Unix-gated. |
| Persisted measurements | `tasks/blocks-v2/baseline-results.md` | Record commands, machine details, results, and manual findings. |

If the current repository structure has moved, find the existing owner of the behavior instead
of duplicating it in a new module. Phase 0 instrumentation must remain small and read-only; it
must not become an alternative runtime or benchmark subsystem.

## Required implementation order

Follow this order so the baseline is not accidentally defined by the later fix.

1. **Record the starting environment.** Capture the source commit, dirty-worktree note,
   operating system, kernel, architecture, CPU, total RAM, Rust/Cargo versions, and Cargo
   profile in `baseline-results.md`.
2. **Write failing or behavior-capturing tests first.** Add the focused regression cases for
   B2-001–B2-003 before changing production behavior or adding counters. A legacy defect may
   be reproduced by an isolated test fixture; production code must not regain a legacy path.
3. **Add the minimum instrumentation required by B2-004.** Counters and estimates are
   observations of existing state. They must not copy prompt, command, or output strings into
   logs or reports.
4. **Build the ignored combined scenario for B2-005.** It must exercise the same public path
   that the application uses: protocol parsing, the block model, snapshots, terminal runtime,
   and PTY transport.
5. **Run the focused commands.** Fix failures in the owning package before running the full
   workspace checks.
6. **Run the release scenario twice with identical constants.** Store both final report lines,
   not only an average or a hand-written summary.
7. **Perform the real-window scenario.** Record Bash and Zsh, root and nested-shell results
   separately. Automated tests do not replace this step.
8. **Close the phase only after every acceptance checkbox is supported by a test, report
   field, or explicit manual observation.**

## Detailed requirements

### B2-001 — block model and viewport regressions

Add deterministic tests with small fixtures where possible. Each test must state the setup,
the model mutation, and the observable invariant.

#### Active block growth while manually scrolled

- Create history taller than the viewport.
- Leave `FollowTail` by scrolling to an older visible line.
- Capture the visible cell text or stable anchor.
- Append several lines to the active block while the viewport is not at the bottom.
- Assert that the same historical content remains visible. Merely asserting that the scroll
  offset is non-zero is insufficient because the offset can change while the view jumps.

#### Resize while anchored

- Start with wrapped history and an anchored old line.
- Resize through the logical equivalent of `80 -> 200 -> 40` columns. Unit tests may use
  smaller proportional values for speed, but the combined and manual scenarios must use the
  real values.
- After every resize, build a snapshot and assert that the same logical line or block remains
  visible. Visual row numbers may change because wrapping changes.

#### Head truncation

- Configure a deliberately small retained-block limit.
- Verify first that adding history does not move an anchor whose block is still retained.
- Add enough history to remove the anchored block.
- Assert the documented fallback: the anchor moves to the nearest retained successor, not to
  the active tail and not to an invalid block ID.
- The test must also prove that no unfinished block is removed to satisfy the limit.

#### Off-screen `ScrollToBlock`

- Create at least three blocks with a viewport that cannot show all of them.
- Choose a block that is completely absent from the current snapshot.
- Call `scroll_to_block` with a stable `BlockId` and `BlockAlignment::Start`.
- Assert that the call reports success and that the next snapshot contains at least one line
  of the requested block.
- Do not locate the target through the current frame's rectangles; a current frame cannot
  describe a fully off-screen block.

### B2-002 — legacy protocol fixture and final behavior

`otty-dcs;block` is legacy test data, not a supported production protocol.

The parser tests must collectively cover all of the following inputs:

- a complete, formerly valid legacy frame;
- the same kind of frame delivered one byte at a time;
- an empty legacy payload;
- a malformed legacy payload;
- a valid protocol-v2 frame after legacy input on the same parser instance.

The required assertions are:

- no legacy input emits a semantic `Action` or creates a block;
- malformed or fragmented legacy input does not panic;
- parser state recovers, demonstrated by accepting the following valid v2 frame exactly once;
- no compatibility adapter, runtime switch, or production v1 schema is introduced.

### B2-003 — real-shell PTY regression harness

The integration harness must use a controlling pseudo-terminal because prompt hooks and
interactive nested shells behave differently when stdin/stdout are ordinary pipes.

The harness must:

- launch the requested shell interactively through the project's PTY API;
- collect raw PTY bytes until both the child status is known and the PTY closes;
- parse emitted protocol events through the production parser;
- enforce a timeout and print enough safe context to identify the failed stage;
- treat Bash as required on the primary Unix test environment;
- check whether Zsh is available and record a capability skip when it is not.

Keep the old failure mechanism in a test-only fixture. The fixture is successful only when it
reliably demonstrates both historical symptoms:

- starting a nested interactive shell loses or overwrites the parent/child integration
  context;
- root and nested commands can receive the same legacy ID, for example two `cmd-1` values.

Production-v2 assertions in this phase must prove that root and child shell-instance IDs
differ and block IDs are unique. Parent-context recovery after child exit belongs to the
complete Bash/Zsh lifecycle matrix in phase 3.

### B2-004 — metrics and privacy contract

Instrumentation must report mutable active content separately from finished history. Memory
figures are deterministic model estimates, not process RSS, and their names must make that
clear in documentation.

The final `BLOCKS_BASELINE` line must contain the following fields:

| Field | Meaning |
|---|---|
| `version` | Report schema version. Increment it if fields or meanings change. |
| `requested_blocks` | Blocks generated before the retention policy is applied. |
| `retained_blocks` | Blocks still present when measurements are taken. |
| `finished_blocks`, `active_blocks` | Retained lifecycle categories, reported separately. |
| `finished_lines`, `active_lines` | Retained logical/model lines by lifecycle category. |
| `columns`, `viewport_lines` | Final terminal dimensions used for the snapshot. |
| `long_output_lines` | Lines appended to the mutable active block. |
| `model_duration_ms` | Duration of model construction, output, resize, and snapshot preparation before the PTY queue scenario. |
| `snapshot_bytes` | Estimated owned snapshot storage, including cells and snapshot-owned metadata. |
| `snapshot_build_us` | Time to create the measured owned snapshot. |
| `block_memory_bytes` | Total deterministic estimate of retained block model memory. |
| `active_memory_bytes`, `finished_memory_bytes` | The same estimate split by mutable active content and finished history. |
| `queue_output_lines`, `queue_duration_ms` | PTY load and time until the terminal runtime drains it. |
| `replaceable_frame_depth` | Current unread latest-frame mailbox depth; it must be `0` or `1`. |
| `replaced_frames` | Number of unread render frames superseded by newer frames. |
| `lossless_queue_depth` | Current queued critical-event count at the end of the scenario. |
| `max_lossless_queue_depth` | Largest observed critical-event count during the scenario. |
| `scroll_correct` | Combined assertion result for off-screen navigation and resize-anchor preservation. |

`BLOCKS_BASELINE_STAGE` lines may report a stage name and duration so a timeout can be
localized. Neither stage nor final lines may contain prompt text, commands, output, working
directories, environment values, or raw protocol payloads.

Two identical runs must emit the same schema and scenario parameters. Timing and replaced
frame counts are expected to vary; do not require byte-for-byte identical output. Structural
counts and deterministic byte estimates must match unless the source or scenario changed.

### B2-005 — combined release scenario

Create one ignored test named for phase 0 and run it in the optimized profile. It must use
fixed, visible constants and exercise this sequence:

1. Build 10,000 small lifecycle blocks through protocol-v2 events.
2. Apply the current retention policy and retain its observed block/line counts.
3. Create one mutable active block and append 100,000 output lines.
4. Navigate to an old, fully off-screen block.
5. Resize from 80 to 200 and then 40 columns with a 24-line viewport, checking the anchor
   after each resize.
6. Build one owned snapshot and capture build time and estimated bytes.
7. Capture total, active, and finished block-memory estimates.
8. Start a real `/bin/sh` PTY that emits 50,000 lines while the event consumer deliberately
   remains unread until the child exits.
9. Fail after 30 seconds so a PTY deadlock does not hang CI or a developer terminal.
10. Assert scroll correctness, a replaceable frame depth no greater than one, at least one
    replaced frame, and a bounded/lossless critical-event queue.
11. Print exactly one machine-readable `BLOCKS_BASELINE version=1 ...` summary line.

This scenario is ignored so ordinary debug test runs stay fast and stable. Ignored does not
mean optional: it must be executed explicitly when phase 0 is recorded and again in phase 10.

## Recorded implementation state

- [x] **B2-001** Model regression tests cover active growth, resize, retained/removed head
  anchors, and a fully off-screen `ScrollToBlock`.
- [x] **B2-002** Legacy framing exists only as test input; complete, fragmented, empty, and
  malformed `otty-dcs;block` input emits no semantic event and does not prevent later v2
  parsing.
- [x] **B2-003** Bash/Zsh integration tests use a real PTY. A test-only fixture reproduces lost
  nested integration and colliding IDs, while v2 tests require unique shell and block IDs.
- [x] **B2-004** Snapshot, block-memory, replaceable-frame, and lossless-queue measurements are
  available with active and finished content separated and no terminal payload in reports.
- [x] **B2-005** The ignored release scenario covers 10,000 blocks, 100,000 active output
  lines, resize, off-screen navigation, and a 50,000-line slow-consumer PTY workload.
- [x] Two automated runs and their machine details are stored in
  [baseline-results.md](baseline-results.md).
- [ ] The manual Bash/Zsh root/nested GUI matrix is recorded in
  [baseline-results.md](baseline-results.md).

## Automated verification

### Focused feedback loop

Run these commands from the workspace root while implementing the corresponding item:

```bash
cargo test -p otty-surface block
cargo test -p otty-escape dcs
cargo test -p otty-libterm terminal::channel
cargo test -p otty --test shell_integration -- --nocapture
```

If Zsh is not installed, record the capability skip in `baseline-results.md`. A missing Bash
binary or a timed-out PTY is a failure, not a skip.

### Record the numerical baseline

Run the following command twice without changing source, constants, machine load setup, or
Cargo profile between runs:

```bash
cargo test --release -p otty --test blocks_baseline -- --ignored --nocapture
```

For each run, copy the final `BLOCKS_BASELINE version=1` line or transcribe every field into a
two-run table in `baseline-results.md`. Confirm that both result sets contain the complete
field set above. If a run fails, preserve the failing stage and classify it as one of:
model/memory, snapshot latency, viewport correctness, PTY timeout, replaceable-frame depth, or
lossless queue delivery.

### Required repository-wide closeout

After focused tests pass, run the project-mandated checks:

```bash
cargo +nightly fmt
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features
```

Record overall line, region, and function coverage. Overall line coverage must not be lower
than it was before the phase. Existing `cargo deny` warnings may be documented only if the
command exits successfully and this phase introduced no new warning.

## Manual GUI verification

This procedure requires a real application window. Wayland environments generally do not
allow the test process to synthesize the required global wheel and keyboard input, so a
successful launch alone does not complete the phase.

### Preparation

1. From the workspace root, launch `cargo run -p otty` and wait for the terminal to become
   interactive.
2. Make the terminal approximately 80 columns by 24 rows. Exact pixel dimensions are not
   important; record the observed terminal columns and rows.
3. Create a result subsection in `baseline-results.md` for each context: Bash root, nested
   Bash, Zsh root, and nested Zsh. Record `pass`, `fail`, or `not available` for every check.

### Scenario to run in each shell context

1. Start the required shell interactively. For a nested context, launch a second interactive
   instance of the same shell from the first one and confirm that both remain usable.
2. Run this command to print 100,000 numbered lines:

   ```sh
   i=1; while [ "$i" -le 100000 ]; do printf '%06d\n' "$i"; i=$((i + 1)); done
   ```

3. While output is still arriving, scroll with the mouse wheel to roughly the middle of
   history. Note the top and bottom numbered lines currently visible.
4. Wait for more output. Pass only if the viewport stays on the same logical area; a jump to
   newer output or to the bottom is a failure.
5. Repeat the previous two steps using keyboard scrolling so mouse and keyboard paths are both
   exercised.
6. While still viewing old history, resize the window through approximately 80, 200, and 40
   columns. Wrapping may change. Pass only if the same numbered logical line remains anchored
   in the viewport after every resize.
7. Return to the bottom, create several short command blocks, then use the application's block
   navigation action on a block that is completely outside the viewport. Pass only if the
   requested block becomes visible; scrolling to a neighboring block is a failure.
8. Exit the nested shell, if applicable. Run one more command in the parent and confirm that
   it receives a new block and does not alter a nested-shell block.

### After the GUI scenario

1. Run the ignored release baseline command once more on the same source revision.
2. Compare structural counts and memory estimates with the two recorded automated runs.
   Timing may vary, but missing fields, different fixed workload parameters, an unbounded
   queue, or `scroll_correct=false` is a failure.
3. In `baseline-results.md`, record the date, desktop/session type, shell/version, terminal
   dimensions, result of each step, and a short payload-free description of any visual jump.
   Do not paste prompt, command output, or protocol payloads into the report.

## Failure handling

- A deterministic regression-test failure blocks the phase; do not replace the assertion with
  a looser timing threshold.
- A timing outlier should be rerun after recording system load. If correctness and structural
  fields still pass, keep the outlier and explain it rather than deleting it.
- A GUI failure must include the shell context, resize step, and whether the viewport jumped
  toward the head or tail. Do not include terminal contents.
- If Zsh is unavailable for automated coverage, record the explicit skip. Phase acceptance on
  a target that claims Zsh support still requires the manual Zsh scenario on a machine where
  Zsh is installed.
- Do not fix a phase 1–10 architectural issue inside the baseline scenario merely to make the
  number look better. Preserve the reproduction, link the failure to its owning phase, and
  repeat the identical scenario after that phase is implemented.

## Definition of Done

Phase 0 is complete only when all statements below are true:

- [x] B2-001–B2-005 each have an automated test or a named report field proving their result.
- [x] The legacy defect is reproducible only through a test fixture; no production v1 path was
  restored.
- [x] The combined scenario uses fixed parameters, has a timeout, and is ignored in ordinary
  test runs.
- [x] Two release runs with the same metric schema and environment details are stored in
  `baseline-results.md`.
- [x] Focused tests, workspace checks, `cargo deny`, and coverage were run and their outcome is
  recorded.
- [ ] A person completed and recorded the scroll, resize, off-screen navigation, and nested
  Bash/Zsh checks in a real GUI.
- [ ] Another developer can reproduce both the automated report and the manual procedure from
  these documents without verbal instructions.

Until the final two items are checked, report the phase as **automated work complete, awaiting
manual GUI confirmation**, not as fully complete.
