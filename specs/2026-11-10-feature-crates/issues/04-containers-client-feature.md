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
