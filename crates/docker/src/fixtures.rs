//! Values mapped from the engine answers captured in `tests/fixtures`
//! (Docker 29.9.1 and Podman 5.8.8), for the tests of crates that use this
//! one. The plain types have no public constructors, so tests outside this
//! crate get real values from here. Only built with the `fixtures` feature.
//!
//! # Example
//!
//! ```
//! use otty_docker::{EngineKind, fixtures};
//!
//! let engine = fixtures::engine(EngineKind::Docker);
//! let resources = fixtures::resources(EngineKind::Docker);
//!
//! assert_eq!(engine.version(), Some("29.9.1"));
//! assert!(!resources.containers().is_empty());
//! ```

use std::path::{Path, PathBuf};

use bollard::models::{
    ContainerSummary, CreateImageInfo, ImageInspect, ImageSummary, Network,
    SystemVersion, Volume as EngineVolume, VolumeListResponse,
};

use crate::address::EngineAddress;
use crate::candidates::{CandidateSource, SocketCandidate};
use crate::cli::CliPresence;
use crate::connection::Resources;
use crate::container::Container;
use crate::discovery::Discovery;
use crate::engine::{EngineInfo, EngineKind};
use crate::image::Image;
use crate::image_defaults::ImageDefaults;
use crate::logs::{ContainerRun, LogLine, LogPosition};
use crate::probe::{Probe, ProbeOutcome};
use crate::pull::PullProgress;
use crate::volume::Volume;

/// The fixtures are part of the crate, so one that does not parse is a
/// build defect.
const BROKEN_FIXTURE: &str = "captured fixture should parse";

/// Where the fixture Docker engine listens.
pub const DOCKER_SOCKET: &str = "/var/run/docker.sock";
/// Where the fixture Podman engine listens.
pub const PODMAN_SOCKET: &str = "/run/user/1000/podman/podman.sock";
/// The home folder a fixture search finds, whatever this computer has.
///
/// ```
/// use std::path::Path;
///
/// use otty_docker::{EngineKind, fixtures};
///
/// let discovery = fixtures::discovery(&[EngineKind::Docker]);
///
/// assert_eq!(
///     discovery.home_folder(),
///     Some(Path::new(fixtures::HOME_FOLDER))
/// );
/// ```
pub const HOME_FOLDER: &str = "/home/ann";

/// The engine that sent the captured `/version` answer of `kind`.
pub fn engine(kind: EngineKind) -> EngineInfo {
    let (answer, socket) = match kind {
        EngineKind::Docker => (
            include_str!("../tests/fixtures/docker-version.json"),
            DOCKER_SOCKET,
        ),
        EngineKind::Podman => (
            include_str!("../tests/fixtures/podman-version.json"),
            PODMAN_SOCKET,
        ),
    };

    let version: SystemVersion =
        serde_json::from_str(answer).expect(BROKEN_FIXTURE);

    EngineInfo::from_version(
        &version,
        EngineAddress::Unix(PathBuf::from(socket)),
    )
}

/// The Podman fixture engine at the socket `path`, for a test that answers
/// on that socket itself.
pub fn engine_at(path: &Path) -> EngineInfo {
    let version: SystemVersion = serde_json::from_str(include_str!(
        "../tests/fixtures/podman-version.json"
    ))
    .expect(BROKEN_FIXTURE);

    EngineInfo::from_version(&version, EngineAddress::Unix(path.to_path_buf()))
}

/// A search that reached the fixture engines of `kinds`, in that order.
/// With no kinds nothing answered and no command line tool is on `PATH`,
/// so the diagnosis is [`crate::Diagnosis::NotInstalled`].
pub fn discovery(kinds: &[EngineKind]) -> Discovery {
    discovery_with_cli(kinds, None, None)
}

/// Like [`discovery`], on a computer with SELinux enabled.
pub fn discovery_with_selinux(kinds: &[EngineKind]) -> Discovery {
    build_discovery(kinds, None, None, true, Some(HOME_FOLDER))
}

/// Like [`discovery`], on a computer that does not say where the home folder
/// is.
///
/// ```
/// use otty_docker::{EngineKind, fixtures};
///
/// let discovery = fixtures::discovery_without_home(&[EngineKind::Docker]);
///
/// assert_eq!(discovery.home_folder(), None);
/// ```
pub fn discovery_without_home(kinds: &[EngineKind]) -> Discovery {
    build_discovery(kinds, None, None, false, None)
}

