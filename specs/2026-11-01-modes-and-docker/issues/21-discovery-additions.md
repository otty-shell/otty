# 21 — Discovery: more sockets, CLI install folders, `podman-docker` wrapper

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 07
Spec: [../spec.md](../spec.md)

## What

Additions to engine discovery in `crates/docker` that came out of ticket 07
(spec Decision 8, owner approved 2026-11-01). Tests first.

- Socket candidates, after the Podman ones and before deduplication: Docker
  Desktop on Linux `~/.docker/desktop/docker.sock`, Colima
  `~/.colima/default/docker.sock` (Linux and macOS), OrbStack
  `~/.orbstack/run/docker.sock` (macOS). Their engines show as Docker (the
  `/version` answer decides the kind, as today).
- CLI lookup: besides `PATH`, look in `/usr/local/bin`, `/opt/homebrew/bin`,
  `~/.docker/bin`, `/Applications/Docker.app/Contents/Resources/bin` and
  `/opt/podman/bin`. Keep the lookup std only.
- A `docker` command that is really Podman (Fedora `podman-docker`) counts as
  Podman: run `docker --version` with a short time limit and look for
  "podman" in the answer. Then the diagnosis gives "Podman API off" instead
  of "Docker not running" for a Podman-only user. Keep the check a small pure
  function over the command's output; the process call stays thin.
- Research and note in `## Comments`: whether Colima's older default path
  (`~/.colima/docker.sock`) is still used, and what `docker --version` prints
  with `podman-docker` installed.

All new text goes through `i18n` in the ten locales (spec Decision 17); this
ticket is expected to add none. Run fmt, clippy, tests, `cargo deny check`
and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Candidate tests for each new socket on Linux and macOS, still deduplicated.
- CLI lookup tests: found only in an install folder, found on `PATH`, not found.
- Wrapper tests: Docker's and Podman's `--version` output; the diagnosis for a
  `podman-docker` user with no engine answering is "Podman API off".
