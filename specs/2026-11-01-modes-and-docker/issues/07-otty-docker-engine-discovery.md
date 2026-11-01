# 07 — `otty-docker` crate: bollard, engine discovery for Docker and Podman

Status: resolved
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

## Comments

### Research: socket paths on macOS and the `/version` answer (2026-11-01)

**Docker Desktop on macOS.** Docker's page
[Understand permission requirements for Docker Desktop on Mac](https://docs.docker.com/desktop/setup/install/mac-permission-requirements/)
says:

- The per-user socket is `/Users/<user>/.docker/run/docker.sock`
  (`$HOME/.docker/run/docker.sock`). Clients that do not find Docker "may
  have to explicitly set the `DOCKER_HOST` environment variable" to it.
- `/var/run/docker.sock` is only a link to it, made when "the default
  Docker socket" option is on in Settings → Advanced. `/var/run` is a tmpfs,
  so the link is made again at each start. When the option is off, only
  the per-user socket exists.

**Default Podman machine on macOS, Podman 5.** From Podman's source
(`main`):

- `pkg/machine/env/dir_darwin.go`: `getRuntimeDir()` returns `$TMPDIR`, or
  `/tmp` when `TMPDIR` is not set.
- `pkg/machine/env/dir.go`: `GetMachineDirs` appends `podman` to it.
- `pkg/machine/vmconfigs/sockets.go`: `apiSocket(name, dir)` is
  `name + "-api.sock"` in that dir.
- So the default machine's API socket is
  `$TMPDIR/podman/podman-machine-default-api.sock`
  (`$TMPDIR` is a per-user `/var/folders/…/T/` on macOS). `podman machine
  inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}'` prints it.

**Default Podman machine on macOS, Podman 4.** From Podman `v4.9.5`:

- `pkg/machine/qemu/machine.go`: `forwardSocketPath` is
  `GetDataDir(QemuVirt)/podman.sock`; `userGlobalSocketLink` is
  `podman.sock` in the parent of that dir.
- `pkg/machine/config.go`: `GetDataDir` is
  `$XDG_DATA_HOME or ~/.local/share` + `containers/podman/machine/<vm type>`
  (`qemu` by default).
- So: `~/.local/share/containers/podman/machine/qemu/podman.sock` and the
  link `~/.local/share/containers/podman/machine/podman.sock`.
- Both versions can also take over `/var/run/docker.sock` (Podman's
  "claim the Docker socket" / `podman-mac-helper`); that is the same engine
  reached through a link, so discovery follows links and tries it once.

**`/version` from Podman vs Docker.** Podman's compat handler
(`pkg/api/handlers/compat/version.go`) builds the components list as
`"Podman Engine"`, then `"Conmon"`, then the OCI runtime; the top-level
`Version` is the Podman Engine version and `Platform.Name` is
`<os>/<arch>/<distro>-<version>`. Docker's answer has components `Engine`,
`containerd`, `runc`, `docker-init` and `Platform.Name` "Docker Engine -
Community" (Docker Desktop reports its own name there). Real answers were
captured on this machine (Fedora 43) with
`curl --unix-socket <sock> http://d/version`:

- Docker 29.9.1 from `/var/run/docker.sock` (`ApiVersion` 1.56,
  `MinAPIVersion` 1.40) →
  `crates/docker/tests/fixtures/docker-version.json`.
- Podman 5.8.8 from a temporary `podman system service` on a scratch socket
  (the user's `podman.socket` was left as it was) (`ApiVersion` 1.44,
  `MinAPIVersion` 1.24) → `crates/docker/tests/fixtures/podman-version.json`.

**API version check (found while testing).** Docker refuses any request whose
URL version is newer than it speaks or older than its minimum, also
`/version` and `/_ping` (`curl …/v1.99/version` → "client version 1.99 is
too new. Maximum supported API version is 1.56"; `/v1.30/version` → "too
old. Minimum supported API version is 1.40"). bollard 0.21.1 puts `v1.53`
in every URL by default, so on any Docker older than 29.x every call would
fail. Podman accepts any version. Discovery therefore asks `/version` at
1.40.

## Answer

Done in `crates/docker` (package `otty-docker`), added to the workspace
members. The app does not depend on it yet (ticket 09).

**Public API** (`otty_docker::…`):

- `async fn discover() -> Discovery`: builds the candidate list from the
  process environment, follows links so the same socket is tried once,
  tries every candidate at the same time (connect + `/version`, 2 s limit
  each, no retries), looks for `docker` and `podman` on `PATH`, and
  diagnoses. It never fails. Needs a tokio runtime (iced runs on tokio).
- `Discovery`: `probes() -> &[Probe]` (every place tried, in order, with
  its outcome: the "looked for" line), `cli() -> CliPresence`,
  `diagnosis() -> &Diagnosis`.
- `Probe`: `candidate() -> &SocketCandidate`, `outcome() -> &ProbeOutcome`.
- `ProbeOutcome`: `Reached(EngineInfo)`, `Missing` (no socket file, or OTTY
  cannot see it), `PermissionDenied` (the socket is there, connect gives
  `EACCES`), `NotAnswering(String)` (refused, timed out, or an engine error;
  the text says why).
- `SocketCandidate`: `address() -> &EngineAddress`,
  `source() -> CandidateSource` (`DockerHost`, `Docker`, `Podman`).
- `EngineAddress`: `Unix(PathBuf)` or `Tcp(String)`; `Display` prints the
  path, or `tcp://host:port`.
- `EngineInfo`: `kind() -> EngineKind` (`Docker`, `Podman`),
  `version() -> Option<&str>`, `api_version() -> Option<&str>`,
  `address() -> &EngineAddress`.
- `CliPresence`: `docker()`, `podman()`.

**Candidate order** (`socket_candidates`, crate-private and pure, tested on
plain inputs; the same address is listed once, at its first place):

1. `$DOCKER_HOST` if it is `unix://<path>`, `tcp://host:port` or
   `http://host:port`. Other kinds (`ssh://`, `npipe://`, empty) are skipped.
2. `/var/run/docker.sock`; on macOS then `~/.docker/run/docker.sock`.
3. `$XDG_RUNTIME_DIR/podman/podman.sock` (if set).
4. Linux: `/run/user/$UID/podman/podman.sock`, `/run/podman/podman.sock`.
   macOS: `$TMPDIR/podman/podman-machine-default-api.sock` (Podman 5;
   `/tmp` without `TMPDIR`), `~/.local/share/containers/podman/machine/podman.sock`
   and `~/.local/share/containers/podman/machine/qemu/podman.sock` (Podman 4).

At probe time candidates whose socket resolves (through links) to an already
listed one are dropped (`/var/run` → `/run` on Linux, Podman's
`/var/run/docker.sock` link on macOS).

**Kind detection** (`EngineInfo::from_version`, pure): a `Podman Engine`
component means Podman, anything else Docker. Version: top-level `Version`,
else the `Podman Engine` / `Engine` component's version. Blank values count
as missing.

**`Diagnosis`** (`diagnose`, pure, from per-candidate outcomes and
`CliPresence`; first match wins):

1. `Engines(Vec<EngineInfo>)`: at least one engine answered, in search order.
2. `NoPermission(SocketCandidate)`: some socket gave permission denied.
3. `DockerNotRunning`: `docker` on `PATH`, or a Docker socket file that does
   not answer.
4. `PodmanApiOff`: `podman` on `PATH`, or a Podman socket file that does not
   answer.
5. `NotInstalled`.

**Fixtures:** real `/version` JSON of Docker 29.9.1 and Podman 5.8.8 in
`crates/docker/tests/fixtures/` (see Comments). Probe tests run a small fake
engine on a unix socket and on tcp (std listeners in a thread) for: Docker
reached, Podman reached, tcp reached, missing file, stale socket (refused),
permission denied (`chmod 000`; skipped when running as root), timeout, and
an engine error answer. `tests/live_engine.rs` is the `#[ignore]` test
against a live engine; it passed here against Docker, and against Docker plus
Podman with `DOCKER_HOST` set to a Podman socket.

**Checks:** `cargo +nightly fmt`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo test --workspace --all-features` and
`cargo deny check` pass (deny: only the three `yanked` warnings that were
there before; bollard's tree is Apache-2.0/MIT, no advisory). Line coverage
from `cargo llvm-cov --workspace --all-features`: 79.57% on the merge base,
79.84% after. `discovery.rs` (the `discover()` wiring) is covered only by
the ignored live test.

**Deviations and choices:**

- No i18n strings: the crate has no user-facing text (the cards' texts are
  ticket 10). `NotAnswering` carries the English OS or engine message for
  logs and details.
- "Docker not running" and "Podman API off" also fire without the CLI when a
  socket file of that engine is there but does not answer (a stopped engine
  leaves its socket file). Decision 9 names only the CLI on `PATH`.
- Both CLIs on `PATH` and nothing answers → `DockerNotRunning` (one card).
- A path OTTY cannot even look at (for example `/run/podman/podman.sock`
  inside the root-only `/run/podman`) is `Missing`, not `NoPermission`: OTTY
  cannot tell that a socket is there, and the docker-group fix would be wrong
  for it.
- `/version` is asked at API 1.40 (see Comments); `EngineInfo::api_version()`
  gives what later calls should use.
- The Podman 4 data dir honours `$XDG_DATA_HOME` in Podman; discovery uses
  `~/.local/share` only.
- `socket_candidates` and its input type are crate-private; `discover()` is
  the only entry point the app needs.
- No `README.md` for the crate.

**For the next tickets:**

- 08: add a connect function that uses
  `min(EngineInfo::api_version(), bollard::API_DEFAULT_VERSION)`; with
  bollard's default 1.53 every call fails on Docker before 29.x. The private
  `probe::connect` shows the bollard calls.
- 09: add `otty-docker` to `app/Cargo.toml`; run `discover()` through
  `Task::perform`.
- 10: `Discovery::probes()` and `cli()` give the "looked for" line;
  `NoPermission` carries the denied socket. A macOS app started from Finder
  gets a short `PATH` (`/usr/bin:/bin:/usr/sbin:/sbin`), so `docker` in
  `/usr/local/bin` or `/opt/homebrew/bin` is not found and the diagnosis can
  be `NotInstalled` even with Docker installed but stopped; the app has no
  login-shell `PATH` lookup today. On Fedora, `podman-docker` puts a
  `docker` wrapper for Podman on `PATH`, which gives `DockerNotRunning` for a
  Podman user.
- Not in Decision 8, owner decision: Colima (`~/.colima/default/docker.sock`),
  OrbStack (`~/.orbstack/run/docker.sock`), Docker Desktop on Linux
  (`~/.docker/desktop/docker.sock`).