/// Like [`discovery`], with the `docker` and `podman` commands found at
/// these paths (`None`: not installed).
pub fn discovery_with_cli(
    kinds: &[EngineKind],
    docker_path: Option<&str>,
    podman_path: Option<&str>,
) -> Discovery {
    build_discovery(kinds, docker_path, podman_path, false, Some(HOME_FOLDER))
}

/// A search where the only socket, at `socket`, refused this user.
///
/// ```
/// use otty_docker::{CandidateSource, Diagnosis, fixtures};
///
/// let discovery = fixtures::denied_discovery(
///     fixtures::DOCKER_SOCKET,
///     CandidateSource::Docker,
/// );
///
/// assert!(matches!(discovery.diagnosis(), Diagnosis::NoPermission(_)));
/// ```
pub fn denied_discovery(socket: &str, source: CandidateSource) -> Discovery {
    denied_discovery_with_cli(socket, source, None, None)
}

/// Like [`denied_discovery`], with the `docker` and `podman` commands found at
/// these paths (`None`: not installed).
///
/// ```
/// use otty_docker::{CandidateSource, Diagnosis, EngineKind, fixtures};
///
/// let discovery = fixtures::denied_discovery_with_cli(
///     fixtures::PODMAN_SOCKET,
///     CandidateSource::Podman,
///     Some("/usr/bin/docker"),
///     None,
/// );
///
/// assert!(matches!(
///     discovery.diagnosis(),
///     Diagnosis::NoPermissionAndOtherOff {
///         denied_engine: EngineKind::Podman,
///         ..
///     }
/// ));
/// ```
pub fn denied_discovery_with_cli(
    socket: &str,
    source: CandidateSource,
    docker_path: Option<&str>,
    podman_path: Option<&str>,
) -> Discovery {
    let address = EngineAddress::Unix(PathBuf::from(socket));
    let candidate = SocketCandidate::new(address, source);
    let probes = vec![Probe::new(candidate, ProbeOutcome::PermissionDenied)];
    let cli = CliPresence::new(docker_path.is_some(), podman_path.is_some())
        .with_paths(
            docker_path.map(PathBuf::from),
            podman_path.map(PathBuf::from),
        );

    Discovery::new(probes, cli, false, Some(PathBuf::from(HOME_FOLDER)))
}

/// The four captured lists of `kind`, mapped as [`crate::Engine::resources`]
/// maps them.
pub fn resources(kind: EngineKind) -> Resources {
    load_resources(kind, true)
}

/// The captured images, volumes and networks of `kind` as if no container
/// existed: nothing uses them, so every one can be removed.
pub fn idle_resources(kind: EngineKind) -> Resources {
    load_resources(kind, false)
}

/// A container named `web` that lists `ports`, each as host address, host
/// port, container port and protocol, as the engine would send them. A
/// port without a host port is only exposed.
pub fn container_with_ports(
    ports: &[(Option<&str>, Option<u16>, u16, &str)],
) -> Container {
    container_named_with_ports("web", ports)
}

/// A container named `name` that lists `ports`, as
/// [`container_with_ports`] describes them.
///
/// ```
/// use otty_docker::fixtures;
///
/// let container = fixtures::container_named_with_ports(
///     "db",
///     &[(Some("0.0.0.0"), Some(5432), 5432, "tcp")],
/// );
///
/// assert_eq!(container.name(), "db");
/// assert_eq!(container.ports().len(), 1);
/// ```
pub fn container_named_with_ports(
    name: &str,
    ports: &[(Option<&str>, Option<u16>, u16, &str)],
) -> Container {
    let ports: Vec<serde_json::Value> = ports
        .iter()
        .map(|(ip, public, private, protocol)| {
            serde_json::json!({
                "IP": ip,
                "PublicPort": public,
                "PrivatePort": private,
                "Type": protocol,
            })
        })
        .collect();
    let summary: ContainerSummary = serde_json::from_value(serde_json::json!({
        "Id": format!("{name}-id"),
        "Names": [format!("/{name}")],
        "Ports": ports,
    }))
    .expect("the container summary should parse");

    Container::from_summary(&summary)
}

