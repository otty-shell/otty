# 21 — Discovery: more sockets, CLI install folders, `podman-docker` wrapper

Status: resolved
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

## Comments

### Research: Colima's socket paths (2026-11-01)

From Colima's source (`abiosoft/colima`, `main`) and its FAQ:

- `docs/FAQ.md`, "Docker socket location": v0.3.4 and older used
  `$HOME/.colima/docker.sock`; v0.4.0 and newer use
  `$HOME/.colima/default/docker.sock` (one folder per profile).
- `environment/container/docker/context.go`: `HostSocketFile()` is
  `<profile config dir>/docker.sock`; `LegacyDefaultHostSocketFile()` is
  `docker.sock` in the parent folder, that is `~/.colima/docker.sock`.
- `environment/vm/lima/yaml.go`: for the `default` profile Colima still
  forwards the engine socket to the legacy path too, "for backward
  compatibility, will be removed in future releases". Both host sockets are
  separate forwards of the same guest `/var/run/docker.sock`, not links.
- So the old path is still created today, but only next to the new one and
  for the same engine. Listing it would show the same Colima engine twice
  (discovery removes duplicates by resolved path, and these are two
  sockets). Discovery lists only `~/.colima/default/docker.sock`.
- `config/files.go`: Colima's base folder is `$COLIMA_HOME` if it exists,
  else `~/.colima` if it exists, else `$XDG_CONFIG_HOME/colima` or
  `~/.config/colima` when that exists; on macOS a fresh install uses
  `~/.colima`, **on Linux a fresh install uses `~/.config/colima`**. A new
  Colima user on Linux therefore has `~/.config/colima/default/docker.sock`,
  which Decision 8 does not list (see Answer, follow-ups).

### Research: what `docker --version` prints with `podman-docker` (2026-11-01)

`podman-docker` is not installed on this machine (Fedora 43, Podman 5.8.8);
nothing was installed. From Podman's source (`containers/podman`, `main`)
and packaging:

- `docker/docker.in` is the wrapper. `make install.docker` fills in
  `BINDIR`/`ETCDIR` and installs it as `$BINDIR/docker`:

  ```sh
  #!/bin/sh
  [ -e ${ETCDIR}/containers/nodocker ] || [ -e "${XDG_CONFIG_HOME-$HOME/.config}/containers/nodocker" ] || \
  echo "Emulate Docker CLI using podman. Create ${ETCDIR}/containers/nodocker to quiet msg." >&2
  exec ${BINDIR}/podman "$@"
  ```

  Fedora's `rpm/podman.spec` (`%files docker`) and Arch's `podman-docker`
  package (`make install.docker-full`) ship this script.
- `cmd/podman/root.go`: the root command's `Use` is the base name of
  `argv[0]`, and podman sets no version template, so cobra's default
  `{{.DisplayName}} version {{.Version}}` is printed.
- The wrapper `exec`s `/usr/bin/podman`, so `argv[0]` is `/usr/bin/podman`
  and stdout is `podman version 5.8.8`; stderr has the "Emulate Docker CLI
  using podman…" note unless a `nodocker` file exists. Checked here by
  running a copy of the filled-in wrapper against the installed Podman.
- A plain link `docker → podman` instead prints `docker version 5.8.8`
  (checked here), which does not name Podman. Discovery therefore also
  treats a `docker` command whose resolved file is named `podman` as Podman,
  without running it.
- Docker's own CLI prints `Docker version 29.9.1, build dfc520d`.
- `podman-docker` also installs `contrib/systemd/system/podman-docker.conf`
  as a system and a user tmpfiles rule,
  `L+ %t/docker.sock - - - - %t/podman/podman.sock`: `/run/docker.sock`
  links to `/run/podman/podman.sock` (and `$XDG_RUNTIME_DIR/docker.sock` to
  the rootless socket), plus `/etc/profile.d/podman-docker.sh`, which sets
  `DOCKER_HOST` to the Podman socket in login shells. Discovery follows the
  `/var/run/docker.sock` link, so a running rootful Podman is reached once
  and shows as Podman.

### Follow-up: Colima's real base folder and the "looked for" text (2026-11-01)

Done after ticket 16, in its own commit, tests first.

- `candidates.rs`: Colima's socket is now `<colima base>/default/docker.sock`,
  where the base folder is chosen as Colima's `configBaseDir`
  (`config/files.go`, `main`) does: `$COLIMA_HOME` if it exists; else
  `~/.colima` if it exists; else `$XDG_CONFIG_HOME/colima` when that
  variable is set; else `~/.config/colima` if it exists; else `~/.colima`
  on macOS and `~/.config/colima` on Linux. A fresh Colima on Linux is
  therefore found at `~/.config/colima/default/docker.sock`. The choice is
  the pure `colima_dir(&ColimaInputs, exists)`, tested with a fake
  `exists`; `DiscoveryEnv::from_process` calls it with `Path::exists` and
  keeps the result in `DiscoveryEnv::colima_dir`. The place in the order
  (after Docker Desktop on Linux, before OrbStack on macOS) and the
  deduplication are unchanged. Without `$HOME`, only an existing
  `$COLIMA_HOME` or a set `$XDG_CONFIG_HOME` gives a Colima socket.
