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

use std::path::PathBuf;

use bollard::models::{
    ContainerSummary, CreateImageInfo, ImageInspect, ImageSummary, Network,
    SystemVersion, VolumeListResponse,
};

use crate::address::EngineAddress;
use crate::candidates::{CandidateSource, SocketCandidate};
use crate::cli::CliPresence;
use crate::connection::Resources;
use crate::discovery::Discovery;
use crate::engine::{EngineInfo, EngineKind};
use crate::image_defaults::ImageDefaults;
use crate::probe::{Probe, ProbeOutcome};
use crate::pull::PullProgress;

/// The fixtures are part of the crate, so one that does not parse is a
/// build defect.
const BROKEN_FIXTURE: &str = "captured fixture should parse";

/// Where the fixture Docker engine listens.
pub const DOCKER_SOCKET: &str = "/var/run/docker.sock";
/// Where the fixture Podman engine listens.
pub const PODMAN_SOCKET: &str = "/run/user/1000/podman/podman.sock";

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

/// A search that reached the fixture engines of `kinds`, in that order.
/// With no kinds nothing answered and no command line tool is on `PATH`,
/// so the diagnosis is [`crate::Diagnosis::NotInstalled`].
pub fn discovery(kinds: &[EngineKind]) -> Discovery {
    discovery_with_cli(kinds, None, None)
}

/// Like [`discovery`], with the `docker` and `podman` commands found at
/// these paths (`None`: not installed).
pub fn discovery_with_cli(
    kinds: &[EngineKind],
    docker_path: Option<&str>,
    podman_path: Option<&str>,
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

    Discovery::new(probes, cli)
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
}