/// A container named `name` in `state` (for example `exited`), run from the
/// image `image` whose id is `image_id`, as the engine would send it.
///
/// ```
/// use otty_docker::fixtures;
///
/// let container =
///     fixtures::container_of_image("web", "exited", "app:1", "sha256:one");
///
/// assert_eq!(container.name(), "web");
/// assert_eq!(container.image(), "app:1");
/// assert_eq!(container.image_id(), "sha256:one");
/// ```
pub fn container_of_image(
    name: &str,
    state: &str,
    image: &str,
    image_id: &str,
) -> Container {
    let summary: ContainerSummary = serde_json::from_value(serde_json::json!({
        "Id": format!("{name}-id"),
        "Names": [format!("/{name}")],
        "State": state,
        "Image": image,
        "ImageID": image_id,
    }))
    .expect("the container summary should parse");

    Container::from_summary(&summary)
}

/// A container named `name` in the Compose project `project`, as the engine
/// would send it.
///
/// ```
/// use otty_docker::fixtures;
///
/// let container = fixtures::container_in_project("web", "shop");
///
/// assert_eq!(container.compose_project(), Some("shop"));
/// ```
pub fn container_in_project(name: &str, project: &str) -> Container {
    let summary: ContainerSummary = serde_json::from_value(serde_json::json!({
        "Id": format!("{name}-id"),
        "Names": [format!("/{name}")],
        "Labels": { "com.docker.compose.project": project },
    }))
    .expect("the container summary should parse");

    Container::from_summary(&summary)
}

/// A volume named `name` with the driver `driver`, mounted by the containers
/// named `used_by`, as the engine would send it.
///
/// ```
/// use otty_docker::fixtures;
///
/// let volume = fixtures::volume_named("data", "local", &["web"]);
///
/// assert_eq!(volume.driver(), "local");
/// assert_eq!(volume.containers(), ["web"]);
/// ```
pub fn volume_named(name: &str, driver: &str, used_by: &[&str]) -> Volume {
    let volume: EngineVolume = serde_json::from_value(serde_json::json!({
        "Name": name,
        "Driver": driver,
        "Mountpoint": format!("/volumes/{name}"),
        "Labels": {},
        "Scope": "local",
        "Options": {},
    }))
    .expect("the volume should parse");
    let containers: Vec<Container> = used_by
        .iter()
        .map(|user| {
            let summary: ContainerSummary =
                serde_json::from_value(serde_json::json!({
                    "Id": format!("{user}-id"),
                    "Names": [format!("/{user}")],
                    "Mounts": [{ "Type": "volume", "Name": name }],
                }))
                .expect("the container summary should parse");

            Container::from_summary(&summary)
        })
        .collect();

    Volume::from_volume(&volume, &containers)
}

/// A network named `name` with the driver `driver`, the scope `scope` and the
/// address range `subnet`, as the engine would send it.
///
/// ```
/// use otty_docker::fixtures;
///
/// let network =
///     fixtures::network_named("back", "bridge", "local", Some("10.0.0.0/8"));
///
/// assert_eq!(network.scope(), "local");
/// assert_eq!(network.subnet(), Some("10.0.0.0/8"));
/// ```
pub fn network_named(
    name: &str,
    driver: &str,
    scope: &str,
    subnet: Option<&str>,
) -> crate::network::Network {
    let config: Vec<serde_json::Value> = subnet
        .iter()
        .map(|subnet| serde_json::json!({ "Subnet": subnet }))
        .collect();
    let network: Network = serde_json::from_value(serde_json::json!({
        "Id": format!("{name}-id"),
        "Name": name,
        "Driver": driver,
        "Scope": scope,
        "IPAM": { "Config": config },
    }))
    .expect("the network should parse");

    crate::network::Network::from_network(&network, &[])
}

