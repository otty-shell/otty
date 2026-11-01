# 07 — `otty-docker` crate: bollard, engine discovery for Docker and Podman

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

New crate `crates/docker`, package `otty-docker` (spec Decisions 7, 8, 9, 16).
Tests first.

- Add `bollard = "0.21.1"` (default features) and `futures-util`, approved by the owner on 2026-11-01. Add the crate to the workspace members. `cargo deny check` stays clean.
- `socket_candidates(...)`: a pure function from environment (`DOCKER_HOST`, `XDG_RUNTIME_DIR`, `HOME`, uid) and platform to the ordered list in Decision 8. Same socket counted once.
- Research for macOS, note the answer in Comments: where the socket of the default Podman machine is (Podman 5 and 4), and the Docker Desktop paths.
- `EngineInfo` (kind Docker or Podman, version, address). The kind comes from `/version`: a "Podman Engine" component means Podman.
- `Diagnosis`: reached engines, or not installed / Docker not running / Podman API off / no permission (Decision 9). A pure function builds it from per-candidate results and which CLIs are on `PATH`; the `PATH` lookup uses std only.
- I/O: connect, ping and version per candidate with a short timeout; no retries here.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Unit tests: candidates with and without `DOCKER_HOST` (unix and tcp), with and without `XDG_RUNTIME_DIR`, Linux and macOS, deduplication.
- Unit tests: every `Diagnosis` case; kind detection from captured `/version` JSON of Docker and of Podman (fixtures in `tests/fixtures/`).
- One `#[ignore]` integration test against a live engine.
