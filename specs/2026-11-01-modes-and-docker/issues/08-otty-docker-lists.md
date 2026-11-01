# 08 — `otty-docker`: list and map containers, images, volumes, networks

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 07
Spec: [../spec.md](../spec.md)

## What

Plain OTTY types and list calls (spec Decisions 7, 10). Tests first.

- `Container` (id, name, state enum, status text, exit code, image, ports, created, compose project, mounts, networks, command), `Image` (id, tags, size, created, how many containers use it), `Volume` (name, driver, mountpoint, created, labels, containers that use it), `Network` (id, name, driver, scope, subnet, gateway, containers, created, built-in).
- Mapping from bollard models is a pure function per type, tested on fixture JSON captured from Docker and from Podman. Where Podman fills a field differently (for example image container counts), compute it from the container list.
- "Used by" for images and volumes comes from the container list, not from extra calls.
- No time or size formatting here; the app formats.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Mapping tests for every type on Docker and Podman fixtures, including dangling images, anonymous volumes, built-in networks, exited containers with and without an error code.

## Comments

### Research: fixtures, Podman differences, compose labels, API version (2026-11-01)

**How the fixtures were made.** Docker 29.9.1 (`/var/run/docker.sock`) and
Podman 5.8.8 (a temporary `podman system service` on a short socket in
`/tmp`; the user's `podman.socket` was not touched, and the service was
stopped afterwards) each got the same throwaway objects, all named
`otty-fx…`:

- the image `busybox:1.37`;
- a network `otty-fx-net` and a named volume `otty-fx-data`;
- containers `otty-fx-running` (port 18080→80, the volume, the network),
  `otty-fx-paused`, `otty-fx-exit0` (`true`), `otty-fx-exit3`
  (`sh -c 'exit 3'` with an anonymous volume `-v /scratch`) and
  `otty-fx-created` (never started);
- a compose project `otty-fx` with one service `web`, started with
  `docker compose` (for Podman: `docker compose` against the Podman socket,
  which is what `podman compose` runs on this machine). Docker's address
  pools were full, so the project uses `otty-fx-net` as an external network;
- a dangling image (`commit` to `otty-fx-img:1` twice).

The four lists were captured with `GET /containers/json?all=1`,
`/images/json`, `/volumes` and `/networks`, then **filtered to the fixture
objects plus the built-in networks**. The engines hold the owner's other
projects, which do not belong in the repository. The only hand edit is that
local paths were replaced (`/home/<user>` → `/home/user`, the compose
directory → `/home/user/otty-fx`) in `docker-containers.json`,
`podman-containers.json` and `podman-volumes.json`. Everything else is the
JSON the engines sent. Cases that these two engines do not produce are small
inline JSON in the tests: Docker before 29 (`<none>:<none>` tag,
`Containers: -1`), an unknown state, bind and tmpfs mounts, and an exposed
port without a host port. All throwaway objects were removed from both
engines afterwards.

**Podman differences found** (Podman 5.8.8 vs Docker 29.9.1):

- Image names: `docker.io/library/busybox:1.37` and `localhost/otty-fx-img:1`
  (Docker: `busybox:1.37`, `otty-fx-img:1`). The container `Image` field is
  the same.
- Image list: Podman adds `Dangling`, `Names`, `History` and `Digest`;
  `Labels` is `null` when empty. Both engines fill `Containers` here (Docker
  before 29 sends `-1`), but OTTY counts from the container list anyway.
- Dangling images: both send `RepoTags: []`; older Docker sends
  `["<none>:<none>"]`.
- Anonymous volumes: Docker labels them `com.docker.volume.anonymous`.
  Podman's compat list has no marker (only its own API has `Anonymous`).
  Both name them with 64 hex digits, so OTTY uses the label or the name.
- Networks: Podman shows its default network `podman` as **`bridge`** in the
  compat API, and sends the time of the request as its `Created`. Podman
  fills `Containers` in the network list; Docker leaves it out. Podman has
  no `host` or `none` entries.
- Rootless containers are on `pasta`, which is not a listed network, so
  Podman's `bridge` shows no containers. A paused Podman container has no
  networks at all in the list.
- Command text: Podman drops the quotes (`sh -c exit 3`; Docker:
  `sh -c 'exit 3'`).
- Ports: Docker lists a published port once for `0.0.0.0` and once for `::`;
  Podman lists it once.
- States: Podman's compat list maps its own states to Docker's (`stopped` →
  `exited`, `stopping` → `running` with the status text "Stopping"). See
  `pkg/api/handlers/compat/containers.go`, `LibpodToContainer`.

**Compose project label.** All three tools set `com.docker.compose.project`:

- Docker Compose sets `com.docker.compose.project`, `…service`,
  `…container-number`, `…project.working_dir`, `…project.config_files` and
  more (seen in the fixtures).
- `podman compose` is only a wrapper. It runs an external provider, here
  `/usr/libexec/docker/cli-plugins/docker-compose` (Docker Compose 5.6.0),
  so the labels are Docker Compose's (seen in the Podman fixture).
- `podman-compose` is not installed here. Its source (`podman_compose.py` on
  `containers/podman-compose` `main`) sets on every container both
  `io.podman.compose.project=<name>` and `com.docker.compose.project=<name>`,
  plus `io.podman.compose.version`,
  `PODMAN_SYSTEMD_UNIT=podman-compose@<name>.service`,
  `com.docker.compose.project.working_dir`,
  `com.docker.compose.project.config_files`, `io.podman.compose.config-hash`,
  `com.docker.compose.container-number`, `io.podman.compose.service` and
  `com.docker.compose.service`. It finds its own containers by
  `io.podman.compose.project`.

**The API version in the URL (correction to 07).** bollard 0.21.1 does
**not** put its version in request URLs. `Uri::parse` builds `…/v1.53` and
then calls `url.join(path)` with an absolute path such as
`/containers/json`, which replaces the whole path. The fake engine in the
tests receives `GET /containers/json?all=true&size=false`. Every call goes
through the same code. An engine answers an unversioned request at its
newest API version. For all four lists, the Docker 29.9.1 answers at
`/v1.53/…` and unversioned have the same keys (checked). The live test also
lists 122 containers, 125 images, 92 volumes and 34 networks of the owner's
Docker without an error. So the problem 07 described ("every call fails on
Docker before 29") does not happen with bollard 0.21.1. OTTY still gives the
client `min(EngineInfo::api_version(), 1.53)`, so old Docker keeps working
if a later 0.21.x sends the version again.

**Risk.** bollard reads the container state into a closed list of values.
If an engine sends a state outside
`created/running/paused/restarting/removing/exited/dead/stopping/""`
(Podman could send `unknown` for a broken container), the whole container
list fails to parse and the call returns `EngineError::Request`. Neither
engine here sent such a state.

## Answer

Done in `crates/docker` (package `otty-docker`). No new dependencies.

**Public API** (`otty_docker::…`, all re-exported from the crate root):

- `Engine`: `Engine::connect(&EngineInfo) -> Result<Engine, EngineError>`
  does no I/O. The client uses `min(api_version, 1.53)`, or 1.40 when the
  engine sent no readable version, with a 30 s limit per request. `info()`
  gives the `EngineInfo`. The calls make one request each, with no retries:
  - `resources() -> Result<Resources, EngineError>` loads all four lists at
    the same time, with "used by" taken from that container list. Use it
    when entering Docker mode and for Refresh.
  - `containers()`, and `images(&[Container])`, `volumes(&[Container])` and
    `networks(&[Container])`. The last three take the container list the
    caller already has, for reloading one list after an event.
  - `Clone` is cheap (it shares the connection pool). Calls need a tokio
    runtime.
- `Resources`: `containers()`, `images()`, `volumes()`, `networks()`.
- `EngineError`: `PathNotUtf8(PathBuf)`, or `Request(bollard::errors::Error)`
  (cannot connect, engine error with its message, or an answer OTTY cannot
  read).
- `Container`: `id`, `name` (without `/`), `state() -> ContainerState`,
  `status` (the engine's text), `exit_code() -> Option<i64>` (read from
  `Exited (N) …`, for exited containers only), `image`, `image_id`,
  `ports() -> &[PortMapping]`, `created() -> Option<i64>`,
  `compose_project()` (from `com.docker.compose.project`; blank means none),
  `mounts() -> &[Mount]`, `networks() -> &[ContainerNetwork]` (sorted by
  name), `command`.
- `ContainerState`: `Created`, `Running`, `Paused`, `Restarting`, `Removing`,
  `Exited`, `Dead`, `Stopping`, `Unknown`.
- `PortMapping`: `host_ip`, `host_port` (`None` means only exposed),
  `container_port`, `protocol` (`tcp` when not sent). Kept as the engine
  lists them (Docker: one per address family).
- `Mount`: `kind() -> MountKind` (`Volume`, `Bind`, `Other`), `name` (the
  volume name), `source`, `destination`, `is_read_only`.
- `ContainerNetwork`: `name`, `id` (`None` when Docker sent none, for a
  container that never ran), `ip_address`.
- `Image`: `id`, `tags` (without `<none>` tags), `size: u64`,
  `created: i64`, `containers: usize` (counted from the container list by
  image id), `is_dangling` (no tags).
- `Volume`: `name`, `driver`, `mountpoint`, `created: Option<i64>`, `labels`
  (a sorted map), `containers() -> &[String]` (sorted names of the
  containers that mount it), `is_anonymous` (Docker's label, or a name of 64
  hex digits).
- `Network`: `id`, `name`, `driver`, `scope`, `subnet` and `gateway` (from
  the first address range), `containers() -> &[String]` (sorted names;
  matched by network id, or by name when the container entry has no id),
  `created`, `is_built_in` (`bridge`, `host`, `none`).

All times are Unix seconds. Volumes and networks send RFC 3339 text, which
a small std-only reader (`timestamp.rs`) turns into seconds; Go's zero time
and anything before 1970 become `None`. Times and sizes are not formatted.

**Mapping** is a pure, crate-private function per type, each in its own
file: `Container::from_summary`, `Image::from_summary`, `Volume::from_volume`
and `Network::from_network`. The private `probe::connect` became the shared
`connection::open_client`, which discovery (at 1.40) and `Engine` both use.

**Fixtures** (`crates/docker/tests/fixtures/`): `docker-` and `podman-`
`containers.json`, `images.json`, `volumes.json` and `networks.json`.
Comments say how they were made and which were edited.

**Tests:** 52 new unit tests.

- Mapping on both fixtures: running, paused, created, exited 0 and exited 3
  containers, with ports, mounts, networks and compose project.
- Images with container counts, and dangling images, including the old
  `<none>:<none>` form and `Containers: -1`.
- Named and anonymous volumes with their users, on both engines.
- Built-in networks (`bridge`, `host` and `none` on Docker; `bridge` on
  Podman), and user networks with their containers. These match Podman's
  own `Containers` map. A Docker container that never ran is matched by
  name.
- RFC 3339 reading and the choice of API version.
- `Engine` against a fake engine on a unix socket that serves the fixtures:
  `resources()` (the four requests, with `all=true`), the four single calls
  on Podman, an engine error with its message, a non-UTF-8 socket path, and
  tcp.

The `#[ignore]` live test now also connects to every reached engine and
loads all four lists. It passed against the owner's Docker, and against
Podman with `DOCKER_HOST` set to the temporary Podman socket.

**Checks:** `cargo +nightly fmt`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, `cargo test --workspace --all-features` and
`cargo deny check` pass. Deny shows only the three `yanked` warnings that
were there before. Line coverage (`cargo llvm-cov --workspace
--all-features`) is 80.70% on the base (`9798b42`) and 81.21% after.

**Deviations and choices:**

- No i18n strings: the crate has no user-facing text.
- 07 said bollard puts `v1.53` in every URL; it does not (see Comments). The
  version cap is kept as a guard, and it is what `Docker::client_version()`
  reports.
- `resources()` was added beside the four single calls. Loading everything
  is the main call (ticket 09), it runs the four requests at the same time,
  and it keeps "used by" consistent with one container list.
- `Volume::is_anonymous` is extra: Docker's volume prune removes only
  anonymous volumes unless asked for all, which ticket 12 needs.
- Podman's default network counts as built-in because the API calls it
  `bridge`. The name `podman` is not in the built-in list, because the compat
  API never shows it.
- Ports are not merged across `0.0.0.0` and `::`; the app decides how to
  show them.

**For the next tickets:**

- 09: call `Engine::connect(info)` for each reached engine, then clone the
  engine into `Task::perform` for `resources()`. Tree counts come from
  `containers().len()` and the running count from `state()`; compose
  projects come from `compose_project()`. For the "Ports" column, consider
  merging Docker's `0.0.0.0`/`::` pairs. For "ID (12 chars)", `id()` starts
  with `sha256:`, so cut that first. Podman image tags carry the registry
  (`docker.io/library/…`).
- 11/12: disable Remove when `Volume::containers()` or
  `Network::containers()` is not empty, or when `Network::is_built_in()`.
  Offer "Force" for images with `Image::containers() > 0`. For volume prune,
  Docker removes only anonymous volumes unless `all=true` is passed in the
  filters.
- 13: Overview from the `Container` fields; Inspect needs new inspect calls
  on `Engine`. For following logs, check that the 30 s bollard request limit
  does not cut a long `follow` stream. If it does, give `Engine` a second
  client without a limit for streams.
- 14: the Run form's network select comes from `Network::name()`; Podman's
  default network shows as `bridge`.
- 16: after a container event, reload images, volumes or networks with the
  newest container list (`images/volumes/networks(&containers)`) to keep
  "used by" right. A container in an unknown state makes the whole container
  list fail (see Risk in Comments); show the `EngineError` as a Notice.