/// The rows of one image with the id `id`, whose names are `repo_tags` and
/// `repo_digests`, as the engine would send them; `containers` are the
/// engine's containers.
///
/// ```
/// use otty_docker::fixtures;
///
/// let user =
///     fixtures::container_of_image("web", "exited", "app:1", "sha256:one");
/// let rows =
///     fixtures::image_rows("sha256:one", &["app:1", "app:2"], &[], &[user]);
///
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[1].reference(), Some("app:2"));
/// assert_eq!(rows[1].containers(), 1);
/// ```
pub fn image_rows(
    id: &str,
    repo_tags: &[&str],
    repo_digests: &[&str],
    containers: &[Container],
) -> Vec<Image> {
    let summary: ImageSummary = serde_json::from_value(serde_json::json!({
        "Id": id,
        "ParentId": "",
        "RepoTags": repo_tags,
        "RepoDigests": repo_digests,
        "Created": 1_793_557_837,
        "Size": 6_688_167,
        "SharedSize": -1,
        "Labels": null,
        "Containers": -1,
    }))
    .expect("the image summary should parse");

    Image::rows_from_summary(&summary, containers)
}

/// What `nginx:1.27` sets on its own: ports 80 and 443, the nginx command,
/// the entrypoint script, 192 MB. Read from `tests/fixtures/
/// docker-image-inspect.json`, written by hand in the shape Docker 29 sends.
pub fn image_defaults() -> ImageDefaults {
    let answer: ImageInspect = serde_json::from_str(include_str!(
        "../tests/fixtures/docker-image-inspect.json"
    ))
    .expect(BROKEN_FIXTURE);

    ImageDefaults::from_inspect(&answer)
}

/// The progress of a pull half way: three layers, one already on this
/// computer, 15 of 47 MB downloaded. Read from `tests/fixtures/
/// docker-pull.jsonl`, which is written by hand in the shape Docker 29
/// sends (no image was pulled to make it).
pub fn pull_progress() -> PullProgress {
    let mut progress = PullProgress::default();
    for line in include_str!("../tests/fixtures/docker-pull.jsonl").lines() {
        let info: CreateImageInfo =
            serde_json::from_str(line).expect(BROKEN_FIXTURE);
        progress.apply(&info);
    }

    progress
}

/// A run of a container started `started_at` seconds after the Unix epoch.
pub fn container_run(started_at: i64, is_running: bool) -> ContainerRun {
    ContainerRun::new(Some(started_at * 1_000_000_000), None, is_running)
}

/// The position of the `nth` line (from 1) that has the time `time`, in
/// seconds after the Unix epoch.
pub fn log_position(time: i64, nth: u32) -> LogPosition {
    LogPosition::new(time * 1_000_000_000, nth)
}

/// A log line with `text` that ends at `position`.
pub fn log_line(text: &str, position: Option<LogPosition>) -> LogLine {
    LogLine::new(text.to_string(), position)
}

/// A search that reached the fixture engines of `kinds`.
fn build_discovery(
    kinds: &[EngineKind],
    docker_path: Option<&str>,
    podman_path: Option<&str>,
    is_selinux_enabled: bool,
    home_folder: Option<&str>,
) -> Discovery {
    let probes = kinds
        .iter()
        .map(|kind| {
            let info = engine(*kind);
            let source = match kind {
                EngineKind::Docker => CandidateSource::Docker,
                EngineKind::Podman => CandidateSource::Podman,
            };
            let candidate =
                SocketCandidate::new(info.address().clone(), source);

            Probe::new(candidate, ProbeOutcome::Reached(info))
        })
        .collect();

    let cli = CliPresence::new(docker_path.is_some(), podman_path.is_some())
        .with_paths(
            docker_path.map(PathBuf::from),
            podman_path.map(PathBuf::from),
        );

    Discovery::new(
        probes,
        cli,
        is_selinux_enabled,
        home_folder.map(PathBuf::from),
    )
}