- The no-engine card's line now says "…, and for docker and podman on
  PATH and in the usual install folders." (`tpl_docker_looked_for` in the
  ten locales; an i18n test checks the text of every locale). No new keys.
- Coverage (lines): 84.02% on `4d43660` → 84.07%.

## Answer

Done in `crates/docker`. The public API is unchanged: same types, same
functions, same signatures. `CliPresence::docker()` and `podman()` now mean
"installed" (found on `PATH` or in an install folder) instead of "on
`PATH`", and a `docker` command that is really Podman counts under
`podman()`, not `docker()`.

**Sockets** (`candidates.rs`, pure, tested on plain inputs): after the
Podman sockets, with source `CandidateSource::Docker` (the `/version`
answer still decides the kind):

- Linux: `~/.docker/desktop/docker.sock` (Docker Desktop),
  `~/.colima/default/docker.sock` (Colima).
- macOS: `~/.colima/default/docker.sock` (Colima),
  `~/.orbstack/run/docker.sock` (OrbStack).
- Skipped without `$HOME`. The same address is still listed once at its
  first place (for example in `$DOCKER_HOST`), and a link such as
  `/var/run/docker.sock` → OrbStack's socket is still tried once.

**Commands** (`cli.rs`, std only):

- `command_dirs(PATH, HOME)` (pure): the `PATH` folders, then
  `/usr/local/bin`, `/opt/homebrew/bin`, `~/.docker/bin`,
  `/Applications/Docker.app/Contents/Resources/bin`, `/opt/podman/bin`, on
  both platforms (a folder that does not exist simply finds nothing).
- `CliPresence::find(dirs, limit)`: finds the first executable `docker` and
  `podman`. Only when `docker` is found it checks whether it is Podman:
  first a link check (the resolved file is named `podman`), else it runs
  `docker --version` with a 1.5 s limit (`CLI_VERSION_TIMEOUT`); on time out
  the process is killed (`kill_on_drop`). It runs at the same time as the
  socket probes (2 s), so discovery takes no longer than before. A run that
  fails, times out or exits non-zero leaves `docker` counted as Docker.
- `version_names_podman` (pure): "podman" in the answer, any case.
- `CliPresence::from_lookup(docker_found, docker_is_podman, podman_found)`
  (pure): a `docker` that is Podman gives `docker() == false`,
  `podman() == true`. The diagnosis code is unchanged; for a
  `podman-docker` user with no engine answering it now gives
  `PodmanApiOff`.

**Tests:** candidates for each new socket on Linux and macOS, without
`$HOME`, in `$DOCKER_HOST` and through a link (deduplicated); CLI lookup
found on `PATH`, found only in an install folder (`~/.docker/bin` under a
temporary home), not found, order of the folders; `--version` answers of
Docker and of the `podman-docker` wrapper; a link to `podman`; a hanging
`docker` (killed after 200 ms); a failing `docker`; the diagnosis for a
`podman-docker` user ("Podman API off"), also without `podman` found. The
process tests run small shell scripts in a temporary folder (30 repeated
runs, no flakes). The ignored live test still passes against this machine's
Docker and Podman.

**Checks:** `cargo +nightly fmt`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo test --workspace --all-features`,
`cargo deny check` pass (deny: only the three `yanked` warnings that were
there before). Line coverage (`cargo llvm-cov --workspace --all-features`):
82.66% on the merge base, 82.78% after.

**Deviations and choices:**

- tokio's `process` feature is turned on in `crates/docker/Cargo.toml` for
  the time-limited `docker --version` run. It is not a new crate: the
  workspace already builds tokio with all features (`otty-ui-term`), and
  `Cargo.lock` did not change. std has no way to wait on a process with a
  time limit without blocking the async runtime.
- Besides the `--version` check, a `docker` link to the `podman` binary also
  counts as Podman (its answer `docker version 5.8.8` does not name Podman;
  see Comments).
- Only stdout of `docker --version` is read; the wrapper's stderr note is
  not needed.
- The old Colima path `~/.colima/docker.sock` is not listed (same engine as
  the new path; see Comments).
- No i18n strings: the crate has no user-facing text.

**Follow-ups:**

- Ticket 10: `Discovery::cli()` now reflects install folders, so the macOS
  Finder case from ticket 07 (Docker installed but stopped, `docker` only in
  `/usr/local/bin` or `/opt/homebrew/bin`) gives "Docker not running" instead
  of "not installed". The "looked for" line can say "`docker` and `podman`
  on `PATH` and in the usual install folders". A `podman-docker` user sees
  the Podman card.
- Ticket 10: with `podman-docker` and the rootful `podman.socket` on but
  root-only `/run/podman`, `/var/run/docker.sock` links into it and gives
  permission denied, so the diagnosis is `NoPermission` with the
  docker-group advice, which does not fit Podman. Rare; the card could say
  "Docker socket" only when the candidate is not a link to a Podman socket.
- Owner decision: a fresh Colima install on Linux keeps its socket in
  `~/.config/colima/default/docker.sock` (or `$XDG_CONFIG_HOME/colima/…`),
  which Decision 8 does not list. `$COLIMA_HOME` is not read either.
