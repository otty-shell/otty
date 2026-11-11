# 04 Docker types without the client

Status: needs-triage
Blocked by: —

## What to do

The containers feature needs the data types of `otty-containers` (`Container`,
`EngineKind`, `EngineAddress`, `Image`, `RunRequest` and others), but must not
be able to call Docker. Split the library with a cargo feature:

- add a `client` feature to `otty-containers`, on by default, so the old app and
  every current user keep working without changes;
- behind `client`: everything that talks to an engine or the system
  (`Engine`, `discover`, `Probe`, `host_folder_exists`, `wait_for`, the CLI
  check, logs, pulls, watching and events streams) and the dependencies only
  they need (`bollard`, `tokio`, `futures-util`);
- without `client`: the data types, `compare_text`, `MAX_LOG_LINE_BYTES` and the
  `fixtures` feature.

Where a data type is built from a `bollard` answer, the conversion goes behind
`client` and the type itself stays outside it.

Write the tests first: a build of the crate with `--no-default-features` that
uses the data types, and a check that `Engine` is not there.

## Done when

- `cargo build -p otty-containers --no-default-features` builds and has no
  `bollard` or `tokio` in its dependency tree.
- The default build and all its tests behave as before.
- The old `app` builds unchanged.
- fmt, clippy, `cargo deny check`, the tests and the coverage comparison from
  AGENTS.md are run, and their results are reported.

## Comments

### 2026-11-10: the `client` feature is in

`otty-containers` has a `client` feature, on by default, so the old app and
its tests build as before. Without it the crate has no dependencies at all:
`bollard`, `tokio`, `futures-util`, `log`, `serde_json` and `thiserror` are
used only by the client, so all six come with `client`.

Behind `client`:

- `Engine`, `EngineError` and every call on an engine (lists, actions, run,
  pull, logs, inspect, image defaults, watching and the events stream);
- `discover`, `host_folder_exists`, `wait_for`, and the parts of the search
  that read this computer: the socket list, the `docker` and `podman` lookup,
  the probes, the SELinux check;
- the mapping of each bollard answer into a data type (`from_summary`,
  `from_volume`, `from_version`, `PullProgress` reading a pull line, the create
  request of a `RunRequest`, and so on), and the `pub(crate)` constructors that
  only the client and `fixtures` call.

Without `client`: `Container` and its parts, `ContainerState`, `Image`,
`Volume`, `Network`, `Resources`, `EngineKind`, `EngineInfo`, `EngineAddress`,
`Discovery`, `Diagnosis`, `Probe`, `ProbeOutcome`, `SocketCandidate`,
`CandidateSource`, `CliPresence`, `ImageDefaults`, `RunRequest` and the run
types, `PullProgress`, `LogLine`, `LogPosition`, `ContainerRun`, `ListKind`,
`WatchEvent`, `compare_text` and `MAX_LOG_LINE_BYTES`.

Decisions:

- `fixtures` turns on `client`. It reads the captured engine answers with
  bollard's models and maps them with the client's mapping code, and builds
  `Discovery` values with the client-only constructors. A feature crate that
  takes `fixtures` in its dev-dependencies gets the client in its test build
  only; its normal build stays without it.
- The issue lists `Probe` behind `client`. The probe run is there, but the
  `Probe` and `ProbeOutcome` values stay outside it: `Discovery::probes`
  returns them and the no-engine cards show them.
- `EngineError` stays behind `client`: it holds a bollard error.
- `Diagnosis` is plain data, but the `diagnose` logic that builds it runs only
  during the search, so it is behind `client` with the search.
- Types that lived in a file that is all client moved to a file of their own:
  `SocketCandidate` (from `candidates.rs`), `CliPresence` (from `cli.rs`),
  `Probe` (from `probe.rs`), `Resources` (from `connection.rs`), `ListKind`
  and `WatchEvent` (from `events.rs` and `watch.rs`). In `logs.rs` the client
  code reads the private fields of the log types, so it moved into the child
  module `logs/follow.rs` instead, and `MAX_LOG_LINE_BYTES` moved to
  `logs.rs`.

Proof: `tests/without_client.rs` uses the data types and builds only without
`client`; two `compile_fail` examples in the crate docs, present only in that
build, check that `Engine` and `discover` are not there. Run them with
`cargo test -p otty-containers --no-default-features`; CI (`tests.yaml`) now
runs that too. `cargo tree -p otty-containers --no-default-features -e normal`
lists only the crate itself.