/// Map the captured lists of `kind`; without `with_containers` the
/// container list is left out.
fn load_resources(kind: EngineKind, with_containers: bool) -> Resources {
    let (containers, images, volumes, networks) = match kind {
        EngineKind::Docker => (
            include_str!("../tests/fixtures/docker-containers.json"),
            include_str!("../tests/fixtures/docker-images.json"),
            include_str!("../tests/fixtures/docker-volumes.json"),
            include_str!("../tests/fixtures/docker-networks.json"),
        ),
        EngineKind::Podman => (
            include_str!("../tests/fixtures/podman-containers.json"),
            include_str!("../tests/fixtures/podman-images.json"),
            include_str!("../tests/fixtures/podman-volumes.json"),
            include_str!("../tests/fixtures/podman-networks.json"),
        ),
    };

    let mut containers: Vec<ContainerSummary> =
        serde_json::from_str(containers).expect(BROKEN_FIXTURE);
    if !with_containers {
        containers.clear();
    }
    let images: Vec<ImageSummary> =
        serde_json::from_str(images).expect(BROKEN_FIXTURE);
    let volumes: VolumeListResponse =
        serde_json::from_str(volumes).expect(BROKEN_FIXTURE);
    let networks: Vec<Network> =
        serde_json::from_str(networks).expect(BROKEN_FIXTURE);

    Resources::from_answers(
        &containers,
        &images,
        volumes.volumes.as_deref().unwrap_or_default(),
        &networks,
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::diagnosis::Diagnosis;

    #[test]
    fn each_engine_kind_has_its_engine_and_lists() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let engine = engine(kind);
            let resources = resources(kind);

            assert_eq!(engine.kind(), kind);
            assert_eq!(resources.containers().len(), 6);
            assert!(!resources.images().is_empty());
            assert!(!resources.volumes().is_empty());
            assert!(!resources.networks().is_empty());
        }
    }

    #[test]
    fn idle_lists_have_no_containers_and_no_users() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            let idle = idle_resources(kind);

            assert!(idle.containers().is_empty());
            assert_eq!(idle.images().len(), resources(kind).images().len());
            assert!(idle.images().iter().all(|image| image.containers() == 0));
            assert!(
                idle.volumes()
                    .iter()
                    .all(|volume| volume.containers().is_empty())
            );
            assert!(
                idle.networks()
                    .iter()
                    .all(|network| network.containers().is_empty())
            );
        }
    }

    #[test]
    fn the_pull_fixture_stops_half_way() {
        let progress = pull_progress();

        assert_eq!(progress.layer_count(), 3);
        assert!(progress.downloaded() < progress.total());
    }

    #[test]
    fn a_discovery_lists_the_engines_it_reached_or_says_none() {
        let both = discovery(&[EngineKind::Docker, EngineKind::Podman]);
        let none = discovery(&[]);

        assert_eq!(
            both.diagnosis(),
            &Diagnosis::Engines(vec![
                engine(EngineKind::Docker),
                engine(EngineKind::Podman)
            ])
        );
        assert_eq!(none.diagnosis(), &Diagnosis::NotInstalled);
    }

    #[test]
    fn a_denied_discovery_is_diagnosed_from_its_source() {
        let docker = denied_discovery(DOCKER_SOCKET, CandidateSource::Docker);
        let podman = denied_discovery(PODMAN_SOCKET, CandidateSource::Podman);

        assert!(matches!(docker.diagnosis(), Diagnosis::NoPermission(_)));
        assert!(matches!(
            podman.diagnosis(),
            Diagnosis::PodmanNoPermission(_)
        ));
    }

    #[test]
    fn a_discovery_knows_the_fixture_home_unless_asked_not_to() {
        let home = Some(Path::new(HOME_FOLDER));
        let kinds = [EngineKind::Docker];

        assert_eq!(discovery(&kinds).home_folder(), home);
        assert_eq!(discovery_with_selinux(&kinds).home_folder(), home);
        assert_eq!(discovery_with_cli(&kinds, None, None).home_folder(), home);
        assert_eq!(
            denied_discovery(DOCKER_SOCKET, CandidateSource::Docker)
                .home_folder(),
            home
        );
        assert_eq!(discovery_without_home(&kinds).home_folder(), None);
        assert_eq!(
            discovery_without_home(&kinds).diagnosis(),
            discovery(&kinds).diagnosis()
        );
    }

    #[test]
    fn a_discovery_has_selinux_off_unless_asked_for() {
        let plain = discovery(&[EngineKind::Docker]);
        let with_cli = discovery_with_cli(
            &[EngineKind::Docker],
            Some("/bin/docker"),
            None,
        );
        let selinux = discovery_with_selinux(&[EngineKind::Docker]);

        assert!(!plain.is_selinux_enabled());
        assert!(!with_cli.is_selinux_enabled());
        assert!(selinux.is_selinux_enabled());
        assert_eq!(selinux.diagnosis(), plain.diagnosis());
    }
}
